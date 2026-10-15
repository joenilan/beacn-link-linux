//! Owned-window synthetic native playback; no USB and no physical PCM target.
#[cfg(all(feature = "native-playback-test", target_os = "linux"))]
fn main() {
    use beacn_link_protocol::executor::{Destination, ObjectRef};
    use beacn_linx_core::{private_session, studio_session, worker::Connection};
    use std::path::PathBuf;
    assert_eq!(std::env::var("LINX_PRIVATE_GUI_TEST").as_deref(), Ok("1"));
    assert_eq!(
        std::env::var("LINX_NATIVE_PLAYBACK_GUI_TEST").as_deref(),
        Ok("1")
    );
    let a: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(a.len(), 14);
    let root = PathBuf::from(&a[4]);
    std::fs::create_dir(&root).expect("exclusive evidence directory");
    unsafe {
        std::env::set_var("LINX_PRIVATE_GUI_EVIDENCE", &root);
    }
    let destinations = a[10..]
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
        routing: private_session::Configuration {
            runtime: PathBuf::from(&a[5]),
            core: a[6].clone(),
            cookie: a[7].parse().unwrap(),
            server: a[8].parse().unwrap(),
            owner: a[9].clone(),
            destinations,
            cycles: 40,
        },
        sysfs: PathBuf::new(),
        expected: vec![],
        pair: 1,
        physical: true,
        playback: studio_session::Playback::NativeSynthetic,
    };
    beacn_linx_gpui::ui::run_qualification(
        Connection::NativePlaybackSession(config),
        beacn_linx_gpui::qualification::Configuration {
            first: a[2].clone(),
            second: a[3].clone(),
            root,
            studio: false,
            managed: false,
            native_playback: true,
        },
    );
}
#[cfg(not(all(feature = "native-playback-test", target_os = "linux")))]
fn main() {
    eprintln!("requires explicit Linux native-playback-test feature");
    std::process::exit(2);
}
