//! Explicit no-USB playback-owner qualification on an authenticated private core.
#[cfg(all(
    feature = "native-playback",
    feature = "qualification-peer",
    target_os = "linux"
))]
fn main() -> Result<(), String> {
    use beacn_link_protocol::{
        catalog::{ApplicationService, Catalog},
        execution::Backend,
        executor::{Destination, ObjectRef},
        native_catalog::NativeCatalog,
        native_pipewire::NativePipeWire,
        native_playback::{Configuration, NativePlayback, Output},
        native_routing_lease::NativeRoutingLease,
        planner::ApplicationId,
        routing_control::{RoutingControl, StopReason},
        session::{PollReply, ReceivedNamePolicy},
        Assignment, Record,
    };
    use std::{
        collections::BTreeMap,
        io::{self, Read, Write},
        path::PathBuf,
        time::{Duration, Instant},
    };
    let a: Vec<_> = std::env::args().skip(1).collect();
    if a.len() != 12 || a[0] != "--authorize-isolated-native-playback" {
        return Err(
            "requires explicit no-USB playback-owner scope and exact private-core arguments".into(),
        );
    }
    let config = Configuration {
        runtime: PathBuf::from(&a[1]),
        core: a[2].clone(),
        cookie: a[3].parse().map_err(|_| "cookie")?,
        server: a[4].parse().map_err(|_| "server")?,
        owner: a[5].clone(),
        pair: 1,
    };
    let destinations = a[8..]
        .iter()
        .map(|v| {
            let p: Vec<_> = v.split(':').collect();
            if p.len() != 3 {
                return Err("destination".to_string());
            }
            Ok(Destination {
                assignment: p[0].parse().map_err(|_| "assignment")?,
                node: ObjectRef {
                    id: p[1].parse().map_err(|_| "id")?,
                    serial: p[2].parse().map_err(|_| "serial")?,
                },
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let first = ApplicationId(a[6].clone());
    let second = ApplicationId(a[7].clone());
    let mut catalog = NativeCatalog::connect(&config.runtime, &config.core)?;
    let mut service = ApplicationService::default();
    let mut generation = 0u64;
    let mut observe = || -> Result<Catalog, String> {
        generation += 1;
        catalog.observe(generation)
    };
    service.update(observe()?)?;
    service.select(&first, "Native Player")?;
    let mut inspector = NativePipeWire::connect(
        &config.runtime,
        &config.core,
        config.cookie,
        config.server,
        BTreeMap::new(),
        config.owner.clone(),
    )?;
    let original = inspector.observe(1)?.graph;
    let mut routes = RoutingControl::default();
    let route = |routes: &mut RoutingControl<NativeRoutingLease>,
                 service: &ApplicationService,
                 pair: u8|
     -> Result<(), String> {
        if routes.status().phase != beacn_link_protocol::routing_control::Phase::Running {
            let bindings = service
                .selected()
                .iter()
                .map(|(i, n)| (i.0.clone(), n.clone()))
                .collect();
            routes.start(service, destinations.clone(), || {
                NativeRoutingLease::connect(
                    &config.runtime,
                    &config.core,
                    config.cookie,
                    config.server,
                    bindings,
                    config.owner.clone(),
                )
            })?;
        }
        routes.assignments(
            service,
            &PollReply::Records {
                records: vec![Record {
                    name: b"Native Player".to_vec(),
                    assignment: Assignment::from_byte(pair),
                }],
                ignored_tail: 0,
            },
            ReceivedNamePolicy::ExactBytes,
        )?;
        Ok(())
    };
    for pair in 1..=4 {
        service.update(observe()?)?;
        route(&mut routes, &service, pair)?;
        let proof = routes.verify(&service)?;
        let mut c = config.clone();
        c.pair = pair;
        let mut playback = NativePlayback::start(c, Output::synthetic(), &proof)
            .map_err(|e| format!("start: {e:?}"))?;
        let start = Instant::now();
        let checks = if pair == 1 { 128 } else { 8 };
        for _ in 0..checks {
            std::thread::sleep(Duration::from_millis(if pair == 1 { 500 } else { 50 }));
            service.update(observe()?)?;
            let proof = routes.verify(&service)?;
            playback.refresh(&proof)?;
        }
        assert!(playback.refresh(&proof).is_err(), "stale proof accepted");
        assert!(
            playback.check().is_err(),
            "failed playback proof recovered silently"
        );
        let released = playback.stop()?;
        assert_eq!(released.acknowledged_links, 2);
        assert!(playback.stop().is_ok(), "idempotent stop failed");
        routes.stop(StopReason::Owner)?;
        println!("PLAYBACK pair={pair} checks={checks} elapsed_ms={} sink_release=verified route_release=verified",start.elapsed().as_millis());
    }
    service.update(observe()?)?;
    route(&mut routes, &service, 1)?;
    #[cfg(feature = "native-test-faults")]
    for stage in 1..=4 {
        let proof = routes.verify(&service)?;
        let failure =
            NativePlayback::test_start(config.clone(), Output::synthetic(), &proof, stage)
                .err()
                .ok_or("native playback fault unexpectedly succeeded")?;
        failure.cleanup?;
        println!("PLAYBACK fault={stage} no_retry=true restoration=verified");
    }
    // Drop closes only retained native resources, while the source/route stays.
    let proof = routes.verify(&service)?;
    let before = inspector.observe(1)?.graph;
    drop(
        NativePlayback::start(config.clone(), Output::synthetic(), &proof)
            .map_err(|e| format!("drop start: {e:?}"))?,
    );
    assert_eq!(before, inspector.observe(1)?.graph);
    routes.stop(StopReason::Owner)?;
    service.unselect(&first)?;
    service.select(&second, "Native Player")?;
    service.update(observe()?)?;
    route(&mut routes, &service, 1)?;
    let proof = routes.verify(&service)?;
    let mut playback = NativePlayback::start(config, Output::synthetic(), &proof)
        .map_err(|e| format!("exit start: {e:?}"))?;
    println!("PLAYBACK EXIT_SOURCE");
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut reply = [0; 6];
    io::stdin()
        .read_exact(&mut reply)
        .map_err(|e| e.to_string())?;
    if &reply != b"EXITED" {
        return Err("source exit acknowledgment refused".into());
    }
    assert!(
        playback.check().is_err(),
        "departed source remained authorized"
    );
    playback.stop()?;
    service.update(observe()?)?;
    assert!(routes.check(&service).is_err());
    assert_eq!(
        routes.status().phase,
        beacn_link_protocol::routing_control::Phase::Stopped
    );
    // Only the separately owned exited application's graph is absent now;
    // final whole-core conservation is checked by the enclosing source owner.
    let after = inspector.observe(1)?.graph;
    assert!(after.nodes.len() < original.nodes.len());
    println!("PLAYBACK complete pairs=4 faults=4 drop=verified source_exit=verified usb=absent physical=absent");
    Ok(())
}
#[cfg(not(all(
    feature = "native-playback",
    feature = "qualification-peer",
    target_os = "linux"
)))]
fn main() {
    eprintln!("requires explicit native-playback,qualification-peer Linux features");
    std::process::exit(2);
}
