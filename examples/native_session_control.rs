//! Bounded stdin qualification of Rust controls; private core only, no USB.
#[cfg(all(feature = "native-pipewire", target_os = "linux"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use beacn_link_protocol::{
        catalog::ApplicationService,
        executor::{Destination, ObjectRef},
        native_catalog::NativeCatalog,
        native_routing_lease::NativeRoutingLease,
        planner::ApplicationId,
        routing_control::{AssignmentResult, RoutingControl, StopReason},
        session::{PollReply, ReceivedNamePolicy},
        Assignment, Record,
    };
    use std::{
        collections::BTreeMap,
        io::{self, BufRead, Read, Write},
        path::Path,
    };
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 12 || args[0] != "--authorize-isolated-routing" {
        return Err("qualification only: native_session_control --authorize-isolated-routing RUNTIME CORE COOKIE SERVER-PID OWNER APP-ID=ALIAS APP-ID=ALIAS 1:ID:SERIAL 2:ID:SERIAL 3:ID:SERIAL 4:ID:SERIAL".into());
    }
    let runtime = Path::new(&args[1]);
    let name = &args[2];
    let cookie: u32 = args[3].parse()?;
    let server: u32 = args[4].parse()?;
    let mut bindings = BTreeMap::new();
    for arg in &args[6..8] {
        let (id, alias) = arg.split_once('=').ok_or("invalid binding")?;
        if bindings.insert(id.to_owned(), alias.to_owned()).is_some() {
            return Err("distinct applications required".into());
        }
    }
    let mut destinations = Vec::new();
    for arg in &args[8..] {
        let p: Vec<_> = arg.split(':').collect();
        if p.len() != 3 {
            return Err("invalid destination".into());
        }
        destinations.push(Destination {
            assignment: p[0].parse()?,
            node: ObjectRef {
                id: p[1].parse()?,
                serial: p[2].parse()?,
            },
        });
    }
    let mut catalog = NativeCatalog::connect(runtime, name)?;
    let first = catalog.observe(1)?;
    if first.core != format!("pw-core:{cookie}:pid:{server}:{name}") {
        return Err("catalog/core pin mismatch".into());
    }
    let mut service = ApplicationService::default();
    service.update(first)?;
    for (id, alias) in &bindings {
        service.select(&ApplicationId(id.clone()), alias)?;
    }
    let mut control = RoutingControl::<NativeRoutingLease>::default();
    fn status(control: &RoutingControl<NativeRoutingLease>, service: &ApplicationService) {
        let s = control.status();
        let (cleanup, released, absent) = match &s.cleanup {
            Some(Ok(r)) => ("verified", r.observed_links, r.already_absent),
            Some(Err(_)) => ("unverified", 0, 0),
            None => ("none", 0, 0),
        };
        println!("CONTROL status phase={:?} selected={} bound={} cleanup={cleanup} released={released} already_absent={absent}",s.phase,service.selected().len(),s.assignments.values().filter(|a|**a!=0).count());
        let _ = io::stdout().flush();
    }
    status(&control, &service);
    let mut generation = 1u64;
    let result: Result<(), String> = (|| {
        let mut input = io::stdin().lock();
        for _ in 0..24 {
            let mut line = String::new();
            if (&mut input)
                .take(1026)
                .read_line(&mut line)
                .map_err(|e| e.to_string())?
                == 0
            {
                return Ok(());
            }
            if line.len() > 1024 || !line.ends_with('\n') {
                return Err("bounded command required".into());
            }
            let fields: Vec<_> = line.split_whitespace().collect();
            if fields == ["quit"] {
                return Ok(());
            }
            generation = generation.checked_add(1).ok_or("generation exhausted")?;
            service.update(catalog.observe(generation)?)?;
            if control.check(&service).is_err() {
                println!("CONTROL lifecycle-stopped");
            }
            let command: Result<(), String> = (|| match fields.as_slice() {
                ["start"] => {
                    let selected = service
                        .selected()
                        .iter()
                        .map(|(id, alias)| (id.0.clone(), alias.clone()))
                        .collect();
                    control.start(&service, destinations.clone(), || {
                        NativeRoutingLease::connect(
                            runtime,
                            name,
                            cookie,
                            server,
                            selected,
                            args[5].clone(),
                        )
                    })?;
                    println!("CONTROL started");
                    Ok(())
                }
                ["stop"] => {
                    control.stop(StopReason::Owner)?;
                    println!("CONTROL stopped");
                    Ok(())
                }
                ["refresh"] | ["status"] => Ok(()),
                ["route", a, b] => {
                    let values = [
                        a.parse::<u8>().map_err(|_| "invalid assignment")?,
                        b.parse::<u8>().map_err(|_| "invalid assignment")?,
                    ];
                    let records = bindings
                        .values()
                        .zip(values)
                        .map(|(name, value)| Record {
                            name: name.as_bytes().to_vec(),
                            assignment: Assignment::from_byte(value),
                        })
                        .collect();
                    match control.assignments(
                        &service,
                        &PollReply::Records {
                            records,
                            ignored_tail: 0,
                        },
                        ReceivedNamePolicy::ExactBytes,
                    )? {
                        AssignmentResult::Verified(p) => {
                            println!("CONTROL routed records={}", p.records().len())
                        }
                        AssignmentResult::Blocked(_) => println!("CONTROL blocked no-mutation"),
                        AssignmentResult::NoAssignments => println!("CONTROL no-assignments"),
                    };
                    Ok(())
                }
                _ => Err("unknown qualification command".into()),
            })();
            if let Err(error) = command {
                println!("CONTROL refused {error:?}");
            }
            status(&control, &service);
        }
        Err("qualification command budget exhausted".into())
    })();
    let cleanup = control.stop(StopReason::Shutdown);
    status(&control, &service);
    result?;
    cleanup?;
    println!("CONTROL complete usb=absent physical_audio=absent");
    Ok(())
}
#[cfg(not(all(feature = "native-pipewire", target_os = "linux")))]
fn main() {
    eprintln!("requires Linux and native-pipewire; private-core qualification only");
    std::process::exit(2);
}
