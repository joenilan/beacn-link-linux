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
    #[cfg(all(feature = "desktop-session-test", target_os = "linux"))]
    DesktopStudioSession(crate::desktop_session::Configuration),
    #[cfg(all(feature = "desktop-session-test", target_os = "linux"))]
    Studio,
    Private {
        runtime: PathBuf,
        core: String,
    },
    #[cfg(all(feature = "private-session-test", target_os = "linux"))]
    PrivateSession(crate::private_session::Configuration),
    #[cfg(all(feature = "private-session-test", target_os = "linux"))]
    ManagedPrivateSession(crate::private_session::Configuration),
    #[cfg(all(feature = "studio-session-test", target_os = "linux"))]
    StudioSession(crate::studio_session::Configuration),
    #[cfg(all(feature = "native-playback-test", target_os = "linux"))]
    NativePlaybackSession(crate::studio_session::Configuration),
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
            let persistent = match &connection {
                Connection::Desktop => true,
                #[cfg(all(feature = "desktop-session-test", target_os = "linux"))]
                Connection::Studio => true,
                _ => false,
            };
            let mut preferences = None;
            let mut preference_error = None;
            let mut automatic = false; // explicit private fixtures remain manual
            if persistent {
                match crate::preferences::Preferences::user_path()
                    .ok_or_else(|| "No user configuration directory is available.".to_string())
                    .and_then(crate::preferences::Preferences::load)
                {
                    Ok((store, enabled)) => {
                        preferences = Some(store);
                        automatic = enabled;
                    }
                    Err(error) => preference_error = Some(error),
                }
            }
            let mut supervisor = crate::startup::Supervisor::new(automatic);
            #[cfg(all(feature = "private-session-test", target_os = "linux"))]
            let mut link = crate::session_owner::Owner::new(&connection);
            #[cfg(all(feature = "native-catalog", target_os = "linux"))]
            let mut catalog = None;
            let mut generation = 0u64;
            let mut command = None;
            let mut retry_at = std::time::Instant::now();
            let mut next_tick = std::time::Instant::now() + Duration::from_millis(500);
            while !ending.load(Ordering::Acquire) {
                #[cfg(all(feature = "native-playback-test", target_os = "linux"))]
                if matches!(&connection,Connection::StudioSession(config)
                    if config.playback==crate::studio_session::Playback::NativeStudio)
                    && crate::studio_session::native_supervisor_closed()
                {
                    model.notice(
                        "Native qualification supervisor ended; releasing owned playback.".into(),
                    );
                    break;
                }
                #[cfg(all(feature = "native-catalog", target_os = "linux"))]
                let observation = (|| {
                    use beacn_link_protocol::native_catalog::NativeCatalog;
                    if catalog.is_none() {
                        if std::time::Instant::now() < retry_at {
                            return Ok(());
                        }
                        catalog = Some(match &connection {
                            Connection::Desktop => NativeCatalog::desktop()?,
                            #[cfg(feature = "desktop-session-test")]
                            Connection::DesktopStudioSession(_) | Connection::Studio => {
                                NativeCatalog::desktop()?
                            }
                            Connection::Private { runtime, core } => {
                                NativeCatalog::connect(runtime, core)?
                            }
                            #[cfg(feature = "private-session-test")]
                            Connection::PrivateSession(config)
                            | Connection::ManagedPrivateSession(config) => {
                                NativeCatalog::connect(&config.runtime, &config.core)?
                            }
                            #[cfg(feature = "studio-session-test")]
                            Connection::StudioSession(config) => NativeCatalog::connect(
                                &config.routing.runtime,
                                &config.routing.core,
                            )?,
                            #[cfg(feature = "native-playback-test")]
                            Connection::NativePlaybackSession(config) => NativeCatalog::connect(
                                &config.routing.runtime,
                                &config.routing.core,
                            )?,
                        });
                    }
                    generation = generation.checked_add(1).ok_or("generation exhausted")?;
                    model.observe(catalog.as_mut().unwrap().observe(generation)?)
                })();
                #[cfg(not(all(feature = "native-catalog", target_os = "linux")))]
                let observation: Result<(), String> = {
                    let _ = (&connection, &mut generation, &mut retry_at);
                    Err("This build has no Linux application discovery.".into())
                };
                if let Err(error) = observation {
                    #[cfg(all(feature = "private-session-test", target_os = "linux"))]
                    {
                        let _ = link.stop(
                            beacn_link_protocol::routing_control::StopReason::OperationFailed(
                                error.clone(),
                            ),
                        );
                        model.session(link.status().clone(), link.available(), link.private());
                    }
                    #[cfg(all(feature = "native-catalog", target_os = "linux"))]
                    {
                        catalog = None;
                    }
                    retry_at = std::time::Instant::now() + Duration::from_secs(2);
                    model.disconnected(error);
                }
                if ending.load(Ordering::Acquire) {
                    break;
                }
                #[cfg(all(feature = "private-session-test", target_os = "linux"))]
                let mut just_started = false;
                if let Some(request) = command.take() {
                    match model.accept(request) {
                        Ok(command) => {
                            let result = match command {
                                Command::AutoConnect(enabled) => {
                                    let saved = if let Some(store) = preferences.as_mut() {
                                        store.save(enabled)
                                    } else if persistent {
                                        Err(preference_error.clone().unwrap_or_else(|| {
                                            "LinX preferences are unavailable.".into()
                                        }))
                                    } else {
                                        Ok(())
                                    };
                                    match saved {
                                        Ok(()) => {
                                            supervisor.set_automatic(enabled);
                                            preference_error = None;
                                            Ok(())
                                        }
                                        Err(error) => {
                                            preference_error = Some(error.clone());
                                            Err(error)
                                        }
                                    }
                                }
                                Command::Stop => {
                                    supervisor.pause();
                                    #[cfg(all(
                                        feature = "private-session-test",
                                        target_os = "linux"
                                    ))]
                                    {
                                        link.stop(
                                            beacn_link_protocol::routing_control::StopReason::Owner,
                                        )
                                    }
                                    #[cfg(not(all(
                                        feature = "private-session-test",
                                        target_os = "linux"
                                    )))]
                                    {
                                        Ok(())
                                    }
                                }
                                Command::Start => {
                                    supervisor.manual_start();
                                    #[cfg(all(
                                        feature = "private-session-test",
                                        target_os = "linux"
                                    ))]
                                    {
                                        let result = link.start(model.service());
                                        just_started = result.is_ok();
                                        result
                                    }
                                    #[cfg(not(all(
                                        feature = "private-session-test",
                                        target_os = "linux"
                                    )))]
                                    {
                                        Err("Device connection is not enabled in this preview."
                                            .into())
                                    }
                                }
                                Command::Assign(id, alias, pair) => {
                                    #[cfg(all(
                                        feature = "desktop-session-test",
                                        target_os = "linux"
                                    ))]
                                    {
                                        if !matches!(
                                            &connection,
                                            Connection::Studio
                                                | Connection::DesktopStudioSession(_)
                                        ) {
                                            Err("Local assignment is available only for a desktop Studio session.".into())
                                        } else {
                                            (|| {
                                                if model.service().selected().get(&id).is_none() {
                                                    model.command(Command::Select(
                                                        id.clone(),
                                                        alias,
                                                    ))?;
                                                }
                                                supervisor.manual_start();
                                                if link.status().phase
                                                    != beacn_link_protocol::link_session::Phase::Running
                                                {
                                                    link.start(model.service())?;
                                                    just_started = true;
                                                }
                                                let assigned = link.assign(model.service(), pair);
                                                if assigned.is_ok() {
                                                    // The next serial tick needs a newer catalog observation
                                                    // than the one sealed into this assignment request.
                                                    just_started = true;
                                                }
                                                assigned
                                            })()
                                        }
                                    }
                                    #[cfg(not(all(
                                        feature = "desktop-session-test",
                                        target_os = "linux"
                                    )))]
                                    {
                                        let _ = (id, alias, pair);
                                        Err("Local assignment is not enabled in this build.".into())
                                    }
                                }
                                command => model.command(command),
                            };
                            if let Err(error) = result {
                                model.notice(error);
                            }
                        }
                        Err(error) => model.notice(error),
                    }
                }
                #[cfg(all(feature = "private-session-test", target_os = "linux"))]
                {
                    if let Err(error) = link.check(model.service()) {
                        model.notice(error);
                    }
                    if !just_started
                        && std::time::Instant::now() >= next_tick
                        && link.status().phase == beacn_link_protocol::link_session::Phase::Running
                    {
                        if let Err(error) = link.tick(model.service()) {
                            model.notice(error);
                        }
                    }
                    let view = model.snapshot();
                    if supervisor.update(
                        link.status(),
                        link.available() && view.connected,
                        !view.selected.is_empty(),
                    ) {
                        match link.start(model.service()) {
                            Ok(()) => just_started = true,
                            Err(error) => model.notice(error),
                        }
                    }

                    // Refresh displayed state after an automatic start; its attempt is latched.
                    supervisor.update(
                        link.status(),
                        link.available() && view.connected,
                        !view.selected.is_empty(),
                    );
                    model.session(link.status().clone(), link.available(), link.private());
                    #[cfg(feature = "desktop-session-test")]
                    model.desktop_cleanup(link.desktop_cleanup());
                    if just_started {
                        next_tick = std::time::Instant::now() + Duration::from_millis(500);
                    }
                }
                #[cfg(not(all(feature = "private-session-test", target_os = "linux")))]
                {
                    supervisor.update(&model.snapshot().link, false, false);
                }
                model.startup(&supervisor, preference_error.clone());
                if std::time::Instant::now() >= next_tick {
                    next_tick = std::time::Instant::now() + Duration::from_millis(500);
                }
                publish(&model);
                match rx
                    .recv_timeout(next_tick.saturating_duration_since(std::time::Instant::now()))
                {
                    Ok(next) => command = Some(next),
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
            #[cfg(all(feature = "private-session-test", target_os = "linux"))]
            {
                if let Err(error) =
                    link.stop(beacn_link_protocol::routing_control::StopReason::Shutdown)
                {
                    model.notice(error);
                }
                model.session(link.status().clone(), link.available(), link.private());
                #[cfg(feature = "desktop-session-test")]
                model.desktop_cleanup(link.desktop_cleanup());
            }
            // Drop native discovery before publishing the final joined state.
            #[cfg(all(feature = "native-catalog", target_os = "linux"))]
            drop(catalog);
            publish(&model);
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
