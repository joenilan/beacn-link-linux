//! Opt-in, same-user Linux control socket. UI actions stay in the GPUI owner.
//!
//! The socket accepts one local controller at a time, passes bounded JSONL
//! requests to the UI thread, and waits for that thread to return a response.
//! It never opens a TCP listener or dispatches UI work itself.
#![cfg(target_os = "linux")]

use serde_json::{Value, json};
use std::{
    fs,
    io::{self, Read, Write},
    os::unix::{
        fs::{FileTypeExt, MetadataExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const MAX_REQUEST_BYTES: usize = 16 * 1024;
const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const QUEUE_CAPACITY: usize = 8;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const READ_POLL: Duration = Duration::from_millis(100);
const CLIENT_IDLE_TIMEOUT: Duration = Duration::from_secs(5);
const SOL_SOCKET: i32 = 1;
const SO_PEERCRED: i32 = 17;

static CONFIGURED_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Validate and remember the explicit CLI socket path without binding it.
pub fn configure(path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref().to_owned();
    validate_parent(&path)?;
    if CONFIGURED_PATH.set(path).is_err() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "control socket path is already configured",
        ));
    }
    Ok(())
}

/// Private screenshot destination associated with the configured socket.
pub fn screenshot_path() -> Option<PathBuf> {
    CONFIGURED_PATH.get().and_then(|path| {
        Some(
            path.parent()?
                .join(format!("{}.ppm", path.file_name()?.to_string_lossy())),
        )
    })
}

#[repr(C)]
struct PeerCred {
    pid: i32,
    uid: u32,
    gid: u32,
}

unsafe extern "C" {
    fn getsockopt(
        socket: i32,
        level: i32,
        option: i32,
        value: *mut std::ffi::c_void,
        length: *mut u32,
    ) -> i32;
    fn geteuid() -> u32;
}

/// A UI-thread request. `value` contains the caller's `id` and `op` fields.
/// Respond with the result object; the transport wraps it with the same id.
pub struct Request {
    pub value: Value,
    id: Value,
    expires: Instant,
    reply: SyncSender<Value>,
    cancelled: Arc<AtomicBool>,
}

impl Request {
    pub fn id(&self) -> &Value {
        &self.id
    }

    pub fn op(&self) -> &str {
        self.value.get("op").and_then(Value::as_str).unwrap_or("")
    }

    /// Timed-out/cancelled requests must not be dispatched later.
    pub fn expired(&self) -> bool {
        self.cancelled.load(Ordering::Acquire) || Instant::now() >= self.expires
    }

    pub fn respond(self, response: Value) -> Result<(), Value> {
        self.reply.send(response).map_err(|error| error.0)
    }
}

/// Owns one explicitly configured socket and its bounded UI request queue.
#[derive(Default)]
pub struct Control {
    receiver: Option<Receiver<Request>>,
    stop: Option<Arc<AtomicBool>>,
    listener_thread: Option<JoinHandle<()>>,
    socket: Option<OwnedSocketPath>,
}

struct OwnedSocketPath {
    path: PathBuf,
    device: u64,
    inode: u64,
}

impl Control {
    /// Bind the path remembered by [`configure`]. Existing files are preserved.
    pub fn from_configuration() -> io::Result<Self> {
        let Some(path) = CONFIGURED_PATH.get() else {
            return Ok(Self::default());
        };
        Self::bind(path)
    }

    fn bind(path: impl AsRef<Path>) -> io::Result<Self> {
        let mut control = Self::default();
        control.bind_into(path)?;
        Ok(control)
    }

    fn bind_into(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        if self.listener_thread.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "control socket is already configured",
            ));
        }
        let path = path.as_ref();
        validate_parent(path)?;
        let listener = UnixListener::bind(path)?;
        let initial = match fs::symlink_metadata(path) {
            Ok(metadata)
                if metadata.file_type().is_socket() && metadata.uid() == unsafe { geteuid() } =>
            {
                metadata
            }
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "bound control socket identity is invalid",
                ));
            }
            Err(error) => return Err(error),
        };
        let owned = OwnedSocketPath {
            path: path.to_owned(),
            device: initial.dev(),
            inode: initial.ino(),
        };
        if let Err(error) = fs::set_permissions(path, fs::Permissions::from_mode(0o600)) {
            remove_owned_socket(&owned);
            return Err(error);
        }
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) => {
                remove_owned_socket(&owned);
                return Err(error);
            }
        };
        if !metadata.file_type().is_socket()
            || metadata.uid() != unsafe { geteuid() }
            || metadata.mode() & 0o777 != 0o600
            || metadata.dev() != owned.device
            || metadata.ino() != owned.inode
        {
            remove_owned_socket(&owned);
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "control socket ownership or permissions are invalid",
            ));
        }
        if let Err(error) = listener.set_nonblocking(true) {
            remove_owned_socket(&owned);
            return Err(error);
        }
        let (sender, receiver) = mpsc::sync_channel(QUEUE_CAPACITY);
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let thread = match thread::Builder::new()
            .name("linx-control-socket".into())
            .spawn(move || serve(listener, sender, thread_stop))
        {
            Ok(thread) => thread,
            Err(error) => {
                remove_owned_socket(&owned);
                return Err(error);
            }
        };
        self.receiver = Some(receiver);
        self.stop = Some(stop);
        self.listener_thread = Some(thread);
        self.socket = Some(owned);
        Ok(())
    }

    /// Nonblocking alias suitable for the GPUI event/poll loop.
    pub fn poll(&self) -> Option<Request> {
        match self.receiver.as_ref()?.try_recv() {
            Ok(request) => Some(request),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
    }

    pub fn take(&self) -> Option<Request> {
        self.poll()
    }
}

impl Drop for Control {
    fn drop(&mut self) {
        if let Some(stop) = &self.stop {
            stop.store(true, Ordering::Release);
        }
        if let Some(thread) = self.listener_thread.take() {
            let _ = thread.join();
        }
        if let Some(socket) = self.socket.take() {
            remove_owned_socket(&socket);
        }
    }
}

fn validate_parent(path: &Path) -> io::Result<()> {
    if !path.is_absolute() || path.file_name().is_none() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "control socket path must be absolute",
        ));
    }
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "control socket parent is absent",
        )
    })?;
    let metadata = fs::metadata(parent)?;
    if !metadata.is_dir()
        || metadata.uid() != unsafe { geteuid() }
        || metadata.mode() & 0o077 != 0
        || fs::canonicalize(parent)? != parent
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "control socket parent must be a private directory owned by this user",
        ));
    }
    Ok(())
}

fn serve(listener: UnixListener, sender: SyncSender<Request>, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::Acquire) {
        match listener.accept() {
            Ok((stream, _)) => {
                if peer_uid(&stream).ok() != Some(unsafe { geteuid() }) {
                    continue;
                }
                serve_client(stream, &sender, &stop);
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(READ_POLL);
            }
            Err(_) => break,
        }
    }
}

fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    let mut credentials = PeerCred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut length = std::mem::size_of::<PeerCred>() as u32;
    // SAFETY: the kernel fills the fixed-size Linux SO_PEERCRED structure.
    let result = unsafe {
        getsockopt(
            stream.as_raw_fd(),
            SOL_SOCKET,
            SO_PEERCRED,
            (&mut credentials as *mut PeerCred).cast(),
            &mut length,
        )
    };
    if result != 0 || length as usize != std::mem::size_of::<PeerCred>() {
        return Err(io::Error::last_os_error());
    }
    Ok(credentials.uid)
}

fn serve_client(mut stream: UnixStream, sender: &SyncSender<Request>, stop: &AtomicBool) {
    let _ = stream.set_read_timeout(Some(READ_POLL));
    let _ = stream.set_write_timeout(Some(REQUEST_TIMEOUT));
    let mut pending = Vec::new();
    let mut last_request = Instant::now();
    while !stop.load(Ordering::Acquire) {
        if last_request.elapsed() >= CLIENT_IDLE_TIMEOUT {
            return;
        }
        let line = match read_line(&mut stream, &mut pending) {
            Ok(Some(line)) => line,
            Ok(None) => continue,
            Err(_) => return,
        };
        last_request = Instant::now();
        let value: Value = match serde_json::from_slice(&line) {
            Ok(value) => value,
            Err(_) => {
                if write_error(&mut stream, Value::Null, "invalid JSON request").is_err() {
                    return;
                }
                continue;
            }
        };
        let Some(id) = value
            .get("id")
            .filter(|id| id.is_string() || id.is_number())
            .cloned()
        else {
            if write_error(
                &mut stream,
                Value::Null,
                "request id must be a string or number",
            )
            .is_err()
            {
                return;
            }
            continue;
        };
        let valid_op = value
            .get("op")
            .and_then(Value::as_str)
            .is_some_and(|op| !op.is_empty() && op.len() <= 64);
        if !value.is_object() || !valid_op {
            if write_error(&mut stream, id, "request op is missing or invalid").is_err() {
                return;
            }
            continue;
        }
        let (reply, response) = mpsc::sync_channel(1);
        let cancelled = Arc::new(AtomicBool::new(false));
        let request = Request {
            value,
            id: id.clone(),
            expires: Instant::now() + REQUEST_TIMEOUT,
            reply,
            cancelled: cancelled.clone(),
        };
        match sender.try_send(request) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                if write_error(&mut stream, id, "UI request queue is full").is_err() {
                    return;
                }
                continue;
            }
            Err(TrySendError::Disconnected(_)) => return,
        }
        match response.recv_timeout(REQUEST_TIMEOUT) {
            Ok(result) => {
                let frame = json!({ "id": id, "result": result });
                if write_frame(&mut stream, &frame).is_err() {
                    return;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                cancelled.store(true, Ordering::Release);
                if write_error(&mut stream, id, "UI request timed out").is_err() {
                    return;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
    }
}

fn read_line(stream: &mut UnixStream, pending: &mut Vec<u8>) -> io::Result<Option<Vec<u8>>> {
    loop {
        if let Some(end) = pending.iter().position(|byte| *byte == b'\n') {
            if end > MAX_REQUEST_BYTES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "request too large",
                ));
            }
            let mut line: Vec<_> = pending.drain(..=end).collect();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            return Ok(Some(line));
        }
        if pending.len() > MAX_REQUEST_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "request too large",
            ));
        }
        let mut chunk = [0u8; 2048];
        match stream.read(&mut chunk) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "client closed",
                ));
            }
            Ok(count) => {
                pending.extend_from_slice(&chunk[..count]);
                if pending.len() > MAX_REQUEST_BYTES + 1 && !pending.contains(&b'\n') {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "request too large",
                    ));
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                return Ok(None);
            }
            Err(error) => return Err(error),
        }
    }
}

fn write_error(stream: &mut UnixStream, id: Value, error: &str) -> io::Result<()> {
    write_frame(stream, &json!({ "id": id, "error": error }))
}

fn write_frame(stream: &mut UnixStream, frame: &Value) -> io::Result<()> {
    let mut bytes = serde_json::to_vec(frame)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "response too large",
        ));
    }
    bytes.push(b'\n');
    stream.write_all(&bytes)
}

fn remove_owned_socket(owned: &OwnedSocketPath) {
    if fs::symlink_metadata(&owned.path).is_ok_and(|metadata| {
        metadata.file_type().is_socket()
            && metadata.uid() == unsafe { geteuid() }
            && metadata.dev() == owned.device
            && metadata.ino() == owned.inode
    }) {
        let _ = fs::remove_file(&owned.path);
    }
}

use std::os::fd::AsRawFd;

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        os::unix::fs::PermissionsExt,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT: AtomicU64 = AtomicU64::new(0);
    #[test]
    fn omitted_control_flag_keeps_the_endpoint_disabled() {
        let control = Control::from_configuration().unwrap();
        assert!(control.socket.is_none());
        assert!(control.take().is_none());
    }

    fn private_dir() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "linx-control-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }

    #[test]
    fn local_same_user_request_round_trips_through_ui_queue() {
        let dir = private_dir();
        let path = dir.join("control.sock");
        let control = Control::bind(&path).unwrap();
        let metadata = fs::symlink_metadata(&path).unwrap();
        assert_eq!(metadata.mode() & 0o777, 0o600);
        let mut client = UnixStream::connect(&path).unwrap();
        client.write_all(br#"{"id":7,"op":"state"}"#).unwrap();
        client.write_all(b"\n").unwrap();
        let request = loop {
            if let Some(request) = control.poll() {
                break request;
            }
            thread::sleep(Duration::from_millis(5));
        };
        assert_eq!(request.id(), &json!(7));
        assert_eq!(request.op(), "state");
        request.respond(json!({"status":"ready"})).unwrap();
        let mut reply = Vec::new();
        let mut reader = io::BufReader::new(client);
        io::BufRead::read_until(&mut reader, b'\n', &mut reply).unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&reply).unwrap(),
            json!({
                "id": 7,
                "result": { "status": "ready" }
            })
        );
        drop(control);
        assert!(!path.exists());
        fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn existing_path_is_preserved_and_overlong_lines_are_rejected() {
        let dir = private_dir();
        let path = dir.join("occupied");
        fs::write(&path, b"keep").unwrap();
        assert!(Control::bind(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"keep");
        let mut pending = vec![b'x'; MAX_REQUEST_BYTES + 1];
        assert_eq!(
            read_line(&mut UnixStream::pair().unwrap().0, &mut pending)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        fs::remove_file(path).unwrap();
        fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn idle_client_cannot_block_the_next_controller_indefinitely() {
        let dir = private_dir();
        let path = dir.join("control.sock");
        let control = Control::bind(&path).unwrap();
        let idle = UnixStream::connect(&path).unwrap();
        std::thread::sleep(READ_POLL * 2);
        let mut next = UnixStream::connect(&path).unwrap();
        next.write_all(b"{\"id\":2,\"op\":\"state\"}\n").unwrap();
        let deadline = Instant::now() + CLIENT_IDLE_TIMEOUT + Duration::from_secs(2);
        let request = loop {
            if let Some(request) = control.take() {
                break request;
            }
            assert!(
                Instant::now() < deadline,
                "idle controller kept listener occupied"
            );
            std::thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(request.id(), &json!(2));
        request.respond(json!({"ready":true})).unwrap();
        drop(idle);
        drop(next);
        drop(control);
        assert!(!path.exists());
        fs::remove_dir(dir).unwrap();
    }
}
