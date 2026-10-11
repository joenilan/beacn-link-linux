//! Deliberately separate from the offline CLI. Feature gated and private-core only.
#[cfg(all(feature = "native-pipewire", target_os = "linux"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use beacn_link_protocol::{
        decode_frame, decode_records,
        execution::{Backend, Effect, Execution, Observation},
        executor::{prepare, Destination, ObjectRef, Operation},
        lifecycle::InventoryTracker,
        native_pipewire::NativePipeWire,
        planner,
        receipts::Acknowledgment,
        Frame,
    };
    use std::{
        collections::BTreeMap,
        io::{self, BufRead, Read},
        path::Path,
    };
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|v| v == "--help") || args.len() < 7 {
        println!("Explicit disposable-core executor. Audio mutation requires owner authorization.\nUsage: native_private RUNTIME CORE COOKIE SERVER-PID OWNER APP-ID=WIRE-NAME ASSIGNMENT:NODE-ID:SERIAL ...\nStdin commands: route GENERATION FRAME-HEX | route-fail-before GENERATION FRAME-HEX STEP | reconcile | rollback | resume-rollback | quit\nWith native-test-faults: route-native-reject/route-native-unknown GENERATION FRAME-HEX STEP; rollback-native-reject/rollback-native-unknown STEP; fault-stats.\nFaults are one-shot and confined to the test feature and this example.\nRequires exact native_pipewire::CLIENT_CONFIG in private RUNTIME/client.conf.\nEvery route uses native observations and session-created receipts. No desktop/default core support.");
        return Ok(());
    }
    let mut bindings = BTreeMap::new();
    let (id, name) = args[5].split_once('=').ok_or("expected APP-ID=WIRE-NAME")?;
    bindings.insert(id.to_owned(), name.to_owned());
    let mut destinations = vec![];
    for value in &args[6..] {
        let f: Vec<_> = value.split(':').collect();
        if f.len() != 3 {
            return Err("expected ASSIGNMENT:NODE-ID:SERIAL".into());
        }
        destinations.push(Destination {
            assignment: f[0].parse()?,
            node: ObjectRef {
                id: f[1].parse()?,
                serial: f[2].parse()?,
            },
        });
    }
    let mut backend = NativePipeWire::connect(
        Path::new(&args[0]),
        &args[1],
        args[2].parse()?,
        args[3].parse()?,
        bindings,
        args[4].clone(),
    )?;
    let mut execution: Option<Execution> = None;
    struct FaultAt<'a> {
        backend: &'a mut NativePipeWire,
        remaining: Option<usize>,
        kind: &'a str,
    }
    impl Backend for FaultAt<'_> {
        fn observe(&mut self, generation: u64) -> Result<Observation, String> {
            self.backend.observe(generation)
        }
        fn perform(&mut self, operation: &Operation) -> Effect {
            if let Some(remaining) = self.remaining {
                if remaining != 0 {
                    self.remaining = Some(remaining - 1);
                } else {
                    self.remaining = None;
                    if self.kind == "before" {
                        return Effect::Failed("controlled failure before native request".into());
                    }
                    #[cfg(feature = "native-test-faults")]
                    {
                        use beacn_link_protocol::native_pipewire::NativeTestFault;
                        let fault = if self.kind == "reject" {
                            if !matches!(operation, Operation::Connect { .. }) {
                                return Effect::Failed("rejection fault requires a create".into());
                            }
                            NativeTestFault::RejectCreate
                        } else {
                            NativeTestFault::WithholdAcknowledgment
                        };
                        if let Err(error) = self.backend.arm_test_fault(fault) {
                            return Effect::Failed(error);
                        }
                    }
                    #[cfg(not(feature = "native-test-faults"))]
                    return Effect::Failed(
                        "native faults require native-test-faults feature".into(),
                    );
                }
            }
            self.backend.perform(operation)
        }
        fn reconcile(&mut self) -> Result<Option<Acknowledgment>, String> {
            self.backend.reconcile()
        }
    }
    let mut blocked = false;
    let mut last_generation = None;
    let mut input = io::stdin().lock();
    loop {
        let mut line = String::new();
        if (&mut input).take(2049).read_line(&mut line)? == 0 {
            break;
        }
        if line.len() > 2048 {
            return Err("command too large".into());
        }
        let fields: Vec<_> = line.split_whitespace().collect();
        let result: Result<(), String> = (|| match fields.as_slice() {
            ["quit"] => return Ok(()),
            ["fault-stats"] => {
                #[cfg(feature = "native-test-faults")]
                {
                    println!("native-test-stats {:?}", backend.test_fault_stats());
                    Ok(())
                }
                #[cfg(not(feature = "native-test-faults"))]
                Err("native faults require native-test-faults feature".into())
            }
            ["route", generation, hex]
            | ["route-fail-before" | "route-native-reject" | "route-native-unknown", generation, hex, _]
                if !blocked =>
            {
                let fail_before = if fields.len() == 4 {
                    Some(
                        fields[3]
                            .parse::<usize>()
                            .map_err(|_| "invalid failure step")?,
                    )
                } else {
                    None
                };
                let generation: u64 = generation.parse().map_err(|_| "invalid generation")?;
                if last_generation.is_some_and(|old| generation <= old) {
                    return Err("generation must increase".into());
                }
                if hex.len() > 1024 || hex.len() % 2 != 0 || !hex.is_ascii() {
                    return Err("invalid frame hex".into());
                }
                let bytes = (0..hex.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| "invalid frame hex")?;
                let Frame::Data(payload) = decode_frame(&bytes).map_err(|e| e.to_string())? else {
                    return Err("data frame required".into());
                };
                let records = decode_records(&payload).map_err(|e| e.to_string())?;
                let anchored = backend.observe(generation)?;
                let fresh = backend.observe(generation)?;
                if anchored.inventory != fresh.inventory {
                    return Err("inventory changed while preparing".into());
                }
                let mut tracker = InventoryTracker::default();
                tracker
                    .advance(&fresh.inventory)
                    .map_err(|e| e.to_string())?;
                let plan = planner::plan(&fresh.inventory, &records).map_err(|e| e.to_string())?;
                let prepared = prepare(
                    &tracker,
                    &plan,
                    &anchored.graph,
                    &fresh.graph,
                    &destinations,
                    backend.ownership(),
                )
                .map_err(|e| e.to_string())?;
                if fail_before.is_some_and(|step| step >= prepared.steps.len()) {
                    return Err("failure step must be inside batch".into());
                }
                let mut run = Execution::new(prepared, &fresh, backend.ownership().clone())?;
                last_generation = Some(generation);
                let result = if let Some(remaining) = fail_before {
                    run.run(&mut FaultAt {
                        backend: &mut backend,
                        remaining: Some(remaining),
                        kind: match fields[0] {
                            "route-native-reject" => "reject",
                            "route-native-unknown" => "unknown",
                            _ => "before",
                        },
                    })
                } else {
                    run.run(&mut backend)
                };
                blocked = result.is_err();
                execution = Some(run);
                result
            }
            ["reconcile"] => execution
                .as_mut()
                .ok_or("no execution")?
                .reconcile(&mut backend),
            ["rollback"] => {
                let result = execution
                    .as_mut()
                    .ok_or("no execution")?
                    .rollback(&mut backend);
                blocked = result.is_err();
                result
            }
            ["rollback-fail-before" | "rollback-native-reject" | "rollback-native-unknown", step] =>
            {
                let remaining = step.parse::<usize>().map_err(|_| "invalid failure step")?;
                let result = execution
                    .as_mut()
                    .ok_or("no execution")?
                    .rollback(&mut FaultAt {
                        backend: &mut backend,
                        remaining: Some(remaining),
                        kind: match fields[0] {
                            "rollback-native-reject" => "reject",
                            "rollback-native-unknown" => "unknown",
                            _ => "before",
                        },
                    });
                blocked = result.is_err();
                result
            }
            ["resume-rollback"] => {
                let result = execution
                    .as_mut()
                    .ok_or("no execution")?
                    .resume_rollback(&mut backend);
                blocked = result.is_err();
                result
            }
            _ => Err("invalid command or stopped execution; reconcile/rollback required".into()),
        })();
        if let Some(run) = &execution {
            println!(
                "forward {:?} prefix={} receipts={:?}; recovery={:?}",
                run.journal().state(),
                run.journal().prefix(),
                backend.ownership().links,
                run.recovery_journal().map(|j| (j.state(), j.prefix()))
            );
        }
        match result {
            Ok(()) => println!("OK"),
            Err(error) => println!("STOP: {error}"),
        }
        if fields.as_slice() == ["quit"] {
            break;
        }
    }
    // Session disconnection releases this client's non-lingering resources.
    // This is private-core teardown, not a claim of full route restoration.
    Ok(())
}
#[cfg(not(all(feature = "native-pipewire", target_os = "linux")))]
fn main() {
    eprintln!("native private-core executor requires Linux and --features native-pipewire");
    std::process::exit(2);
}
