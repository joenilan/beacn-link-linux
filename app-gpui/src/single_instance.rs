//! One LinX per desktop session. The first instance holds an exclusive lock
//! in the user's runtime directory and listens on a private socket next to
//! it. A later launch (application menu, terminal, a second login autostart)
//! asks that instance to show its window and exits before it touches
//! PipeWire or the Studio, so two copies never compete for the device.
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::unix::{
        fs::{FileTypeExt, OpenOptionsExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::PathBuf,
    time::{Duration, Instant},
};

const LOCK: &str = "linx.lock";
const SOCKET: &str = "linx-instance.sock";
/// A starting first instance binds its socket right after taking the lock.
const CONNECT_WAIT: Duration = Duration::from_secs(3);

/// What a launch should do.
pub enum Claim {
    /// This is the only LinX: keep the guard for the process lifetime.
    Primary(Instance),
    /// Another LinX is running; the result of asking it to show (or to stay
    /// hidden). Never start a second copy, even when it did not answer.
    Running(io::Result<()>),
}

/// Held by the running LinX.
pub struct Instance {
    _lock: File,
    listener: UnixListener,
    socket: PathBuf,
}

/// A request from a later launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    /// Bring the window up (an ordinary launch).
    Show,
    /// A hidden launch (login autostart): nothing to show.
    Hidden,
}

fn runtime_dir() -> io::Result<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "XDG_RUNTIME_DIR is not set"))
}

/// Become the running LinX, or hand this launch to the one already running.
pub fn claim(hidden: bool) -> io::Result<Claim> {
    let dir = runtime_dir()?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .mode(0o600)
        .open(dir.join(LOCK))?;
    let socket = dir.join(SOCKET);
    match lock.try_lock() {
        Ok(()) => {
            // Holding the lock, any socket left at this path is stale.
            if fs::symlink_metadata(&socket).is_ok_and(|m| m.file_type().is_socket()) {
                fs::remove_file(&socket)?;
            }
            let listener = UnixListener::bind(&socket)?;
            fs::set_permissions(&socket, fs::Permissions::from_mode(0o600))?;
            listener.set_nonblocking(true)?;
            Ok(Claim::Primary(Instance {
                _lock: lock,
                listener,
                socket,
            }))
        }
        Err(fs::TryLockError::WouldBlock) => Ok(Claim::Running(forward(
            &socket,
            if hidden { Request::Hidden } else { Request::Show },
        ))),
        Err(fs::TryLockError::Error(error)) => Err(error),
    }
}

fn forward(socket: &PathBuf, request: Request) -> io::Result<()> {
    let started = Instant::now();
    let mut stream = loop {
        match UnixStream::connect(socket) {
            Ok(stream) => break stream,
            Err(_) if started.elapsed() < CONNECT_WAIT => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(error) => return Err(error),
        }
    };
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.write_all(match request {
        Request::Show => b"show\n",
        Request::Hidden => b"hidden\n",
    })?;
    // Wait for the acknowledgement so the request is never lost.
    let mut reply = [0u8; 3];
    stream.read_exact(&mut reply)?;
    Ok(())
}

impl Instance {
    /// Pending requests from later launches (never blocks).
    pub fn requests(&self) -> Vec<Request> {
        let mut out = Vec::new();
        while let Ok((mut stream, _)) = self.listener.accept() {
            let _ = stream.set_nonblocking(false);
            let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
            let mut line = [0u8; 16];
            let n = stream.read(&mut line).unwrap_or(0);
            let request = if line[..n].starts_with(b"hidden") {
                Request::Hidden
            } else {
                Request::Show
            };
            let _ = stream.write_all(b"ok\n");
            out.push(request);
        }
        out
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket);
    }
}
