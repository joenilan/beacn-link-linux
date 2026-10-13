//! Bounded VM diagnostic. Fresh inventories come from the supervising owner,
//! never a frozen TSV file; no routing or automatic recovery is performed.
#[cfg(all(feature = "native-usb", target_os = "linux"))]
fn main() -> std::process::ExitCode {
    use beacn_link_protocol::{
        lifecycle::InventoryTracker,
        native_usb, planner,
        session::{PollPolicy, PollReply, ReceivedNamePolicy, Session},
        snapshot,
        usb_descriptors::MAX_DESCRIPTOR_BYTES,
        usb_handshake::{
            Publication, Startup, DIAGNOSTIC_INITIALIZATION_TIMEOUT, MAX_PUBLICATION_CYCLES,
        },
    };
    use std::{
        io::{BufRead, Read, Write},
        path::Path,
        time::Duration,
    };
    let args: Vec<_> = std::env::args().skip(1).collect();
    let offline = args.len() == 2 && args[0] == "--check-periodic";
    let execute = || -> Result<(), String> {
        let windows_names = args
            .first()
            .is_some_and(|arg| arg == "--authorize-periodic-c-string-names");
        let name_policy = if windows_names {
            ReceivedNamePolicy::WindowsCString
        } else {
            ReceivedNamePolicy::ExactBytes
        };
        let initialization_diagnostic = windows_names
            || args
                .first()
                .is_some_and(|arg| arg == "--authorize-periodic-initialization-diagnostic");
        let observe_markers = initialization_diagnostic
            || args
                .first()
                .is_some_and(|arg| arg == "--authorize-periodic-observed-polls");
        let observed_startup = observe_markers
            || args
                .first()
                .is_some_and(|arg| arg == "--authorize-periodic-startup");
        if !offline && (args.len() != 4 || (args[0] != "--authorize-periodic" && !observed_startup))
        {
            return Err("requires an explicit --authorize-periodic[-startup|-observed-polls|-initialization-diagnostic|-c-string-names] <sysfs-device> <expected-cache> <cycles>; or --check-periodic <cycles>".into());
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
        let poll_policy = if observe_markers {
            PollPolicy::ReportRejectedMarker
        } else {
            PollPolicy::Strict
        };
        let publication = Publication::bounded_with_poll_policy(&initial, cycles, poll_policy)
            .map_err(|e| e.to_string())?;
        let publication = if initialization_diagnostic {
            publication.with_initialization_diagnostic()
        } else {
            publication
        };
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
        let initial_read_ms = if initialization_diagnostic { 5000 } else { 500 };
        println!(
            "startup policy: {startup:?}; poll policy: {poll_policy:?}; received_name_policy: {name_policy:?}; initial_read_ms={initial_read_ms}; writes_and_later_reads_ms=500"
        );
        let transport = native_usb::open_publication_with_startup(
            Path::new(&args[1]),
            &expected,
            publication,
            true,
            startup,
        )
        .map_err(|e| e.to_string())?;
        let mut session =
            Session::new_with_poll_policy(transport, Duration::from_millis(500), poll_policy)
                .expect("fixed deadline");
        let result = (|| -> Result<(), String> {
            if initialization_diagnostic {
                session.initialize_with_read_timeout(DIAGNOSTIC_INITIALIZATION_TIMEOUT)
            } else {
                session.initialize()
            }
            .map_err(|e| e.to_string())?;
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
                    PollReply::RejectedMarker { ignored_tail } => println!("cycle={cycle} rejected_marker ignored_tail={ignored_tail}; no assignments or counter update inferred"),
                    PollReply::NoPayload { ignored_tail } => {
                        println!("cycle={cycle} no_payload ignored_tail={ignored_tail}")
                    }
                    PollReply::Records {
                        records: replies,
                        ignored_tail,
                    } => {
                        let planning_records: Vec<_> = replies
                            .iter()
                            .map(|record| name_policy.for_planning(record))
                            .collect();
                        let plan = planner::plan(&current, &planning_records).map_err(|e| e.to_string())?;
                        tracker
                            .require_current_snapshot(&plan)
                            .map_err(|e| e.to_string())?;
                        let assignments: Vec<_> = planning_records
                            .iter()
                            .filter(|r| r.name == records[0].name)
                            .map(|r| r.assignment.byte())
                            .collect();
                        let known = assignments.iter().any(|a| (1..=4).contains(a));
                        println!("cycle={cycle} records={} bound_assignments={assignments:?} proposed_routes={} ignored_tail={ignored_tail}; no routes applied", replies.len(), plan.proposed_routes().count());
                        // Bound diagnostic output independently of the peer's
                        // record count. Raw bytes remain available for diagnostics
                        // even when explicit C-string planning is selected.
                        for (index, record) in replies.iter().take(4).enumerate() {
                            let nul_index = record.name.iter().position(|b| *b == 0);
                            let c_string_name = &record.name[..nul_index.unwrap_or(record.name.len())];
                            let tail_bytes = nul_index.map_or(0, |i| record.name.len() - i - 1);
                            println!(
                                "cycle={cycle} record_index={index} name_bytes={} assignment={} bound_name_match={} nul_bytes={} printable_ascii={} first_nul={nul_index:?} tail_bytes={tail_bytes} bound_c_string_match={}; name redacted",
                                record.name.len(),
                                record.assignment.byte(),
                                record.name == records[0].name,
                                record.name.iter().filter(|b| **b == 0).count(),
                                record.name.iter().all(|b| (0x20..=0x7e).contains(b)),
                                c_string_name == records[0].name,
                            );
                        }
                        if replies.len() > 4 {
                            println!("cycle={cycle} additional_record_diagnostics_omitted={}", replies.len() - 4);
                        }
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
