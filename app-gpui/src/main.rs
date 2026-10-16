use beacn_linx_core::worker::Connection;
use std::path::PathBuf;

fn main() {
    let mut args: Vec<_> = std::env::args().skip(1).collect();
    #[cfg(all(feature = "native-control", target_os = "linux"))]
    if let Some(index) = args.iter().position(|a| a == "--control-socket") {
        if index + 1 >= args.len() {
            eprintln!("--control-socket requires a private Unix socket path");
            std::process::exit(2);
        }
        let path = PathBuf::from(args.remove(index + 1));
        args.remove(index);
        if let Err(error) = beacn_linx_gpui::control::configure(path) {
            eprintln!("Native control: {error}");
            std::process::exit(2);
        }
    }
    #[cfg(all(feature = "ui-qualification", target_os = "linux"))]
    if let Some(index) = args.iter().position(|a| a == "--capture-frame") {
        if index + 1 >= args.len() {
            eprintln!("--capture-frame requires an exclusive output file");
            std::process::exit(2);
        }
        let path = PathBuf::from(args.remove(index + 1));
        args.remove(index);
        beacn_linx_gpui::diagnostics::configure(path);
    }
    if args == ["--help"] {
        println!(
            "LinX\nDefault: read-only application discovery.\n--studio: Link Console (studio-enabled build); four Link Out channels and four Link In returns on BEACN Studio, any application on any channel, BEACN App publication. One LinX per session: launching it again shows the running window. Closing the window keeps LinX in the tray.\n--studio-legacy: earlier single-application desktop session.\n--start-hidden: with --studio, start in the tray (minimized without one).\nOptional: --private-runtime PATH --core NAME\nOptional: --software-renderer (Linux; installed Mesa CPU Vulkan, this process only)\nOptional: --control-socket PATH (private same-user diagnostics and screenshots; off unless given)"
        );
        return;
    }
    let software = args
        .iter()
        .filter(|a| a.as_str() == "--software-renderer")
        .count();
    if software == 1 {
        args.retain(|a| a != "--software-renderer");
    }
    // The Link Console: persistent Link channels (default Studio mode).
    #[cfg(all(feature = "link", target_os = "linux"))]
    {
        let hidden = args.iter().any(|a| a == "--start-hidden");
        let rest: Vec<_> = args.iter().filter(|a| *a != "--start-hidden").collect();
        if rest.len() == 1 && rest[0] == "--studio" {
            use beacn_linx_gpui::single_instance::{claim, Claim};
            // Before anything touches PipeWire, the Studio or the control
            // socket: a second launch only asks the running LinX to show.
            let instance = match claim(hidden) {
                Ok(Claim::Primary(instance)) => Some(instance),
                Ok(Claim::Running(Ok(()))) => {
                    eprintln!(
                        "LinX is already running{}",
                        if hidden { "." } else { "; showing it." }
                    );
                    return;
                }
                Ok(Claim::Running(Err(error))) => {
                    eprintln!("LinX is already running but did not answer: {error}");
                    std::process::exit(1);
                }
                Err(error) => {
                    eprintln!("LinX single-instance check unavailable: {error}");
                    None
                }
            };
            apply_software_renderer(software == 1);
            // Arguments to reuse for launch at login.
            let mut login = vec!["--studio".to_string()];
            if software == 1 {
                login.push("--software-renderer".into());
            }
            beacn_linx_gpui::link_ui::run(hidden, login, instance);
            return;
        }
    }
    let connection = match args.as_slice() {
        [] => Connection::Desktop,
        #[cfg(all(feature = "studio", target_os = "linux"))]
        [flag] if flag == "--studio" || flag == "--studio-legacy" => Connection::Studio,
        [a, r, b, c] if a == "--private-runtime" && b == "--core" => Connection::Private {
            runtime: PathBuf::from(r),
            core: c.clone(),
        },
        _ => {
            eprintln!("Unknown arguments. Use --help.");
            std::process::exit(2);
        }
    };
    apply_software_renderer(software == 1);
    beacn_linx_gpui::ui::run(connection);
}

fn apply_software_renderer(enabled: bool) {
    if enabled {
        #[cfg(target_os = "linux")]
        {
            let manifest = [
                "/usr/share/vulkan/icd.d/lvp_icd.json",
                "/usr/share/vulkan/icd.d/lvp_icd.x86_64.json",
            ]
            .into_iter()
            .find(|path| std::path::Path::new(path).is_file());
            let Some(manifest) = manifest else {
                eprintln!("Installed Mesa CPU renderer is unavailable.");
                std::process::exit(2);
            };
            // Select a system-installed renderer before GPUI creates threads.
            // No driver installation, file changes or global environment edits.
            unsafe {
                std::env::set_var("VK_DRIVER_FILES", manifest);
                std::env::set_var("VK_ICD_FILENAMES", manifest);
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            eprintln!("Software renderer selection is currently Linux-only.");
            std::process::exit(2);
        }
    }
}
