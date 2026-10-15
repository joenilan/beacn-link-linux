//! Native catalog lives on its worker thread; no UI-thread barriers or mutations.
use crate::model::{Command, Model, Request, Snapshot};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender, TrySendError},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[derive(Clone)]
pub enum Connection {
    Desktop,
    Private { runtime: PathBuf, core: String },
}
pub struct Worker {
    tx: SyncSender<Request>,
    latest: Arc<Mutex<Snapshot>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}
impl Worker {
    pub fn spawn(connection: Connection) -> Self {
        let (tx, rx) = mpsc::sync_channel::<Request>(8);
        let latest = Arc::new(Mutex::new(Snapshot {
            busy: true,
            ..Snapshot::default()
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let state = latest.clone();
        let ending = stop.clone();
        let thread = thread::spawn(move || {
            let mut model = Model::default();
            let publish = |model: &Model| {
                if let Ok(mut state) = state.lock() {
                    *state = model.snapshot();
                }
            };
            #[cfg(all(feature = "native-catalog", target_os = "linux"))]
            let run = || -> Result<(), String> {
                use beacn_link_protocol::native_catalog::NativeCatalog;
                let mut catalog = match connection {
                    Connection::Desktop => NativeCatalog::desktop()?,
                    Connection::Private { runtime, core } => {
                        NativeCatalog::connect(&runtime, &core)?
                    }
                };
                let mut generation = 0u64;
                let mut command = None;
                while !ending.load(Ordering::Acquire) {
                    generation = generation.checked_add(1).ok_or("generation exhausted")?;
                    model.observe(catalog.observe(generation)?)?;
                    if ending.load(Ordering::Acquire) {
                        break;
                    }
                    if let Some(command) = command.take() {
                        let _ = model.request(command);
                    }
                    publish(&model);
                    match rx.recv_timeout(Duration::from_millis(500)) {
                        Ok(next) => command = Some(next),
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
                Ok(())
            };
            #[cfg(not(all(feature = "native-catalog", target_os = "linux")))]
            let run = || -> Result<(), String> {
                let _ = (&connection, &rx, &ending);
                Err("This build has no Linux application discovery. Use the native-catalog build on Linux.".into())
            };
            if let Err(error) = run() {
                model.disconnected(error);
                publish(&model);
            }
        });
        Self {
            tx,
            latest,
            stop,
            thread: Some(thread),
        }
    }
    pub fn snapshot(&self) -> Snapshot {
        self.latest.lock().expect("worker state poisoned").clone()
    }
    pub fn send(&self, command: Command, snapshot: &Snapshot) -> Result<(), String> {
        if self.stop.load(Ordering::Acquire) {
            return Err("Application discovery is stopped.".into());
        }
        let request = Request::from_view(command, snapshot)?;
        self.tx.try_send(request).map_err(|e| match e {
            TrySendError::Full(_) => "Please wait for application discovery.".into(),
            TrySendError::Disconnected(_) => {
                "Application discovery is stopped. Reopen the app to try again.".into()
            }
        })
    }
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.stop();
    }
}
