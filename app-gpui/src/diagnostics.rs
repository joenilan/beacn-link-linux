//! Explicit developer capture of this app's own rendered scene. No desktop,
//! microphone, device or application selection is captured or changed here.
use gpui_kit::{App, Bounds, Pixels, Window};
use std::{
    io::Write,
    path::PathBuf,
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

static CAPTURE: OnceLock<(PathBuf, Instant)> = OnceLock::new();
static FINISHED: AtomicBool = AtomicBool::new(false);
static ROW_RECORDED: AtomicBool = AtomicBool::new(false);
static LAST_STATE: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

pub fn state(snapshot: &beacn_linx_core::model::Snapshot) {
    if CAPTURE.get().is_none() {
        return;
    }
    let current = format!(
        "phase={:?} startup={:?} error={:?} assignments={:?} selected={:?} reply={:?}",
        snapshot.link.phase,
        snapshot.startup,
        snapshot.link.error,
        snapshot.routing.assignments.values().collect::<Vec<_>>(),
        snapshot.selected.values().collect::<Vec<_>>(),
        snapshot.link.last_reply
    );
    let mut last = LAST_STATE.lock().unwrap();
    if last.as_ref() != Some(&current) {
        eprintln!(
            "LinX native state: {current}; cycles={}",
            snapshot.link.completed_cycles
        );
        *last = Some(current);
    }
}

pub fn configure(path: PathBuf) {
    CAPTURE
        .set((path, Instant::now()))
        .expect("one frame capture");
}

pub fn row(bounds: &[Bounds<Pixels>], window: &Window) {
    let Some((path, _)) = CAPTURE.get() else {
        return;
    };
    if bounds.len() != 4 || ROW_RECORDED.swap(true, Ordering::Relaxed) {
        return;
    }
    let text = format!(
        "viewport={:?}\nchannel_bounds={bounds:?}\n",
        window.viewport_size()
    );
    let result = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path.with_extension("layout.txt"))
        .and_then(|mut file| file.write_all(text.as_bytes()));
    eprintln!("LinX actual four-column layout: {text}saved={result:?}");
}

pub fn capture(window: &Window, _cx: &mut App) {
    let Some((path, start)) = CAPTURE.get() else {
        return;
    };
    if FINISHED.load(Ordering::Relaxed) || start.elapsed() < Duration::from_secs(3) {
        return;
    }
    match window.render_to_image() {
        Ok(image) => {
            let result = (|| -> std::io::Result<()> {
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path)?;
                write!(file, "P6\n{} {}\n255\n", image.width(), image.height())?;
                for p in image.pixels() {
                    file.write_all(&p.0[..3])?;
                }
                Ok(())
            })();
            eprintln!("LinX own-window frame capture: {result:?}");
            FINISHED.store(true, Ordering::Relaxed);
        }
        Err(error) => {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            if matches!(
                HasWindowHandle::window_handle(window).map(|h| h.as_raw()),
                Ok(RawWindowHandle::Xlib(_) | RawWindowHandle::Xcb(_))
            ) {
                crate::qualification::capture(window, path);
                eprintln!("LinX captured own X11 window (GPU readback unsupported: {error}).");
            } else {
                eprintln!("LinX own-window capture unavailable: {error}");
            }
            FINISHED.store(true, Ordering::Relaxed);
        }
    }
}
