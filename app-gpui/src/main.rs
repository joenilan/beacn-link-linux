use beacn_linx_core::worker::Connection;
use std::path::PathBuf;

fn main() {
    let mut args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--help"] {
        println!(
            "LinX\nDefault: read-only application discovery.\n--studio: native Studio session (studio-enabled build); select one stereo app, Connect, assign it to first Link In on the streaming PC. Pause/Quit releases owned output.\nOptional: --private-runtime PATH --core NAME\nOptional: --software-renderer (Linux; installed Mesa CPU Vulkan, this process only)"
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
    let connection = match args.as_slice() {
        [] => Connection::Desktop,
        #[cfg(all(feature = "studio", target_os = "linux"))]
        [flag] if flag == "--studio" => Connection::Studio,
        [a, r, b, c] if a == "--private-runtime" && b == "--core" => Connection::Private {
            runtime: PathBuf::from(r),
            core: c.clone(),
        },
        _ => {
            eprintln!("Unknown arguments. Use --help.");
            std::process::exit(2);
        }
    };
    if software == 1 {
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
    beacn_linx_gpui::ui::run(connection);
}
