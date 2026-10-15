//! Explicit bounded desktop + Studio publication qualification. Not a default GUI.
#[cfg(all(feature = "native-desktop-target", target_os = "linux"))]
fn main() {
    if let Err(e) = run() {
        eprintln!("desktop session qualification: {e}");
        std::process::exit(2);
    }
}
#[cfg(all(feature = "native-desktop-target", target_os = "linux"))]
fn run() -> Result<(), String> {
    use beacn_link_protocol::{
        catalog::ApplicationService,
        desktop_link_session::{DesktopLinkSession, StudioDesktopLease},
        executor::ObjectRef,
        link_session::{OpenFailure, Options, Phase},
        native_catalog::NativeCatalog,
        native_usb::{self, OpenError},
        planner::ApplicationId,
        session::{PollPolicy, ReceivedNamePolicy},
        usb_handshake::{self, Publication},
    };
    use std::{
        io::{Read, Write},
        path::Path,
    };
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 7 || args[0] != "--authorize-bounded-desktop-studio" {
        return Err("requires selected sysfs/cache, current app ID, explicit wire name, hardware ID:serial and 1..40 cycles".into());
    }
    let cycles = args[6]
        .parse::<usize>()
        .map_err(|_| "Invalid cycle budget.")?;
    if !(1..=40).contains(&cycles) {
        return Err("Cycle budget outside 1..40.".into());
    }
    let mut expected = Vec::new();
    std::fs::File::open(&args[2])
        .map_err(|e| e.to_string())?
        .take((beacn_link_protocol::usb_descriptors::MAX_DESCRIPTOR_BYTES + 1) as u64)
        .read_to_end(&mut expected)
        .map_err(|e| e.to_string())?;
    usb_handshake::check_descriptors(&expected).map_err(|e| e.to_string())?;
    let refs = args[5].split(':').collect::<Vec<_>>();
    if refs.len() != 2 {
        return Err("Hardware ID:serial required.".into());
    }
    let hardware = ObjectRef {
        id: refs[0].parse().map_err(|_| "Invalid node ID.")?,
        serial: refs[1].parse().map_err(|_| "Invalid node serial.")?,
    };
    let options = Options {
        cycles,
        poll_policy: PollPolicy::ReportRejectedMarker,
        names: ReceivedNamePolicy::WindowsCString,
        initialization_timeout: usb_handshake::DIAGNOSTIC_INITIALIZATION_TIMEOUT,
    };
    let mut catalog = NativeCatalog::desktop()?;
    let mut generation = 1;
    let mut service = ApplicationService::default();
    service.update(catalog.observe(generation)?)?;
    service.select(&ApplicationId(args[3].clone()), &args[4])?;
    let mut owner = DesktopLinkSession::default();
    owner.start(
        &service,
        options,
        || StudioDesktopLease::connect(&service, Path::new(&args[1]), &expected, hardware, true),
        |routes, inventory, options| {
            let p = Publication::bounded_desktop(inventory, options.cycles, options.poll_policy)
                .map_err(|e| OpenFailure {
                    message: e.to_string(),
                    cleanup: Ok(()),
                })?
                .with_initialization_diagnostic();
            native_usb::open_publication_with_owned_playback(
                Path::new(&args[1]),
                &expected,
                p,
                true,
                routes.server_pid(),
            )
            .map_err(|e| {
                let cleanup = match &e {
                    OpenError::Setup(f) if !f.cleanup.baseline_restored => {
                        Err("USB setup cleanup unverified.".into())
                    }
                    _ => Ok(()),
                };
                OpenFailure {
                    message: e.to_string(),
                    cleanup,
                }
            })
        },
    )?;
    println!("DESKTOP SESSION READY original_output=true cycles={cycles}");
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let mut physical = None;
    let mut assigned = false;
    let result = (|| {
        while owner.status().phase == Phase::Running {
            generation += 1;
            service.update(catalog.observe(generation)?)?;
            owner.tick(&service)?;
            if let Some(routes) = owner.routes() {
                if let Some(stats) = routes.physical_stats() {
                    physical = Some(stats);
                    assigned = true;
                }
            }
            let s = owner.status();
            println!(
                "DESKTOP SESSION cycle={} assignment={:?} reply={:?}",
                s.completed_cycles, s.assignment, s.last_reply
            );
            std::io::stdout().flush().map_err(|e| e.to_string())?;
        }
        Ok::<(), String>(())
    })();
    let cleanup = owner.stop();
    println!(
        "DESKTOP SESSION STOPPED status={:?} physical_stats={physical:?}",
        owner.status()
    );
    cleanup?;
    result?;
    if !assigned || physical.is_none_or(|(_, frames)| frames < 96000 * 3) {
        return Err("No sufficient assigned desktop PCM progress; cleanup finished.".into());
    }
    println!("DESKTOP SESSION COMPLETE native=true publication=true restored=true");
    Ok(())
}
#[cfg(not(all(feature = "native-desktop-target", target_os = "linux")))]
fn main() {
    eprintln!("requires explicit Linux native-desktop-target feature");
    std::process::exit(2);
}
