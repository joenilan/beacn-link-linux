//! Explicit no-USB qualification only; uses the existing private-core executor.
#[cfg(all(feature = "native-pipewire", target_os = "linux"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use beacn_link_protocol::{
        catalog::ApplicationService,
        execution::{Backend, Effect, Execution, Observation},
        executor::{Destination, ObjectRef, Operation},
        native_catalog::NativeCatalog,
        native_pipewire::NativePipeWire,
        planner::ApplicationId,
        publication_epoch::PublicationEpoch,
        receipts::Acknowledgment,
        session::{PollReply, ReceivedNamePolicy},
        Assignment, Record,
    };
    use std::{collections::BTreeMap, path::Path};
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 12 || args[0] != "--authorize-isolated-routing" {
        return Err("qualification only: native_multi_app --authorize-isolated-routing RUNTIME CORE COOKIE SERVER-PID OWNER APP-ID=ALIAS APP-ID=ALIAS 1:ID:SERIAL 2:ID:SERIAL 3:ID:SERIAL 4:ID:SERIAL".into());
    }
    let runtime = Path::new(&args[1]);
    let name = &args[2];
    let cookie: u32 = args[3].parse()?;
    let server: u32 = args[4].parse()?;
    let mut bindings = BTreeMap::new();
    for arg in &args[6..8] {
        let (id, alias) = arg.split_once('=').ok_or("binding requires APP-ID=ALIAS")?;
        if bindings.insert(id.to_owned(), alias.to_owned()).is_some() {
            return Err("distinct applications required".into());
        }
    }
    let mut destinations = Vec::new();
    for arg in &args[8..] {
        let parts: Vec<_> = arg.split(':').collect();
        if parts.len() != 3 {
            return Err("invalid destination".into());
        }
        destinations.push(Destination {
            assignment: parts[0].parse()?,
            node: ObjectRef {
                id: parts[1].parse()?,
                serial: parts[2].parse()?,
            },
        });
    }
    let mut catalog = NativeCatalog::connect(runtime, name)?;
    let mut service = ApplicationService::default();
    let observed = catalog.observe(1)?;
    if observed.core != format!("pw-core:{cookie}:pid:{server}:{name}") {
        return Err("catalog does not match the explicitly pinned native core".into());
    }
    service.update(observed)?;
    for (id, alias) in &bindings {
        service.select(&ApplicationId(id.clone()), alias)?;
    }
    let connect = || {
        NativePipeWire::connect(
            runtime,
            name,
            cookie,
            server,
            bindings.clone(),
            args[5].clone(),
        )
    };
    let assignments = |a, b| {
        bindings
            .keys()
            .cloned()
            .map(ApplicationId)
            .zip([a, b])
            .collect::<BTreeMap<_, _>>()
    };
    let reply = |a, b| PollReply::Records {
        records: bindings
            .values()
            .zip([a, b])
            .map(|(alias, value)| Record {
                name: alias.as_bytes().to_vec(),
                assignment: Assignment::from_byte(value),
            })
            .collect(),
        ignored_tail: 0,
    };
    // Success: two native app identities simultaneously, then move both to the
    // other pairs. Dropping this retained backend removes only its own proxies.
    {
        let mut backend = connect()?;
        let baseline = backend.observe(service.revision())?;
        let mut epoch = PublicationEpoch::begin(&service, &baseline)?;
        let pending = epoch
            .receive(&service, &reply(1, 4), ReceivedNamePolicy::ExactBytes)?
            .ok_or("missing plan")?;
        let fresh = backend.observe(service.revision())?;
        let prepared = epoch.prepare(
            &service,
            &pending,
            &baseline,
            &fresh,
            &destinations,
            backend.ownership(),
        )?;
        if prepared.steps.len() != 4 {
            return Err("two stereo native applications required".into());
        }
        let mut first = Execution::new(prepared, &fresh, backend.ownership().clone())?;
        first.run(&mut backend)?;
        let before = backend.observe(service.revision())?;
        let after = backend.observe(service.revision())?;
        let proof = epoch.verify_routes(
            &service,
            &before,
            &after,
            &destinations,
            first.journal().ownership(),
            &assignments(1, 4),
        )?;
        beacn_link_protocol::encode_records(proof.records())?;
        let pending = epoch
            .receive(&service, &reply(2, 3), ReceivedNamePolicy::ExactBytes)?
            .ok_or("missing move plan")?;
        let prepared = epoch.prepare(
            &service,
            &pending,
            &before,
            &after,
            &destinations,
            backend.ownership(),
        )?;
        if prepared.steps.len() != 8 {
            return Err("two complete stereo moves required".into());
        }
        let mut moved = Execution::new(prepared, &after, backend.ownership().clone())?;
        moved.run(&mut backend)?;
        let before = backend.observe(service.revision())?;
        let after = backend.observe(service.revision())?;
        epoch.verify_routes(
            &service,
            &before,
            &after,
            &destinations,
            moved.journal().ownership(),
            &assignments(2, 3),
        )?;
        moved.rollback(&mut backend)?;
        let before = backend.observe(service.revision())?;
        let after = backend.observe(service.revision())?;
        epoch.verify_routes(
            &service,
            &before,
            &after,
            &destinations,
            moved
                .recovery_journal()
                .ok_or("missing recovery journal")?
                .ownership(),
            &assignments(1, 4),
        )?;
        println!("MULTI success apps=2 created=4 move_steps=8 restored=4");
    }
    struct FailBefore<'a> {
        backend: &'a mut NativePipeWire,
        remaining: usize,
    }
    impl Backend for FailBefore<'_> {
        fn observe(&mut self, generation: u64) -> Result<Observation, String> {
            self.backend.observe(generation)
        }
        fn perform(&mut self, operation: &Operation) -> Effect {
            if self.remaining == 0 {
                return Effect::Failed("controlled failure before native request".into());
            }
            self.remaining -= 1;
            self.backend.perform(operation)
        }
        fn reconcile(&mut self) -> Result<Option<Acknowledgment>, String> {
            self.backend.reconcile()
        }
    }
    // A new retained backend must observe the prior backend's links gone before
    // a new epoch can begin. No retry/fallback or receipt reuse.
    {
        let mut backend = connect()?;
        let baseline = backend.observe(service.revision())?;
        let mut epoch = PublicationEpoch::begin(&service, &baseline)?;
        let pending = epoch
            .receive(&service, &reply(1, 4), ReceivedNamePolicy::ExactBytes)?
            .ok_or("missing plan")?;
        let fresh = backend.observe(service.revision())?;
        let prepared = epoch.prepare(
            &service,
            &pending,
            &baseline,
            &fresh,
            &destinations,
            backend.ownership(),
        )?;
        let mut execution = Execution::new(prepared, &fresh, backend.ownership().clone())?;
        let result = execution.run(&mut FailBefore {
            backend: &mut backend,
            remaining: 2,
        });
        if result.is_ok() || execution.journal().prefix() != 2 {
            return Err("partial-failure test did not stop at owned prefix".into());
        }
        let before = backend.observe(service.revision())?;
        let after = backend.observe(service.revision())?;
        if epoch
            .verify_routes(
                &service,
                &before,
                &after,
                &destinations,
                execution.journal().ownership(),
                &assignments(1, 4),
            )
            .is_ok()
            || !epoch.halted()
        {
            return Err("partial desired state became publishable".into());
        }
        execution.rollback(&mut backend)?;
        if backend.observe(service.revision())? != baseline {
            return Err("native partial rollback differs from baseline".into());
        }
        println!("MULTI failure acknowledged_prefix=2 publication=refused rollback=exact");
    }
    println!("MULTI complete usb=absent physical_audio=absent");
    Ok(())
}
#[cfg(not(all(feature = "native-pipewire", target_os = "linux")))]
fn main() {
    eprintln!("requires Linux and native-pipewire; private-core qualification only");
    std::process::exit(2);
}
