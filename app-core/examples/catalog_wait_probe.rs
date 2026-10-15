//! Read-only discovery recovery probe for an owner-provisioned disposable core.
#[cfg(all(feature = "native-catalog", target_os = "linux"))]
fn main() {
    use beacn_linx_core::worker::{Connection, Worker};
    use std::{
        io::{self, Write},
        path::PathBuf,
        time::{Duration, Instant},
    };
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 2);
    let mut worker = Worker::spawn(Connection::Private {
        runtime: PathBuf::from(&args[0]),
        core: args[1].clone(),
    });
    let mut previous_core = None;
    for (connected, marker) in [
        (false, "waiting"),
        (true, "online"),
        (false, "lost"),
        (true, "reconnected"),
    ] {
        let until = Instant::now() + Duration::from_secs(12);
        loop {
            let view = worker.snapshot();
            if !view.busy && view.connected == connected {
                assert!(view.selected.is_empty());
                if marker == "online" {
                    previous_core = view.catalog.as_ref().map(|c| c.core.clone());
                }
                if marker == "reconnected" {
                    assert_ne!(view.catalog.as_ref().map(|c| c.core.clone()), previous_core);
                    assert!(!view.discovery_error);
                }
                println!("{marker}");
                io::stdout().flush().unwrap();
                break;
            }
            assert!(Instant::now() < until, "timed out waiting for {marker}");
            std::thread::sleep(Duration::from_millis(50));
        }
        // The external owner changes only its private server between milestones.
        if marker != "reconnected" {
            let mut line = String::new();
            io::stdin().read_line(&mut line).unwrap();
            assert_eq!(line.trim(), "next");
        }
    }
    worker.stop();
    println!("joined");
}
#[cfg(not(all(feature = "native-catalog", target_os = "linux")))]
fn main() {
    panic!("requires native-catalog on Linux");
}
