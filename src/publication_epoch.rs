//! Offline selection -> reply -> guarded preparation -> observed publication.
//! No transport, native connection, route execution, or automatic recovery.
use crate::{
    catalog::ApplicationService,
    execution::Observation,
    executor::{self, Destination, Ownership, PreparedChange},
    lifecycle::InventoryTracker,
    planner::{self, ApplicationId, Inventory, Plan},
    route_publication::VerifiedRoutePublication,
    session::{PollReply, ReceivedNamePolicy},
};
use std::{collections::BTreeMap, sync::Arc};

/// A caller-observed fixed roster. Any selected lifecycle/edit or domain change
/// permanently halts this epoch. Cleanup/recovery belongs to the receipt owner;
/// constructing another epoch never grants authority to remove prior links.
pub struct PublicationEpoch {
    identity: Arc<()>,
    catalog_core: String,
    graph_core: String,
    selection_revision: u64,
    inventory: Inventory,
    halted: bool,
}
/// A plan anchored to this observed inventory, not just a numeric generation.
/// Cannot be constructed or changed by a caller.
pub struct PendingAssignments {
    epoch: Arc<()>,
    inventory: Inventory,
    catalog_core: String,
    graph_core: String,
    selection_revision: u64,
    plan: Plan,
}
impl PendingAssignments {
    pub fn plan(&self) -> &Plan {
        &self.plan
    }
}
impl PublicationEpoch {
    /// The caller must establish that both observations came from the same
    /// retained native core. Catalog and executor domain strings have different
    /// formats; this pure model pins both and never guesses their equivalence.
    /// Start requires every selected source to be observed exactly unlinked.
    pub fn begin(service: &ApplicationService, baseline: &Observation) -> Result<Self, String> {
        let catalog = service.current().ok_or("no observed catalog")?;
        let inventory = service.inventory().ok_or("no selected inventory")?;
        if &baseline.inventory != inventory {
            return Err("baseline differs from current selection".into());
        }
        let unlinked = inventory
            .applications
            .iter()
            .map(|a| (a.id.clone(), 0))
            .collect();
        VerifiedRoutePublication::verify_selected(
            baseline,
            baseline,
            &[],
            &Ownership {
                token: "epoch-start".into(),
                links: Default::default(),
            },
            &unlinked,
        )?;
        Ok(Self {
            identity: Arc::new(()),
            catalog_core: catalog.core.clone(),
            graph_core: baseline.graph.core_id.clone(),
            selection_revision: service.selection_revision(),
            inventory: inventory.clone(),
            halted: false,
        })
    }
    pub fn halted(&self) -> bool {
        self.halted
    }
    fn finish<T>(&mut self, result: Result<T, String>) -> Result<T, String> {
        if result.is_err() {
            self.halted = true;
        }
        result
    }
    /// Revalidate after each catalog/selection update, before any further work.
    /// This is a caller-observation check, not an OS read or cleanup operation.
    pub fn check_current(&mut self, service: &ApplicationService) -> Result<(), String> {
        let result = (|| {
            if self.halted {
                return Err("publication epoch is halted".into());
            }
            let catalog = service.current().ok_or("catalog unavailable")?;
            let inventory = service.inventory().ok_or("selection unavailable")?;
            if catalog.core != self.catalog_core
                || service.selection_revision() != self.selection_revision
                || inventory.applications != self.inventory.applications
                || inventory.snapshot_id < self.inventory.snapshot_id
            {
                return Err("selected roster or domain changed; explicit recovery required".into());
            }
            self.inventory = inventory.clone();
            Ok(())
        })();
        self.finish(result)
    }
    fn observation(&self, observation: &Observation) -> Result<(), String> {
        if observation.inventory != self.inventory
            || observation.graph.generation != self.inventory.snapshot_id
            || observation.graph.core_id != self.graph_core
        {
            return Err("route observation does not match current epoch".into());
        }
        Ok(())
    }
    /// Opaque/no-payload replies yield no plan, never an empty-list/unroute plan.
    /// Unsupported/ambiguous assignments remain explicit planner outcomes.
    pub fn receive(
        &mut self,
        service: &ApplicationService,
        reply: &PollReply,
        names: ReceivedNamePolicy,
    ) -> Result<Option<PendingAssignments>, String> {
        self.check_current(service)?;
        let result = match reply {
            PollReply::Records { records, .. } => {
                let records: Vec<_> = records.iter().map(|r| names.for_planning(r)).collect();
                planner::plan(&self.inventory, &records)
                    .map(|plan| {
                        Some(PendingAssignments {
                            epoch: self.identity.clone(),
                            inventory: self.inventory.clone(),
                            catalog_core: self.catalog_core.clone(),
                            graph_core: self.graph_core.clone(),
                            selection_revision: self.selection_revision,
                            plan,
                        })
                    })
                    .map_err(|e| e.to_string())
            }
            PollReply::NoPayload { .. } | PollReply::RejectedMarker { .. } => Ok(None),
        };
        self.finish(result)
    }
    /// Data-only guarded operations. The existing Execution/receipt owner must
    /// still validate before each real operation and handle partial failures.
    pub fn prepare(
        &mut self,
        service: &ApplicationService,
        pending: &PendingAssignments,
        expected: &Observation,
        observed: &Observation,
        destinations: &[Destination],
        ownership: &Ownership,
    ) -> Result<PreparedChange, String> {
        self.check_current(service)?;
        let result = (|| {
            if !Arc::ptr_eq(&pending.epoch, &self.identity)
                || pending.inventory != self.inventory
                || pending.catalog_core != self.catalog_core
                || pending.graph_core != self.graph_core
                || pending.selection_revision != self.selection_revision
            {
                return Err("pending assignments are stale".into());
            }
            service.require_current_plan(&pending.plan)?;
            self.observation(expected)?;
            self.observation(observed)?;
            let mut tracker = InventoryTracker::default();
            tracker
                .advance(&self.inventory)
                .map_err(|e| e.to_string())?;
            executor::prepare(
                &tracker,
                &pending.plan,
                &expected.graph,
                &observed.graph,
                destinations,
                ownership,
            )
            .map_err(|e| e.to_string())
        })();
        self.finish(result)
    }
    /// Publishable data only after all selected apps' actual routes validate.
    /// No desired/partial result is cached; the native owner supplies freshness.
    pub fn verify_routes(
        &mut self,
        service: &ApplicationService,
        expected: &Observation,
        observed: &Observation,
        destinations: &[Destination],
        ownership: &Ownership,
        assignments: &BTreeMap<ApplicationId, u8>,
    ) -> Result<VerifiedRoutePublication, String> {
        self.check_current(service)?;
        let result = (|| {
            self.observation(expected)?;
            self.observation(observed)?;
            VerifiedRoutePublication::verify_selected(
                expected,
                observed,
                destinations,
                ownership,
                assignments,
            )
        })();
        self.finish(result)
    }
}
