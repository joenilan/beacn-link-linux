//! Explicit desktop routing qualification. Never a default apply entry.
#[cfg(all(feature = "native-desktop-target", target_os = "linux"))]
fn main() {
    if let Err(error) = run() {
        eprintln!("desktop target qualification: {error}");
        std::process::exit(2);
    }
}
#[cfg(all(feature = "native-desktop-target", target_os = "linux"))]
fn run() -> Result<(), String> {
    use beacn_link_protocol::{
        native_catalog::NativeCatalog, native_desktop_target::DesktopTarget, planner::ApplicationId,
    };
    use std::{
        io::Write,
        time::{Duration, Instant},
    };
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if !matches!(args.len(), 3 | 4 | 7) || args[0] != "--authorize-bounded-desktop-route" {
        return Err("requires explicit bounded desktop target authority, observed application ID and 1..10 seconds".into());
    }
    let seconds = args[2].parse::<u64>().map_err(|_| "Invalid duration.")?;
    if !(1..=10).contains(&seconds) {
        return Err("Desktop qualification duration outside 1..10 seconds.".into());
    }
    let mut catalog = NativeCatalog::desktop()?;
    let observed = catalog.observe(1)?;
    let mut owner = DesktopTarget::connect(&observed, &ApplicationId(args[1].clone()), true)?;
    if args.len() == 4 {
        if args[3] != "--test-fail-after-target-echo" {
            return Err("Unknown desktop qualification option.".into());
        }
        #[cfg(feature = "native-test-faults")]
        {
            owner.fail_after_target_echo_for_test()?;
            if owner.start().is_ok() {
                return Err("Desktop injected start unexpectedly succeeded.".into());
            }
            if owner.check().is_ok() || owner.start().is_ok() {
                return Err("Desktop failure did not latch.".into());
            }
            owner.stop()?;
            owner.stop()?;
            println!("DESKTOP FAILURE RESTORED latched=true cleanup_cached=true");
            return Ok(());
        }
        #[cfg(not(feature = "native-test-faults"))]
        return Err("Native test faults are disabled.".into());
    }
    println!("DESKTOP PREPARED");
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let active = owner.start();
    if let Err(error) = active {
        let cleanup = owner.stop();
        return Err(format!("{error}; cleanup={cleanup:?}"));
    }
    let sink = active.unwrap();
    println!("DESKTOP ACTIVE {}:{}", sink.id, sink.serial);
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    if args.len() == 7 {
        use beacn_link_protocol::executor::ObjectRef;
        use std::{io::Read, path::Path};
        if args[3] != "--authorize-studio-first-pair" {
            return Err("Desktop physical mode requires its first-pair authority.".into());
        }
        let mut expected = Vec::new();
        std::fs::File::open(&args[5])
            .map_err(|e| e.to_string())?
            .take((beacn_link_protocol::usb_descriptors::MAX_DESCRIPTOR_BYTES + 1) as u64)
            .read_to_end(&mut expected)
            .map_err(|e| e.to_string())?;
        if expected.len() > beacn_link_protocol::usb_descriptors::MAX_DESCRIPTOR_BYTES {
            return Err("Selected descriptor cache exceeds limit.".into());
        }
        let refs = args[6].split(':').collect::<Vec<_>>();
        if refs.len() != 2 {
            return Err("Expected exact Studio node ID:serial.".into());
        }
        let hardware = ObjectRef {
            id: refs[0].parse().map_err(|_| "Invalid Studio node ID.")?,
            serial: refs[1].parse().map_err(|_| "Invalid Studio serial.")?,
        };
        if let Err(error) = owner.attach_studio(Path::new(&args[4]), &expected, hardware, true) {
            let cleanup = owner.stop();
            return Err(format!("{error}; cleanup={cleanup:?}"));
        }
        println!("DESKTOP STUDIO ACTIVE first_pair=true rate=96000 native=true usb=false");
        std::io::stdout().flush().map_err(|e| e.to_string())?;
    }
    let end = Instant::now() + Duration::from_secs(seconds);
    let mut checks = 0;
    while Instant::now() < end {
        if let Err(error) = owner.check() {
            let cleanup = owner.stop();
            return Err(format!("{error}; cleanup={cleanup:?}"));
        }
        checks += 1;
        std::thread::sleep(Duration::from_millis(50));
    }
    owner.stop()?;
    if let Some((checks, progress)) = owner.physical_stats() {
        println!("DESKTOP STUDIO COMPLETE pcm_checks={checks} pointer_progress={progress}");
        if progress < 96000 * 3 {
            return Err("Desktop Studio clock progress insufficient.".into());
        }
    }
    println!("DESKTOP RESTORED checks={checks}");
    Ok(())
}
#[cfg(not(all(feature = "native-desktop-target", target_os = "linux")))]
fn main() {
    eprintln!("requires explicit Linux native-desktop-target feature");
    std::process::exit(2);
}
