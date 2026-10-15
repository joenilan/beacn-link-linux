//! Explicit private-core GUI qualification; events stay inside its owned window.
use beacn_link_protocol::{link_session::Phase, planner::ApplicationId};
use beacn_linx_core::model::Snapshot;
use gpui_kit::{test::TestWindowExt, *};
use std::{
    io::Write,
    path::PathBuf,
    time::{Duration, Instant},
};

pub struct Configuration {
    pub first: String,
    pub second: String,
    pub root: PathBuf,
    pub studio: bool,
    pub managed: bool,
    pub native_playback: bool,
    pub studio_recovery: bool,
}
pub struct Probe {
    config: Configuration,
    stage: u8,
    start: Instant,
    pause: Instant,
}
pub enum Action {
    Setup(String),
    Click(String),
    Name(String, String),
    ScrollAndSave(String),
    Unchanged(String),
    Search(String),
    Only(String, String),
    Capture(&'static str),
    Resize(f32, f32),
    Finish(PathBuf),
    Fail(PathBuf, String),
}
impl Probe {
    pub fn new(config: Configuration) -> Self {
        Self {
            config,
            stage: 0,
            start: Instant::now(),
            pause: Instant::now(),
        }
    }
    pub fn next(&mut self, snapshot: &Snapshot) -> Option<Action> {
        if self.config.studio_recovery {
            return self.next_studio_recovery(snapshot);
        }
        if self.config.native_playback {
            return self.next_native_playback(snapshot);
        }
        if self.config.studio {
            return self.next_studio(snapshot);
        }
        if self.config.managed {
            return self.next_managed(snapshot);
        }
        let first = ApplicationId(self.config.first.clone());
        let second = ApplicationId(self.config.second.clone());
        if self.start.elapsed() > Duration::from_secs(30) {
            return Some(Action::Fail(
                self.config.root.clone(),
                format!("stage={} message={:?}", self.stage, snapshot.message),
            ));
        }
        if self.pause.elapsed() < Duration::from_millis(220) {
            return None;
        }
        let action = match self.stage {
            0 if snapshot.connected => Action::Setup(format!("details-{}", first.0)),
            1 => Action::Name(first.0.clone(), "Media Player".into()),
            2 if snapshot.selected.len() == 1 => Action::Click(format!("details-{}", second.0)),
            3 => Action::Name(second.0.clone(), "Other Player".into()),
            4 if snapshot.selected.len() == 2 => Action::Click(format!("details-{}", first.0)),
            5 => Action::Click(format!("details-{}", second.0)),
            6 => Action::Search("Media Player".into()),
            7 => Action::Only(first.0.clone(), second.0.clone()),
            8 => Action::Search(String::new()),
            9 => Action::Capture("compact.ppm"),
            10 => Action::Resize(440., 520.),
            11 => Action::Capture("narrow.ppm"),
            12 => Action::Resize(900., 640.),
            13 => Action::Click("settings".into()),
            14 => Action::Click(format!("details-{}", first.0)),
            15 => Action::Capture("expanded.ppm"),
            16 => Action::Resize(440., 380.),
            17 => Action::Capture("small-expanded.ppm"),
            18 => Action::ScrollAndSave(first.0.clone()),
            19 => Action::Capture("small-scrolled.ppm"),
            20 => Action::Resize(900., 640.),
            21 => Action::Click("settings".into()),
            22 => Action::Click(format!("details-{}", first.0)),
            23 => Action::Resize(720., 520.),
            24 => Action::Click("session".into()),
            25 if snapshot.link.completed_cycles >= 2 && snapshot.link.phase == Phase::Running => {
                assert_eq!(snapshot.routing.assignments.len(), 2);
                assert!(snapshot.routing.assignments.values().any(|p| *p == 1));
                assert!(snapshot.routing.assignments.values().any(|p| *p == 4));
                assert_eq!(
                    beacn_linx_core::icon::Status::from_snapshot(snapshot),
                    beacn_linx_core::icon::Status::PrivateTest
                );
                Action::Capture("running.ppm")
            }
            26 => Action::Unchanged(first.0.clone()),
            27 if snapshot.link.phase == Phase::Running => {
                Action::Click(format!("details-{}", first.0))
            }
            28 => Action::Click("session".into()),
            29 if snapshot.link.phase == Phase::Stopped => {
                assert!(
                    snapshot
                        .link
                        .transport_cleanup
                        .as_ref()
                        .is_some_and(Result::is_ok)
                );
                assert!(snapshot.routing.cleanup.as_ref().is_some_and(Result::is_ok));
                Action::Click("session".into())
            }
            30 if snapshot.link.phase == Phase::Running && snapshot.link.completed_cycles >= 1 => {
                Action::Click(format!("select-{}", second.0))
            }
            31 if snapshot.selected.len() == 1 && snapshot.link.phase == Phase::Stopped => {
                assert!(
                    snapshot
                        .link
                        .transport_cleanup
                        .as_ref()
                        .is_some_and(Result::is_ok)
                );
                assert!(snapshot.routing.cleanup.as_ref().is_some_and(Result::is_ok));
                Action::Click("session".into())
            }
            32 if snapshot.link.phase == Phase::Running && snapshot.link.completed_cycles >= 1 => {
                Action::Click("session".into())
            }
            33 if snapshot.link.phase == Phase::Stopped => Action::Click("auto-connect".into()),
            34 if snapshot.auto_connect
                && snapshot.link.phase == Phase::Running
                && snapshot.link.completed_cycles >= 2 =>
            {
                assert_eq!(snapshot.routing.assignments.len(), 1);
                assert_eq!(snapshot.routing.assignments.values().next(), Some(&1));
                Action::Capture("automatic.ppm")
            }
            35 => Action::Click("session".into()),
            36 if snapshot.startup == beacn_linx_core::startup::State::Paused
                && snapshot.link.phase == Phase::Stopped
                && self.pause.elapsed() > Duration::from_millis(1100) =>
            {
                assert!(snapshot.auto_connect);
                assert!(
                    snapshot
                        .link
                        .transport_cleanup
                        .as_ref()
                        .is_some_and(Result::is_ok)
                );
                assert!(snapshot.routing.cleanup.as_ref().is_some_and(Result::is_ok));
                Action::Capture("paused.ppm")
            }
            37 => Action::Click("auto-connect".into()),
            38 if !snapshot.auto_connect && snapshot.link.phase == Phase::Stopped => {
                Action::Click("auto-connect".into())
            }
            39 if snapshot.auto_connect
                && snapshot.link.phase == Phase::Running
                && snapshot.link.completed_cycles >= 2 =>
            {
                assert_eq!(snapshot.routing.assignments.len(), 1);
                Action::Finish(self.config.root.clone())
            }
            _ => return None,
        };
        self.stage += 1;
        self.pause = Instant::now();
        Some(action)
    }
}
impl Probe {
    fn next_studio_recovery(&mut self, snapshot: &Snapshot) -> Option<Action> {
        if snapshot.link.error.is_some()
            || snapshot.link.phase == Phase::NeedsReview
            || self.start.elapsed() > Duration::from_secs(80)
        {
            return Some(Action::Fail(
                self.config.root.clone(),
                format!(
                    "Studio recovery stage={} message={:?}",
                    self.stage, snapshot.message
                ),
            ));
        }
        if self.pause.elapsed() < Duration::from_millis(220) {
            return None;
        }
        let assigned = snapshot.link.phase == Phase::Running
            && snapshot.routing.assignments.len() == 1
            && snapshot.routing.assignments.values().next() == Some(&1)
            && snapshot.link.completed_cycles + 8 < 40;
        let action = match self.stage {
            0 if snapshot.connected => Action::Click(format!("select-{}", self.config.first)),
            1 if snapshot.selected.len() == 1 => Action::Click("auto-connect".into()),
            2 if assigned => Action::Capture("recovery-first-running.ppm"),
            3 if assigned && self.pause.elapsed() > Duration::from_secs(4) => {
                Action::Click("session".into())
            }
            4 if snapshot.link.phase == Phase::Stopped
                && snapshot.startup == beacn_linx_core::startup::State::Paused
                && self.pause.elapsed() > Duration::from_millis(1200) =>
            {
                assert!(snapshot.auto_connect);
                assert!(
                    snapshot
                        .link
                        .transport_cleanup
                        .as_ref()
                        .is_some_and(Result::is_ok)
                );
                assert!(snapshot.routing.cleanup.as_ref().is_some_and(Result::is_ok));
                println!("STUDIO RECOVERY PAUSED");
                std::io::stdout().flush().unwrap();
                Action::Capture("recovery-paused.ppm")
            }
            5 if snapshot.link.phase == Phase::Stopped
                && snapshot.startup == beacn_linx_core::startup::State::Paused
                && self.pause.elapsed() > Duration::from_secs(1)
                // The hardware witness checks both closed PCMs, released USB,
                // full desktop/state and absence of our first route receipts.
                // A synthetic test has no physical target to acknowledge.
                && (snapshot.private_session
                    || std::fs::read(self.config.root.join("paused-witness.txt"))
                        .is_ok_and(|v| v == b"released baseline verified\n")) =>
            {
                println!("STUDIO RECOVERY RESUME");
                std::io::stdout().flush().unwrap();
                Action::Click("session".into())
            }
            6 if assigned => Action::Capture("recovery-resumed.ppm"),
            7 if assigned && self.pause.elapsed() > Duration::from_secs(4) => {
                Action::Finish(self.config.root.clone())
            }
            _ => return None,
        };
        self.stage += 1;
        self.pause = Instant::now();
        Some(action)
    }
    fn next_native_playback(&mut self, snapshot: &Snapshot) -> Option<Action> {
        if snapshot.link.error.is_some()
            || snapshot.link.phase == Phase::NeedsReview
            || self.start.elapsed() > Duration::from_secs(35)
        {
            return Some(Action::Fail(
                self.config.root.clone(),
                format!(
                    "native playback stage={} message={:?}",
                    self.stage, snapshot.message
                ),
            ));
        }
        if self.pause.elapsed() < Duration::from_millis(220) {
            return None;
        }
        let action = match self.stage {
            0 if snapshot.connected => Action::Click(format!("select-{}", self.config.first)),
            1 if snapshot.selected.len() == 1 => Action::Click("auto-connect".into()),
            2 if snapshot.link.phase == Phase::Running && snapshot.link.completed_cycles >= 4 => {
                assert!(snapshot.private_session);
                assert_eq!(snapshot.routing.assignments.values().next(), Some(&1));
                assert_eq!(
                    beacn_linx_core::icon::Status::from_snapshot(snapshot),
                    beacn_linx_core::icon::Status::PrivateTest
                );
                Action::Capture("native-playback-running.ppm")
            }
            3 => Action::Click("session".into()),
            4 if snapshot.link.phase == Phase::Stopped
                && self.pause.elapsed() > Duration::from_millis(1200) =>
            {
                assert_eq!(snapshot.startup, beacn_linx_core::startup::State::Paused);
                assert!(
                    snapshot
                        .link
                        .transport_cleanup
                        .as_ref()
                        .is_some_and(Result::is_ok)
                );
                assert!(snapshot.routing.cleanup.as_ref().is_some_and(Result::is_ok));
                Action::Capture("native-playback-paused.ppm")
            }
            5 => Action::Click("session".into()),
            6 if snapshot.link.phase == Phase::Running && snapshot.link.completed_cycles >= 4 => {
                Action::Finish(self.config.root.clone())
            }
            _ => return None,
        };
        self.stage += 1;
        self.pause = Instant::now();
        Some(action)
    }
    fn next_managed(&mut self, snapshot: &Snapshot) -> Option<Action> {
        if snapshot.link.phase == Phase::NeedsReview
            || (snapshot.link.error.is_some()
                && !(matches!(self.stage, 12 | 13) && snapshot.link.phase == Phase::Stopped))
            || self.start.elapsed() > Duration::from_secs(75)
        {
            return Some(Action::Fail(
                self.config.root.clone(),
                format!(
                    "managed stage={} phase={:?} message={:?}",
                    self.stage, snapshot.link.phase, snapshot.message
                ),
            ));
        }
        if self.pause.elapsed() < Duration::from_millis(220) {
            return None;
        }
        let action = match self.stage {
            0 if snapshot.connected => Action::Setup(format!("details-{}", self.config.first)),
            1 => Action::Name(self.config.first.clone(), "Media Player".into()),
            2 if snapshot.selected.len() == 1 => {
                Action::Click(format!("details-{}", self.config.second))
            }
            3 => Action::Name(self.config.second.clone(), "Other Player".into()),
            4 if snapshot.selected.len() == 2 => {
                Action::Click(format!("details-{}", self.config.first))
            }
            5 => Action::Click(format!("details-{}", self.config.second)),
            6 => Action::Click("auto-connect".into()),
            7 if snapshot.link.phase == Phase::Running && snapshot.link.completed_cycles >= 96 => {
                assert_eq!(snapshot.link.cycle_limit, None);
                assert_eq!(snapshot.routing.assignments.len(), 2);
                assert!(snapshot.routing.assignments.values().any(|p| *p == 1));
                assert!(snapshot.routing.assignments.values().any(|p| *p == 4));
                assert_eq!(
                    beacn_linx_core::icon::Status::from_snapshot(snapshot),
                    beacn_linx_core::icon::Status::PrivateTest
                );
                std::fs::write(
                    self.config.root.join("managed-progress.txt"),
                    format!(
                        "cycles={} applications=2 assignments=1,4 lifetime=managed\n",
                        snapshot.link.completed_cycles
                    ),
                )
                .unwrap();
                Action::Capture("managed-running.ppm")
            }
            8 => Action::Click("session".into()),
            9 if snapshot.link.phase == Phase::Stopped
                && snapshot.startup == beacn_linx_core::startup::State::Paused
                && self.pause.elapsed() > Duration::from_millis(1200) =>
            {
                assert!(snapshot.auto_connect);
                assert!(
                    snapshot
                        .link
                        .transport_cleanup
                        .as_ref()
                        .is_some_and(Result::is_ok)
                );
                assert!(snapshot.routing.cleanup.as_ref().is_some_and(Result::is_ok));
                Action::Capture("managed-paused.ppm")
            }
            10 => Action::Click("session".into()),
            11 if snapshot.link.phase == Phase::Running && snapshot.link.completed_cycles >= 3 => {
                assert_eq!(snapshot.routing.assignments.len(), 2);
                Action::Click(format!("select-{}", self.config.second))
            }
            12 if snapshot.link.phase == Phase::Stopped
                && snapshot.selected.len() == 1
                && self.pause.elapsed() > Duration::from_millis(1200) =>
            {
                assert!(
                    snapshot
                        .link
                        .transport_cleanup
                        .as_ref()
                        .is_some_and(Result::is_ok)
                );
                assert!(snapshot.routing.cleanup.as_ref().is_some_and(Result::is_ok));
                Action::Click("session".into())
            }
            13 if snapshot.link.phase == Phase::Running && snapshot.link.completed_cycles >= 3 => {
                assert_eq!(snapshot.routing.assignments.len(), 1);
                Action::Finish(self.config.root.clone())
            }
            _ => return None,
        };
        self.stage += 1;
        self.pause = Instant::now();
        Some(action)
    }
    fn next_studio(&mut self, snapshot: &Snapshot) -> Option<Action> {
        if snapshot.link.phase == Phase::NeedsReview || snapshot.link.error.is_some() {
            return Some(Action::Fail(
                self.config.root.clone(),
                format!("studio failure: {:?}", snapshot.message),
            ));
        }
        if self.start.elapsed() > Duration::from_secs(70) {
            return Some(Action::Fail(
                self.config.root.clone(),
                format!("studio stage={} message={:?}", self.stage, snapshot.message),
            ));
        }
        if self.pause.elapsed() < Duration::from_millis(220) {
            return None;
        }
        let action = match self.stage {
            0 if snapshot.connected => Action::Click(format!("select-{}", self.config.first)),
            1 if snapshot.selected.len() == 1 => Action::Click("auto-connect".into()),
            2 if snapshot.link.phase == Phase::Running && snapshot.link.completed_cycles >= 1 => {
                assert!(!snapshot.private_session);
                assert_eq!(
                    beacn_linx_core::icon::Status::from_snapshot(snapshot),
                    beacn_linx_core::icon::Status::Connected
                );
                Action::Capture("studio-connected.ppm")
            }
            3 if snapshot.link.phase == Phase::Stopped => {
                if Some(snapshot.link.completed_cycles) != snapshot.link.cycle_limit
                    || snapshot.link.error.is_some()
                {
                    return Some(Action::Fail(
                        self.config.root.clone(),
                        format!("studio stopped early: {:?}", snapshot.link.error),
                    ));
                }
                assert!(
                    snapshot
                        .link
                        .transport_cleanup
                        .as_ref()
                        .is_some_and(Result::is_ok)
                );
                assert!(snapshot.routing.cleanup.as_ref().is_some_and(Result::is_ok));
                Action::Finish(self.config.root.clone())
            }
            _ => return None,
        };
        eprintln!(
            "STUDIO GUI stage={} selected={} connected={}",
            self.stage,
            snapshot.selected.len(),
            snapshot.connected
        );
        self.stage += 1;
        self.pause = Instant::now();
        Some(action)
    }
    pub fn on_quit(&self, snapshot: &Snapshot) {
        if self.config.studio_recovery {
            if self.stage == 8 {
                assert_eq!(snapshot.link.phase, Phase::Stopped);
                assert!(
                    snapshot
                        .link
                        .transport_cleanup
                        .as_ref()
                        .is_some_and(Result::is_ok)
                );
                assert!(snapshot.routing.cleanup.as_ref().is_some_and(Result::is_ok));
                std::fs::write(self.config.root.join("recovery-result.txt"),
                    "sessions=2 pause_held=true explicit_resume=true quit_while_running=true route_release=verified transport_release=verified\n").unwrap();
            }
            return;
        }
        if self.config.native_playback {
            if self.stage == 7 {
                assert_eq!(snapshot.link.phase, Phase::Stopped);
                assert!(
                    snapshot
                        .link
                        .transport_cleanup
                        .as_ref()
                        .is_some_and(Result::is_ok)
                );
                assert!(snapshot.routing.cleanup.as_ref().is_some_and(Result::is_ok));
                std::fs::write(
                    self.config.root.join("result.txt"),
                    "native_playback=true usb=false pause_resume_quit=verified\n",
                )
                .unwrap();
            }
            return;
        }
        if self.config.managed {
            if self.stage == 14 {
                assert_eq!(snapshot.link.phase, Phase::Stopped);
                assert!(
                    snapshot
                        .link
                        .transport_cleanup
                        .as_ref()
                        .is_some_and(Result::is_ok)
                );
                assert!(snapshot.routing.cleanup.as_ref().is_some_and(Result::is_ok));
                let count = std::fs::read_dir("/tmp")
                    .unwrap()
                    .filter_map(Result::ok)
                    .filter(|e| {
                        e.file_name()
                            .to_string_lossy()
                            .starts_with(&format!("beacn-linx-catalog-{}-", std::process::id()))
                    })
                    .count();
                assert_eq!(count, 0);
                std::fs::write(self.config.root.join("result.txt"),
                    "managed=true cycles_at_least=96 applications=2 pause_held=true explicit_resume=true selection_loss_stopped=true quit_release=verified configs_removed=true\n").unwrap();
            }
            return;
        }
        if self.config.studio {
            if self.stage == 4 {
                assert_eq!(snapshot.link.phase, Phase::Stopped);
                assert!(
                    snapshot
                        .link
                        .transport_cleanup
                        .as_ref()
                        .is_some_and(Result::is_ok)
                );
                assert!(snapshot.routing.cleanup.as_ref().is_some_and(Result::is_ok));
                std::fs::write(self.config.root.join("studio-result.txt"),format!("hardware_session=true cycles={} route_release=verified transport_release=verified\n",snapshot.link.completed_cycles)).unwrap();
            }
            return;
        }
        if self.stage != 40 {
            return;
        }
        assert_eq!(snapshot.link.phase, Phase::Stopped);
        assert!(
            snapshot
                .link
                .transport_cleanup
                .as_ref()
                .is_some_and(Result::is_ok)
        );
        assert!(snapshot.routing.cleanup.as_ref().is_some_and(Result::is_ok));
        let count = std::fs::read_dir("/tmp")
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with(&format!("beacn-linx-catalog-{}-", std::process::id()))
            })
            .count();
        assert_eq!(count, 0);
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(self.config.root.join("result.txt"))
            .unwrap();
        writeln!(file,"widgets_selected=2 renamed=true removed=true worker_stopped=true session_started_stopped_restarted=true lifecycle_release_verified=true owned_configs_remaining=0 failed=false").unwrap();
        std::fs::write(self.config.root.join("quit-proof.txt"),"quit_while_running=true route_release=verified transport_release=verified configs_removed=true").unwrap();
        std::fs::write(
            self.config.root.join("startup-proof.txt"),
            "automatic_start=true pause_held=true rearmed_start=true quit_while_automatic=true\n",
        )
        .unwrap();
    }
}

pub fn execute(action: Action, window: &mut Window, cx: &mut App) {
    match action {
        Action::Setup(id) => {
            // Keep both simultaneously expanded name editors visible during setup.
            window.resize(size(px(720.), px(640.)));
            window.click(SharedString::from(id), cx);
        }
        Action::Click(id) => window.click(SharedString::from(id), cx),
        Action::Name(id, name) => {
            window.click(SharedString::from(format!("name-{id}")), cx);
            window.press("ctrl-a", cx);
            window.input(&name, cx);
            window.click(SharedString::from(format!("save-{id}")), cx);
        }
        Action::Search(text) => {
            window.click("search-input", cx);
            window.press("ctrl-a", cx);
            if text.is_empty() {
                window.press("backspace", cx);
            } else {
                window.input(&text, cx);
            }
        }
        Action::Only(first, second) => {
            assert!(
                window
                    .try_find(SharedString::from(format!("select-{first}")))
                    .is_some()
            );
            assert!(
                window
                    .try_find(SharedString::from(format!("select-{second}")))
                    .is_none()
            );
            window.resize(size(px(720.), px(520.)));
        }
        Action::Unchanged(first) => {
            window.click(SharedString::from(format!("details-{first}")), cx);
            window.click(SharedString::from(format!("save-{first}")), cx);
        }
        Action::ScrollAndSave(first) => {
            window.dispatch_event(
                PlatformInput::ScrollWheel(ScrollWheelEvent {
                    position: point(px(430.), px(300.)),
                    delta: ScrollDelta::Pixels(point(px(0.), px(-500.))),
                    modifiers: Modifiers::default(),
                    touch_phase: TouchPhase::Moved,
                }),
                cx,
            );
            window.render_frame(cx);
            window.click(SharedString::from(format!("save-{first}")), cx);
        }
        Action::Resize(w, h) => window.resize(size(px(w), px(h))),
        Action::Capture(name) => {
            window.render_frame(cx);
            let root =
                std::env::var_os("LINX_PRIVATE_GUI_EVIDENCE").expect("qualification evidence root");
            capture(window, &PathBuf::from(root).join(name));
        }
        Action::Finish(_) => {
            cx.quit();
        }
        Action::Fail(root, message) => {
            std::fs::write(root.join("failure.txt"), message).unwrap();
            cx.quit();
        }
    }
}
fn capture(window: &Window, path: &std::path::Path) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::ffi::{c_char, c_int, c_uint, c_ulong, c_void};
    #[link(name = "X11")]
    unsafe extern "C" {
        fn XOpenDisplay(name: *const c_char) -> *mut c_void;
        fn XGetGeometry(
            display: *mut c_void,
            window: c_ulong,
            root: *mut c_ulong,
            x: *mut c_int,
            y: *mut c_int,
            w: *mut c_uint,
            h: *mut c_uint,
            border: *mut c_uint,
            depth: *mut c_uint,
        ) -> c_int;
        fn XGetImage(
            display: *mut c_void,
            window: c_ulong,
            x: c_int,
            y: c_int,
            w: c_uint,
            h: c_uint,
            mask: c_ulong,
            format: c_int,
        ) -> *mut c_void;
        fn XGetPixel(image: *mut c_void, x: c_int, y: c_int) -> c_ulong;
        fn XDestroyImage(image: *mut c_void) -> c_int;
        fn XCloseDisplay(display: *mut c_void) -> c_int;
    }
    let handle = HasWindowHandle::window_handle(window).unwrap();
    let id = match handle.as_raw() {
        RawWindowHandle::Xlib(h) => h.window,
        RawWindowHandle::Xcb(h) => h.window.get() as c_ulong,
        _ => panic!("Xvfb qualification only"),
    };
    unsafe {
        let display = XOpenDisplay(std::ptr::null());
        assert!(!display.is_null());
        let (mut root, mut x, mut y, mut w, mut h, mut border, mut depth) = (0, 0, 0, 0, 0, 0, 0);
        assert_ne!(
            XGetGeometry(
                display,
                id,
                &mut root,
                &mut x,
                &mut y,
                &mut w,
                &mut h,
                &mut border,
                &mut depth
            ),
            0
        );
        assert!(w <= 1280 && h <= 900 && (depth == 24 || depth == 32));
        let image = XGetImage(display, id, 0, 0, w, h, !0, 2);
        assert!(!image.is_null());
        let mut bytes = format!("P6\n{w} {h}\n255\n").into_bytes();
        for y in 0..h {
            for x in 0..w {
                let p = XGetPixel(image, x as c_int, y as c_int);
                bytes.extend([(p >> 16) as u8, (p >> 8) as u8, p as u8]);
            }
        }
        XDestroyImage(image);
        XCloseDisplay(display);
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .unwrap();
        file.write_all(&bytes).unwrap();
    }
}
