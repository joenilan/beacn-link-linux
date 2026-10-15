#[cfg(all(feature = "ui-qualification", target_os = "linux"))]
fn main() {
    use beacn_link_protocol::executor::{Destination, ObjectRef};
    use beacn_linx_core::{private_session::Configuration, worker::Connection};
    use std::path::PathBuf;
    assert_eq!(
        std::env::var("LINX_PRIVATE_GUI_TEST").as_deref(),
        Ok("1"),
        "requires private display owner"
    );
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 14);
    let root = PathBuf::from(&args[4]);
    std::fs::create_dir(&root).expect("exclusive evidence directory");
    let destinations = args[10..]
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
    // A qualification-only process variable, set before its window/threads exist.
    unsafe {
        std::env::set_var("LINX_PRIVATE_GUI_EVIDENCE", &root);
    }
    let managed = std::env::var("LINX_MANAGED_PRIVATE_GUI_TEST").as_deref() == Ok("1");
    let config = Configuration {
        runtime: PathBuf::from(&args[5]),
        core: args[6].clone(),
        cookie: args[7].parse().unwrap(),
        server: args[8].parse().unwrap(),
        owner: args[9].clone(),
        destinations,
        cycles: 30,
    };
    beacn_linx_gpui::ui::run_qualification(
        if managed {
            Connection::ManagedPrivateSession(config)
        } else {
            Connection::PrivateSession(config)
        },
        beacn_linx_gpui::qualification::Configuration {
            first: args[2].clone(),
            second: args[3].clone(),
            root,
            studio: false,
            managed,
        },
    );
}
#[cfg(not(all(feature = "ui-qualification", target_os = "linux")))]
fn main() {
    eprintln!("requires explicit Linux ui-qualification feature");
    std::process::exit(2);
}
