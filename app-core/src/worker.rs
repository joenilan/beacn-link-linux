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
    Private {
        runtime: PathBuf,
        core: String,
    },
    #[cfg(all(feature = "private-session-test", target_os = "linux"))]
    PrivateSession(crate::private_session::Configuration),
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
                #[cfg(feature = "private-session-test")]
                let config = match &connection {
                    Connection::PrivateSession(config) => Some(config.clone()),
                    _ => None,
                };
                #[cfg(feature = "private-session-test")]
                let mut link = crate::private_session::Session::default();
                let mut catalog = match connection {
                    Connection::Desktop => NativeCatalog::desktop()?,
                    Connection::Private { runtime, core } => {
                        NativeCatalog::connect(&runtime, &core)?
                    }
                    #[cfg(feature = "private-session-test")]
                    Connection::PrivateSession(config) => {
                        NativeCatalog::connect(&config.runtime, &config.core)?
                    }
                };
                let mut generation = 0u64;
                let mut command = None;
                let mut next_tick = std::time::Instant::now() + Duration::from_millis(500);
                let result = (|| -> Result<(), String> {
                    while !ending.load(Ordering::Acquire) {
                        generation = generation.checked_add(1).ok_or("generation exhausted")?;
                        model.observe(catalog.observe(generation)?)?;
                        if ending.load(Ordering::Acquire) {
                            break;
                        }
                        #[cfg(feature = "private-session-test")]
                        let mut just_started = false;
                        if let Some(command) = command.take() {
                            match model.accept(command) {
                                Ok(command) => {
                                    #[cfg(feature = "private-session-test")]
                                    let result = match command {
                                        Command::Start => match &config {
                                            Some(config) => crate::private_session::start(
                                                &mut link,
                                                model.service(),
                                                config,
                                            ),
                                            None => Err(
                                                "No routing backend is enabled in this build mode."
                                                    .into(),
                                            ),
                                        },
                                        Command::Stop => link.stop(
                                            beacn_link_protocol::routing_control::StopReason::Owner,
                                        ),
                                        command => model.command(command),
                                    };
                                    #[cfg(not(feature = "private-session-test"))]
                                    let result = model.command(command);
                                    #[cfg(feature = "private-session-test")]
                                    if result.is_ok()
                                        && link.status().phase
                                            == beacn_link_protocol::link_session::Phase::Running
                                        && link.status().completed_cycles == 0
                                    {
                                        just_started = true;
                                        next_tick =
                                            std::time::Instant::now() + Duration::from_millis(500);
                                    }
                                    if let Err(error) = result {
                                        model.notice(error);
                                    }
                                }
                                Err(error) => model.notice(error),
                            }
                        }
                        #[cfg(feature = "private-session-test")]
                        {
                            if let Err(error) = link.check(model.service()) {
                                model.notice(error);
                            }
                            if !just_started
                                && std::time::Instant::now() >= next_tick
                                && link.status().phase
                                    == beacn_link_protocol::link_session::Phase::Running
                            {
                                if let Err(error) = link.tick(model.service()) {
                                    model.notice(error);
                                }
                            }
                            model.session(link.status().clone(), config.is_some());
                        }
                        if std::time::Instant::now() >= next_tick {
                            next_tick = std::time::Instant::now() + Duration::from_millis(500);
                        }
                        publish(&model);
                        match rx.recv_timeout(
                            next_tick.saturating_duration_since(std::time::Instant::now()),
                        ) {
                            Ok(next) => command = Some(next),
                            Err(mpsc::RecvTimeoutError::Timeout) => {}
                            Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        }
                    }
                    Ok(())
                })();
                #[cfg(feature = "private-session-test")]
                {
                    if let Err(error) =
                        link.stop(beacn_link_protocol::routing_control::StopReason::Shutdown)
                    {
                        model.notice(error);
                    }
                    model.session(link.status().clone(), config.is_some());
                    publish(&model);
                }
                result
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
