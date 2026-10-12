#[cfg(all(feature = "native-usb", target_os = "linux"))]
fn main() -> std::process::ExitCode {
    use beacn_link_protocol::{
        native_usb, session::Session, usb_descriptors::MAX_DESCRIPTOR_BYTES,
    };
    use std::{io::Read, path::Path, time::Duration};
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--library-version"] {
        println!(
            "libusb {:?}; metadata only, no initialization or device access",
            native_usb::library_version()
        );
        return std::process::ExitCode::SUCCESS;
    }
    let check_only = args.len() == 2 && args[0] == "--check-cache";
    if !check_only && (args.len() != 3 || args[0] != "--authorize-handshake") {
        eprintln!("requires explicit owner-authorized --authorize-handshake <sysfs-device> <private-expected-cache>; no device access requested");
        return std::process::ExitCode::from(2);
    }
    let mut expected = Vec::new();
    let loaded =
        std::fs::File::open(if check_only { &args[1] } else { &args[2] }).and_then(|file| {
            file.take((MAX_DESCRIPTOR_BYTES + 1) as u64)
                .read_to_end(&mut expected)
        });
    if loaded.is_err() || expected.len() > MAX_DESCRIPTOR_BYTES {
        eprintln!("expected-cache input refused");
        return std::process::ExitCode::from(2);
    }
    if check_only {
        match beacn_link_protocol::usb_handshake::check_descriptors(&expected) {
            Ok(()) => {
                println!(
                    "confirmed descriptor profile matches; no initialization or device access"
                );
                return std::process::ExitCode::SUCCESS;
            }
            Err(error) => {
                eprintln!("{error}");
                return std::process::ExitCode::from(2);
            }
        }
    }
    let transport = match native_usb::open_handshake(Path::new(&args[1]), &expected, true) {
        Ok(t) => t,
        Err(error) => {
            eprintln!("{error}");
            return std::process::ExitCode::from(2);
        }
    };
    let mut session =
        Session::new(transport, Duration::from_millis(500)).expect("fixed nonzero deadline");
    let result = session.initialize();
    match &result {
        Ok(reply) => println!("A0/A1/A2 completed: {reply:?}; remote visibility untested"),
        Err(error) => eprintln!("{error}"),
    }
    // Explicit cleanup before reporting success; never publish or poll.
    let cleanup = session.into_transport().close();
    println!("cleanup: {cleanup:?}");
    if result.is_ok() && cleanup.baseline_restored {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::from(2)
    }
}
#[cfg(not(all(feature = "native-usb", target_os = "linux")))]
fn main() {
    eprintln!("requires Linux and explicit native-usb feature; no device access requested");
    std::process::exit(2)
}
