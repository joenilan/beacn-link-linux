#[cfg(all(feature = "native-usb", target_os = "linux"))]
fn main() -> std::process::ExitCode {
    use beacn_link_protocol::{
        lifecycle::InventoryTracker,
        native_usb, planner,
        session::{PollReply, Session},
        snapshot,
        usb_descriptors::MAX_DESCRIPTOR_BYTES,
        usb_handshake::Publication,
    };
    use std::{io::Read, path::Path, time::Duration};
    let args: Vec<_> = std::env::args().skip(1).collect();
    let offline = args.len() == 3 && args[0] == "--check-inventory";
    if !offline && (args.len() != 5 || args[0] != "--authorize-publication") {
        eprintln!("requires Linux native-usb and explicit --authorize-publication <sysfs-device> <expected-cache> <inventory.tsv> <snapshot-id>; or --check-inventory <inventory.tsv> <snapshot-id>");
        return std::process::ExitCode::from(2);
    }
    let execute = || -> Result<(), String> {
        let (file, id) = if offline {
            (&args[1], &args[2])
        } else {
            (&args[3], &args[4])
        };
        let id = id.parse::<u64>().map_err(|_| "snapshot-id refused")?;
        let mut text = String::new();
        std::fs::File::open(file)
            .and_then(|f| {
                f.take((snapshot::MAX_TSV_BYTES + 1) as u64)
                    .read_to_string(&mut text)
            })
            .map_err(|_| "inventory input refused")?;
        let inventory = snapshot::inventory_from_tsv(id, &text)?;
        let publication = Publication::from_inventory(&inventory).map_err(|e| e.to_string())?;
        let records = publication.records().to_vec();
        let mut tracker = InventoryTracker::default();
        tracker.advance(&inventory).map_err(|e| e.to_string())?;
        if offline {
            println!("one bound live application accepted; initial assignment zero is opaque; no device access");
            return Ok(());
        }
        let mut expected = Vec::new();
        std::fs::File::open(&args[2])
            .and_then(|f| {
                f.take((MAX_DESCRIPTOR_BYTES + 1) as u64)
                    .read_to_end(&mut expected)
            })
            .map_err(|_| "expected-cache input refused")?;
        let transport =
            native_usb::open_publication(Path::new(&args[1]), &expected, publication, true)
                .map_err(|e| e.to_string())?;
        let mut session =
            Session::new(transport, Duration::from_millis(500)).expect("fixed deadline");
        let result = (|| -> Result<(), String> {
            session.initialize().map_err(|e| e.to_string())?;
            println!("A0/A1/A2 completed; metadata redacted");
            session.publish(&records).map_err(|e| e.to_string())?;
            println!(
                "one bound application list transferred completely; remote visibility unverified"
            );
            match session.poll().map_err(|e| e.to_string())? {
                PollReply::NoPayload { ignored_tail } => println!(
                    "poll: no payload; ignored_tail={ignored_tail}; zero semantics remain opaque"
                ),
                PollReply::Records {
                    records: replies,
                    ignored_tail,
                } => {
                    let plan = planner::plan(&inventory, &replies).map_err(|e| e.to_string())?;
                    tracker
                        .require_current_snapshot(&plan)
                        .map_err(|e| e.to_string())?;
                    let assignments: Vec<_> = replies
                        .iter()
                        .filter(|r| r.name == records[0].name)
                        .map(|r| r.assignment.byte())
                        .collect();
                    println!("poll: records={} bound_assignments={assignments:?} ignored_tail={ignored_tail} proposed_routes={}; names redacted; no routes applied", replies.len(), plan.proposed_routes().count());
                }
            }
            // Keep the observed source/claimed interface alive briefly for the
            // owner's remote UI check. No heartbeat, resend or routing occurs.
            println!("exchange complete; retaining claim for a 10-second visibility observation");
            std::thread::sleep(Duration::from_secs(10));
            Ok(())
        })();
        let cleanup = session.into_transport().close();
        println!("cleanup: {cleanup:?}");
        result?;
        if !cleanup.baseline_restored {
            return Err("device baseline restoration failed".into());
        }
        Ok(())
    };
    match execute() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::from(2)
        }
    }
}
#[cfg(not(all(feature = "native-usb", target_os = "linux")))]
fn main() {
    eprintln!("requires Linux and explicit native-usb feature; no device access requested");
    std::process::exit(2);
}
