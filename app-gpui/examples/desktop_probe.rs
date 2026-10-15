//! Explicit owner-attended desktop GUI check. Never an ordinary app entry.
#[cfg(all(feature = "desktop-session-test", target_os = "linux"))]
fn main() {
    use beacn_link_protocol::executor::ObjectRef;
    use beacn_linx_core::{desktop_session, worker::Connection};
    use std::{io::Read, path::PathBuf};
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    assert_eq!(args.len(), 6);
    assert_eq!(args[0], "--authorize-bounded-desktop-studio-gui");
    let root = PathBuf::from(&args[1]);
    std::fs::create_dir(&root).expect("exclusive GUI evidence");
    let mut expected = Vec::new();
    std::fs::File::open(&args[3])
        .unwrap()
        .take((beacn_link_protocol::usb_descriptors::MAX_DESCRIPTOR_BYTES + 1) as u64)
        .read_to_end(&mut expected)
        .unwrap();
    beacn_link_protocol::usb_handshake::check_descriptors(&expected).unwrap();
    let refs = args[5].split(':').collect::<Vec<_>>();
    assert_eq!(refs.len(), 2);
    let hardware = ObjectRef {
        id: refs[0].parse().unwrap(),
        serial: refs[1].parse().unwrap(),
    };
    unsafe {
        std::env::set_var("LINX_PRIVATE_GUI_EVIDENCE", &root);
    }
    beacn_linx_gpui::ui::run_qualification(
        Connection::DesktopStudioSession(desktop_session::Configuration {
            sysfs: PathBuf::from(&args[2]),
            expected,
            hardware,
            cycles: 40,
        }),
        beacn_linx_gpui::qualification::Configuration {
            first: args[4].clone(),
            second: String::new(),
            root,
            studio: false,
            managed: false,
            native_playback: false,
            studio_recovery: false,
            desktop: true,
        },
    );
}
#[cfg(not(all(feature = "desktop-session-test", target_os = "linux")))]
fn main() {
    eprintln!("requires explicit Linux desktop-session-test feature");
    std::process::exit(2);
}
