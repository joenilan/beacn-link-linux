//! Headless LinX (`--studio --headless`): what BEACN Link does on Windows,
//! without a window or the interface toolkit. The Link engine routes, BEACN
//! App on the streaming PC assigns, and the tray offers the sound output,
//! pause and quit. Opening the window or the Sound map (tray, shortcut, a
//! later launch) hands over to the full LinX: this process stops the engine
//! (applications return to their outputs for a moment) and becomes it.
use beacn_linx_core::link::{HiddenOutputs, LinkCommand, LinkWorker, Phase};
use std::time::Duration;

/// What comes after the headless loop.
enum Next {
    Quit,
    /// The full LinX, with its window (or the Sound map), raised with the
    /// launcher's xdg-activation token when there is one.
    Interface { map: bool, token: Option<String> },
}

pub fn run(instance: Option<crate::single_instance::Instance>, gui_args: Vec<String>) {
    crate::shutdown::install();
    let mut worker = LinkWorker::spawn();
    #[cfg(feature = "status-tray")]
    let tray = beacn_linx_core::tray::Tray::spawn();
    let hotkey = crate::hotkey::Hotkey::spawn();
    let hidden = HiddenOutputs::user().load();
    eprintln!("LinX running headless");
    let next = 'run: loop {
        std::thread::sleep(Duration::from_millis(100));
        if crate::shutdown::requested() {
            break Next::Quit;
        }
        let requests = instance.as_ref().map(|i| i.requests()).unwrap_or_default();
        for request in requests {
            match request {
                crate::single_instance::Request::Show(token) => break 'run Next::Interface { map: false, token },
                crate::single_instance::Request::Map(token) => break 'run Next::Interface { map: true, token },
                crate::single_instance::Request::Hidden => {}
            }
        }
        if hotkey.take() && worker.view().phase == Phase::Running {
            break Next::Interface { map: true, token: None };
        }
        #[cfg(feature = "status-tray")]
        {
            use beacn_linx_core::tray;
            let view = worker.view();
            tray.update(crate::link_ui::status_of(&view), view.phase == Phase::Running);
            tray.set_outputs(
                view.outputs
                    .iter()
                    .filter(|o| !hidden.contains(&o.name) || view.default_output.as_deref() == Some(o.name.as_str()))
                    .map(|o| (o.name.clone(), o.description.clone()))
                    .collect(),
                view.default_output.clone(),
            );
            let actions = tray.actions();
            if actions & tray::STOP != 0 {
                let _ = worker.send(LinkCommand::Pause);
            }
            if actions & tray::OUTPUT != 0 {
                if let Some(name) = tray.take_output() {
                    let _ = worker.send(LinkCommand::DefaultOutput(name));
                }
            }
            if actions & tray::QUIT != 0 {
                break Next::Quit;
            }
            if actions & tray::SHOW != 0 {
                break Next::Interface { map: false, token: tray.take_activation_token() };
            }
            if actions & tray::MAP != 0 {
                break Next::Interface { map: true, token: None };
            }
            // The tray host went away: nothing could show LinX any more.
            if tray.available() == Some(false) {
                break Next::Quit;
            }
        }
    };
    // Stop the engine first (channels released), then free the tray, the
    // shortcut and the single-instance lock for the full LinX.
    worker.stop();
    #[cfg(feature = "status-tray")]
    drop(tray);
    drop(hotkey);
    drop(instance);
    let Next::Interface { map, token } = next else {
        return;
    };
    use std::os::unix::process::CommandExt;
    let Ok(exe) = std::env::current_exe() else {
        std::process::exit(1)
    };
    let mut command = std::process::Command::new(exe);
    command.args(&gui_args);
    if map {
        command.arg("--map");
    }
    if let Some(token) = token {
        command.env("XDG_ACTIVATION_TOKEN", token);
    }
    let error = command.exec();
    eprintln!("LinX could not open its window: {error}");
    std::process::exit(1)
}
