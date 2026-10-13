use beacn_link_protocol::{
    execution::{Backend, Execution},
    executor::{prepare, Destination, ObjectRef},
    lifecycle::InventoryTracker,
    native_pipewire::NativePipeWire,
    planner::{self, Inventory},
    route_publication::VerifiedRoutePublication,
    Record,
};
use std::{collections::BTreeMap, path::Path};

pub struct PrivateRoutes {
    backend: NativePipeWire,
    destinations: Vec<Destination>,
    assignment: u8,
}
impl PrivateRoutes {
    pub fn connect(args: &[String], inventory: &Inventory) -> Result<Self, String> {
        if args.len() != 9 || inventory.applications.len() != 1 {
            return Err(
                "requires private runtime/core/cookie/PID/token and four explicit destinations"
                    .into(),
            );
        }
        let mut destinations = Vec::new();
        for (index, value) in args[5..].iter().enumerate() {
            let fields: Vec<_> = value.split(':').collect();
            if fields.len() != 3 || fields[0] != (index + 1).to_string() {
                return Err("requires ordered explicit assignments 1..4".into());
            }
            destinations.push(Destination {
                assignment: (index + 1) as u8,
                node: ObjectRef {
                    id: fields[1].parse().map_err(|_| "invalid destination ID")?,
                    serial: fields[2]
                        .parse()
                        .map_err(|_| "invalid destination serial")?,
                },
            });
        }
        let app = &inventory.applications[0];
        let bindings = BTreeMap::from([(
            app.id.0.clone(),
            String::from_utf8(app.advertised_name.clone()).map_err(|_| "unsupported name")?,
        )]);
        let backend = NativePipeWire::connect(
            Path::new(&args[0]),
            &args[1],
            args[2].parse().map_err(|_| "invalid cookie")?,
            args[3].parse().map_err(|_| "invalid server PID")?,
            bindings,
            args[4].clone(),
        )?;
        Ok(Self {
            backend,
            destinations,
            assignment: 0,
        })
    }
    pub fn verify(&mut self, inventory: &Inventory) -> Result<VerifiedRoutePublication, String> {
        let expected = self.backend.observe(inventory.snapshot_id)?;
        let observed = self.backend.observe(inventory.snapshot_id)?;
        if expected.inventory != *inventory || observed.inventory != *inventory {
            return Err("native and supervising inventories differ".into());
        }
        VerifiedRoutePublication::verify(
            &expected,
            &observed,
            &self.destinations,
            self.backend.ownership(),
            self.assignment,
        )
    }
    pub fn apply(&mut self, inventory: &Inventory, records: &[Record]) -> Result<(), String> {
        let expected = self.backend.observe(inventory.snapshot_id)?;
        let observed = self.backend.observe(inventory.snapshot_id)?;
        if expected.inventory != *inventory || observed.inventory != *inventory {
            return Err("native source changed before routing".into());
        }
        let mut tracker = InventoryTracker::default();
        tracker.advance(inventory).map_err(|e| e.to_string())?;
        let plan = planner::plan(inventory, records).map_err(|e| e.to_string())?;
        let routes: Vec<_> = plan.proposed_routes().collect();
        if routes.len() != 1 {
            return Err("requires one resolved route".into());
        }
        let assignment = routes[0].link.assignment();
        let prepared = prepare(
            &tracker,
            &plan,
            &expected.graph,
            &observed.graph,
            &self.destinations,
            self.backend.ownership(),
        )
        .map_err(|e| e.to_string())?;
        let mut run = Execution::new(prepared, &observed, self.backend.ownership().clone())?;
        let result = run.run(&mut self.backend);
        println!(
            "private_route_execution state={:?} receipt_prefix={}",
            run.journal().state(),
            run.journal().prefix()
        );
        result?;
        self.assignment = assignment;
        let verified = self.verify(inventory)?;
        println!(
            "private_route_applied assignment={} verified=true",
            verified.records()[0].assignment.byte()
        );
        Ok(())
    }
}
