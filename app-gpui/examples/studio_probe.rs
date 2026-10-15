//! Explicit native hardware GUI qualification, never a default production entry.
#[cfg(all(feature = "studio-session-test", target_os = "linux"))]
fn main() {
    eprintln!("STUDIO GUI entry");
    println!("STUDIO OWNER {}", std::process::id());
    use beacn_link_protocol::executor::{Destination, ObjectRef};
    use beacn_linx_core::{private_session, studio_session, worker::Connection};
    use std::{io::Read, path::PathBuf};
    assert_eq!(std::env::var("LINX_STUDIO_GUI_TEST").as_deref(), Ok("1"));
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 16);
    let playback = match args[0].as_str() {
        "--authorize-bounded-studio-gui" => studio_session::Playback::Supervisor,
        #[cfg(feature = "native-playback-test")]
        "--authorize-bounded-native-studio-gui" => studio_session::Playback::NativeStudio,
        #[cfg(feature = "native-playback-test")]
        "--authorize-bounded-native-studio-recovery-gui" => studio_session::Playback::NativeStudio,
        _ => panic!("requires explicit bounded Studio backend authority"),
    };
    let recovery = args[0] == "--authorize-bounded-native-studio-recovery-gui";
    if recovery {
        assert_eq!(args[5], "1");
        assert_eq!(args[6], "40");
    }
    let root = PathBuf::from(&args[1]);
    std::fs::create_dir(&root).expect("exclusive GUI evidence");
    let mut expected = Vec::new();
    std::fs::File::open(&args[3])
        .unwrap()
        .take((beacn_link_protocol::usb_descriptors::MAX_DESCRIPTOR_BYTES + 1) as u64)
        .read_to_end(&mut expected)
        .unwrap();
    assert!(expected.len() <= beacn_link_protocol::usb_descriptors::MAX_DESCRIPTOR_BYTES);
    let destinations = args[12..]
        .iter()
        .map(|arg| {
            let p: Vec<_> = arg.split(':').collect();
            assert_eq!(p.len(), 3);
            Destination {
                assignment: p[0].parse().unwrap(),
                node: ObjectRef {
                    id: p[1].parse().unwrap(),
                    serial: p[2].parse().unwrap(),
                },
            }
        })
        .collect();
    let config = studio_session::Configuration {
        sysfs: PathBuf::from(&args[2]),
        expected,
        pair: args[5].parse().unwrap(),
        physical: true,
        playback,
        routing: private_session::Configuration {
            runtime: PathBuf::from(&args[7]),
            core: args[8].clone(),
            cookie: args[9].parse().unwrap(),
            server: args[10].parse().unwrap(),
            owner: args[11].clone(),
            destinations,
            cycles: args[6].parse().unwrap(),
        },
    };
    unsafe {
        std::env::set_var("LINX_PRIVATE_GUI_EVIDENCE", &root);
    }
    eprintln!("STUDIO GUI configuration accepted");
    beacn_linx_gpui::ui::run_qualification(
        Connection::StudioSession(config),
        beacn_linx_gpui::qualification::Configuration {
            first: args[4].clone(),
            second: String::new(),
            root,
            studio: true,
            managed: false,
            native_playback: false,
            console: false,
            desktop: false,
            studio_recovery: recovery,
        },
    );
}
#[cfg(not(all(feature = "studio-session-test", target_os = "linux")))]
fn main() {
    eprintln!("requires explicit Linux studio-session-test feature");
    std::process::exit(2);
}
