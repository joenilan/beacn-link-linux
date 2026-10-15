//! Read-only PipeWire catalog. Distinct type/FFI mode; cannot apply routes.
use crate::catalog::{self, Catalog};
use std::{
    ffi::{c_char, c_int, c_void, CString},
    fs::{self, DirBuilder, OpenOptions},
    io::Write,
    marker::PhantomData,
    os::unix::{
        fs::{DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt},
        io::IntoRawFd,
        net::UnixStream,
    },
    path::{Path, PathBuf},
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

unsafe extern "C" {
    fn getuid() -> u32;
    fn bcl_open_catalog(fd: c_int, config: *const c_char) -> *mut c_void;
    fn bcl_catalog(session: *mut c_void, buffer: *mut c_char, size: usize) -> c_int;
    fn bcl_catalog_activity(session: *mut c_void, client_serial: u64, pid: u32) -> c_int;
    fn bcl_close(session: *mut c_void);
    #[cfg(feature = "native-test-faults")]
    fn bcl_catalog_test_mutation_refused(session: *mut c_void) -> c_int;
    #[cfg(all(test, feature = "native-test-faults"))]
    fn bcl_catalog_test_activity_identity() -> c_int;
}
pub(crate) struct PrivateConfig {
    pub(crate) directory: PathBuf,
    stamp: (u64, u64),
    text: &'static str,
}
impl PrivateConfig {
    pub(crate) fn new() -> Result<Self, String> {
        Self::with_text(crate::native_pipewire::CLIENT_CONFIG)
    }
    pub(crate) fn with_text(text: &'static str) -> Result<Self, String> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let directory = PathBuf::from(format!(
            "/tmp/beacn-linx-catalog-{}-{nonce}",
            std::process::id()
        ));
        DirBuilder::new()
            .mode(0o700)
            .create(&directory)
            .map_err(|e| e.to_string())?;
        let m = fs::symlink_metadata(&directory).map_err(|e| e.to_string())?;
        let config = Self {
            directory,
            stamp: (m.dev(), m.ino()),
            text,
        };
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(config.directory.join("client.conf"))
            .map_err(|e| e.to_string())?;
        file.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        Ok(config)
    }
}
impl Drop for PrivateConfig {
    fn drop(&mut self) {
        let valid = fs::symlink_metadata(&self.directory).is_ok_and(|m| {
            m.is_dir()
                && !m.file_type().is_symlink()
                && (m.dev(), m.ino()) == self.stamp
                && m.uid() == unsafe { getuid() }
                && m.mode() & 0o777 == 0o700
        });
        if !valid {
            return;
        }
        let path = self.directory.join("client.conf");
        if fs::symlink_metadata(&path).is_ok_and(|m| {
            m.is_file()
                && !m.file_type().is_symlink()
                && m.uid() == unsafe { getuid() }
                && m.len() == self.text.len() as u64
        }) && fs::read(&path).is_ok_and(|b| b == self.text.as_bytes())
        {
            let _ = fs::remove_file(path);
        }
        let _ = fs::remove_dir(&self.directory);
    }
}
pub struct NativeCatalog {
    session: *mut c_void,
    _config: PrivateConfig,
    _single_thread: PhantomData<Rc<()>>,
}
/// Current activity of one exact client generation, including a zero-stream gap.
/// This is read-only continuity evidence, never permission to adopt a route.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    Running,
    Idle,
    Gone,
    Ambiguous,
}
impl NativeCatalog {
    pub fn desktop() -> Result<Self, String> {
        Self::connect(
            &PathBuf::from(format!("/run/user/{}", unsafe { getuid() })),
            "pipewire-0",
        )
    }
    /// Either the owned desktop runtime or an explicitly selected existing
    /// disposable core. Never a fallback or an arbitrary Unix socket.
    pub fn connect(runtime: &Path, name: &str) -> Result<Self, String> {
        if std::env::vars_os()
            .any(|(key, _)| key.to_str().is_some_and(|key| key.starts_with("PIPEWIRE_")))
        {
            return Err(
                "catalog refuses PipeWire environment overrides; use explicit connection arguments"
                    .into(),
            );
        }
        let uid = unsafe { getuid() };
        let desktop = runtime == PathBuf::from(format!("/run/user/{uid}")) && name == "pipewire-0";
        let private = runtime.parent() == Some(Path::new("/tmp"))
            && runtime
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.starts_with("beacn-pw-"))
            && name.starts_with("beacn-test-");
        let m = fs::symlink_metadata(runtime).map_err(|e| e.to_string())?;
        if !(desktop || private)
            || !m.is_dir()
            || m.file_type().is_symlink()
            || m.uid() != uid
            || m.mode() & 0o777 != 0o700
            || fs::canonicalize(runtime).map_err(|e| e.to_string())? != runtime
            || name.len() > 64
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err(
                "read-only catalog requires the owned user runtime or explicit private core".into(),
            );
        }
        let path = runtime.join(name);
        let before = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if !before.file_type().is_socket() || before.uid() != uid {
            return Err("catalog socket ownership refused".into());
        }
        let stream = UnixStream::connect(&path).map_err(|e| e.to_string())?;
        let after = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if (before.dev(), before.ino(), before.uid()) != (after.dev(), after.ino(), after.uid()) {
            return Err("catalog socket changed while connecting".into());
        }
        let config = PrivateConfig::new()?;
        let file = CString::new(
            config
                .directory
                .join("client.conf")
                .to_str()
                .ok_or("invalid catalog config path")?,
        )
        .map_err(|e| e.to_string())?;
        let session = unsafe { bcl_open_catalog(stream.into_raw_fd(), file.as_ptr()) };
        if session.is_null() {
            return Err("read-only catalog connection/core validation failed".into());
        }
        Ok(Self {
            session,
            _config: config,
            _single_thread: PhantomData,
        })
    }
    pub fn observe(&mut self, generation: u64) -> Result<Catalog, String> {
        let mut buffer = vec![0u8; catalog::MAX_CATALOG_BYTES + 1];
        if unsafe { bcl_catalog(self.session, buffer.as_mut_ptr().cast(), buffer.len()) } != 0 {
            return Err("read-only native catalog observation failed".into());
        }
        let len = buffer
            .iter()
            .position(|b| *b == 0)
            .ok_or("unterminated native catalog")?;
        catalog::parse(
            generation,
            std::str::from_utf8(&buffer[..len]).map_err(|_| "catalog transport was not UTF-8")?,
        )
    }
    pub fn activity(&mut self, id: &crate::planner::ApplicationId) -> Result<Activity, String> {
        let (client, pid) =
            id.0.strip_prefix("pw-client:")
                .and_then(|value| value.split_once(":pid:"))
                .ok_or("Invalid application continuity identity.")?;
        let client: u64 = client.parse().map_err(|_| "Invalid client generation.")?;
        let pid: u32 = pid.parse().map_err(|_| "Invalid application PID.")?;
        if client == 0 || pid == 0 {
            return Err("Invalid application continuity identity.".into());
        }
        match unsafe { bcl_catalog_activity(self.session, client, pid) } {
            0 => Ok(Activity::Idle),
            1 => Ok(Activity::Running),
            2 => Ok(Activity::Gone),
            3 => Ok(Activity::Ambiguous),
            _ => Err("Application continuity observation failed.".into()),
        }
    }
    #[cfg(feature = "native-test-faults")]
    pub fn verify_mutation_refusal(&mut self) -> bool {
        // The C mode gate must refuse before dereferencing any endpoint.
        unsafe { bcl_catalog_test_mutation_refused(self.session) == 1 }
    }
}
impl Drop for NativeCatalog {
    fn drop(&mut self) {
        unsafe { bcl_close(self.session) };
    }
}
#[cfg(all(test, feature = "native-test-faults"))]
mod activity_tests {
    #[test]
    fn pinned_client_activity_rejects_pid_reuse_replacement_and_ambiguous_streams() {
        assert_eq!(unsafe { super::bcl_catalog_test_activity_identity() }, 0);
    }
}
