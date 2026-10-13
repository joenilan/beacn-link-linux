//! Publication data derived from guarded route observations, never desired state.
use crate::{
    execution::Observation,
    executor::{self, Destination, Direction, Ownership},
    lifecycle::InventoryTracker,
    planner::{self, Inventory, StreamState},
    Assignment, Record,
};

pub struct VerifiedRoutePublication {
    inventory: Inventory,
    records: Vec<Record>,
}
impl VerifiedRoutePublication {
    /// Pure verification over trusted caller observations/receipt ledger. This
    /// does not establish OS freshness. The native owner supplies both snapshots.
    pub fn verify(
        expected: &Observation,
        observed: &Observation,
        destinations: &[Destination],
        ownership: &Ownership,
        assignment: u8,
    ) -> Result<Self, String> {
        if expected != observed || observed.inventory.applications.len() != 1 {
            return Err("route publication observation changed or is not one app".into());
        }
        executor::validate_graph(&observed.graph).map_err(|e| e.to_string())?;
        let inventory = &observed.inventory;
        if observed.graph.generation != inventory.snapshot_id {
            return Err("route publication generation mismatch".into());
        }
        let app = &inventory.applications[0];
        if app.streams.is_empty() || app.streams.iter().any(|s| s.state != StreamState::Live) {
            return Err("route publication source is unavailable".into());
        }
        let mut tracker = InventoryTracker::default();
        tracker.advance(inventory).map_err(|e| e.to_string())?;
        let records = vec![Record {
            name: app.advertised_name.clone(),
            assignment: Assignment::from_byte(assignment),
        }];
        crate::encode_records(&records).map_err(|e| e.to_string())?;
        let plan = planner::plan(inventory, &records).map_err(|e| e.to_string())?;
        if assignment != 0 {
            if plan.proposed_routes().count() != 1 {
                return Err("route publication assignment is unsupported".into());
            }
            let prepared = executor::prepare(
                &tracker,
                &plan,
                &expected.graph,
                &observed.graph,
                destinations,
                ownership,
            )
            .map_err(|e| e.to_string())?;
            if !prepared.steps.is_empty() {
                return Err("route publication is not yet applied".into());
            }
        }
        for stream in &app.streams {
            let bindings: Vec<_> = observed
                .graph
                .streams
                .iter()
                .filter(|s| s.application_id == app.id && s.stream_id == stream.id)
                .collect();
            if bindings.len() != 1 {
                return Err("route publication stream binding missing or ambiguous".into());
            }
            let ports: Vec<_> = observed
                .graph
                .ports
                .iter()
                .filter(|p| p.node == bindings[0].node && p.direction == Direction::Output)
                .collect();
            if ports.len() != 2
                || !ports.iter().any(|p| p.channel == executor::Channel::Left)
                || !ports.iter().any(|p| p.channel == executor::Channel::Right)
            {
                return Err("route publication requires exact stereo outputs".into());
            }
            let links: Vec<_> = observed
                .graph
                .links
                .iter()
                .filter(|l| ports.iter().any(|p| p.object == l.output))
                .collect();
            if assignment == 0 {
                if !links.is_empty() {
                    return Err("zero publication requires observed unlinked source".into());
                }
            } else if links.len() != 2
                || links.iter().any(|l| {
                    !ownership.links.contains(&l.object)
                        || l.owner.as_deref() != Some(ownership.token.as_str())
                })
            {
                return Err("route publication requires actual native link receipts".into());
            }
        }
        Ok(Self {
            inventory: inventory.clone(),
            records,
        })
    }
    pub fn inventory(&self) -> &Inventory {
        &self.inventory
    }
    pub fn records(&self) -> &[Record] {
        &self.records
    }
}
