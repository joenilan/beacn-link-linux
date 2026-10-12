//! Bounded VM diagnostic. Fresh inventories come from the supervising owner,
//! never a frozen TSV file; no routing or automatic recovery is performed.
#[cfg(all(feature = "native-usb", target_os = "linux"))]
fn main() -> std::process::ExitCode {
    use beacn_link_protocol::{
        lifecycle::InventoryTracker,
        native_usb, planner,
        session::{PollReply, Session},
        snapshot,
        usb_descriptors::MAX_DESCRIPTOR_BYTES,
        usb_handshake::{Publication, Startup, MAX_PUBLICATION_CYCLES},
    };
    use std::{
        io::{BufRead, Read, Write},
        path::Path,
        time::Duration,
    };
    let args: Vec<_> = std::env::args().skip(1).collect();
    let offline = args.len() == 2 && args[0] == "--check-periodic";
    let execute = || -> Result<(), String> {
        let observed_startup = args
            .first()
            .is_some_and(|arg| arg == "--authorize-periodic-startup");
        if !offline && (args.len() != 4 || (args[0] != "--authorize-periodic" && !observed_startup))
        {
            return Err("requires --authorize-periodic or --authorize-periodic-startup <sysfs-device> <expected-cache> <cycles>; or --check-periodic <cycles>".into());
        }
        let cycles = args
            .last()
            .ok_or("missing cycle budget")?
            .parse::<usize>()
            .map_err(|_| "invalid cycle budget")?;
        if !(1..=MAX_PUBLICATION_CYCLES).contains(&cycles) {
            return Err("cycle budget outside 1..=40".into());
        }
        let mut input = std::io::stdin().lock();
        let mut request = 0;
        let mut fresh = || -> Result<planner::Inventory, String> {
            println!("INVENTORY {request}");
            std::io::stdout()
                .flush()
                .map_err(|_| "inventory request failed")?;
            request += 1;
            let mut header = String::new();
            (&mut input)
                .take(65)
                .read_line(&mut header)
                .map_err(|_| "inventory header failed")?;
            if header.len() > 64 || !header.ends_with('\n') {
                return Err("inventory header refused".into());
            }
            let (id, length) = header
                .trim_end_matches('\n')
                .split_once('\t')
                .ok_or("inventory header refused")?;
            let id = id
                .parse::<u64>()
                .map_err(|_| "inventory generation refused")?;
            let length = length
                .parse::<usize>()
                .map_err(|_| "inventory length refused")?;
            if !(1..=3000).contains(&length) {
                return Err("inventory length refused".into());
            }
            let mut text = vec![0; length];
            input
                .read_exact(&mut text)
                .map_err(|_| "inventory body failed")?;
            let text = std::str::from_utf8(&text).map_err(|_| "inventory encoding refused")?;
            snapshot::inventory_from_tsv(id, text)
        };
        let initial = fresh()?;
        let publication = Publication::bounded(&initial, cycles).map_err(|e| e.to_string())?;
        let records = publication.records().to_vec();
        let mut tracker = InventoryTracker::default();
        tracker.advance(&initial).map_err(|e| e.to_string())?;
        let check_fresh =
            |next: &planner::Inventory, tracker: &mut InventoryTracker| -> Result<(), String> {
                if next.applications != initial.applications {
                    return Err("pinned source changed".into());
                }
                tracker.advance(next).map_err(|e| e.to_string())?;
                Ok(())
            };
        if offline {
            for _ in 0..cycles {
                check_fresh(&fresh()?, &mut tracker)?;
                std::thread::sleep(Duration::from_millis(500));
                check_fresh(&fresh()?, &mut tracker)?;
            }
            println!("fresh inventory cycles={cycles}; no device access or routes applied");
            return Ok(());
        }
        let mut expected = Vec::new();
        std::fs::File::open(&args[2])
            .and_then(|f| {
                f.take((MAX_DESCRIPTOR_BYTES + 1) as u64)
                    .read_to_end(&mut expected)
            })
            .map_err(|_| "expected cache refused")?;
        let startup = if observed_startup {
            Startup::ObservedWindows
        } else {
            Startup::SelectedAlternate
        };
        println!("startup policy: {startup:?}; bulk deadlines unchanged");
        let transport = native_usb::open_publication_with_startup(
            Path::new(&args[1]),
            &expected,
            publication,
            true,
            startup,
        )
        .map_err(|e| e.to_string())?;
        let mut session =
            Session::new(transport, Duration::from_millis(500)).expect("fixed deadline");
        let result = (|| -> Result<(), String> {
            session.initialize().map_err(|e| e.to_string())?;
            println!("initialized; bounded publication active; names and metadata redacted");
            for cycle in 1..=cycles {
                let before = fresh()?;
                check_fresh(&before, &mut tracker)?;
                session
                    .refresh_publication(&before)
                    .map_err(|e| e.to_string())?;
                session.publish(&records).map_err(|e| e.to_string())?;
                // Controlled diagnostic timing, not a proven firmware requirement.
                std::thread::sleep(Duration::from_millis(500));
                let reply = session.poll().map_err(|e| e.to_string())?;
                let current = fresh()?;
                check_fresh(&current, &mut tracker)?;
                match reply {
                    PollReply::NoPayload { ignored_tail } => {
                        println!("cycle={cycle} no_payload ignored_tail={ignored_tail}")
                    }
                    PollReply::Records {
                        records: replies,
                        ignored_tail,
                    } => {
                        let plan = planner::plan(&current, &replies).map_err(|e| e.to_string())?;
                        tracker
                            .require_current_snapshot(&plan)
                            .map_err(|e| e.to_string())?;
                        let assignments: Vec<_> = replies
                            .iter()
                            .filter(|r| r.name == records[0].name)
                            .map(|r| r.assignment.byte())
                            .collect();
                        let known = assignments.iter().any(|a| (1..=4).contains(a));
                        println!("cycle={cycle} records={} bound_assignments={assignments:?} proposed_routes={} ignored_tail={ignored_tail}; no routes applied", replies.len(), plan.proposed_routes().count());
                        if known {
                            // Do not overwrite an observed desired assignment by
                            // continuing to advertise the unapplied local zero.
                            println!("bound assignment received; publication stopped; observing for 5 seconds");
                            std::io::stdout().flush().map_err(|_| "output failed")?;
                            std::thread::sleep(Duration::from_secs(5));
                            check_fresh(&fresh()?, &mut tracker)?;
                            break;
                        }
                    }
                }
            }
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
