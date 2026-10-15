//! Publication data derived from guarded route observations, never desired state.
use crate::{
    execution::Observation,
    executor::{self, Destination, Direction, Ownership},
    lifecycle::InventoryTracker,
    planner::{self, ApplicationId, Inventory, StreamState},
    Assignment, Record,
};
use std::collections::{BTreeMap, BTreeSet};

pub struct VerifiedRoutePublication {
    inventory: Inventory,
    records: Vec<Record>,
    destinations: Vec<Destination>,
    ownership: Ownership,
    core: String,
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
        if observed.inventory.applications.len() != 1 {
            return Err("single-app route publication requires one app".into());
        }
        Self::verify_selected(
            expected,
            observed,
            destinations,
            ownership,
            &[(observed.inventory.applications[0].id.clone(), assignment)]
                .into_iter()
                .collect(),
        )
    }
    /// Explicit multi-app proof. Every selected app needs an entry; no absent
    /// assignment is interpreted as zero, departure, or an unroute instruction.
    /// The original single-app USB constructor remains separately restricted.
    pub fn verify_selected(
        expected: &Observation,
        observed: &Observation,
        destinations: &[Destination],
        ownership: &Ownership,
        assignments: &BTreeMap<ApplicationId, u8>,
    ) -> Result<Self, String> {
        if expected != observed || observed.inventory.applications.is_empty() {
            return Err("route publication observation changed or is empty".into());
        }
        executor::validate_graph(&observed.graph).map_err(|e| e.to_string())?;
        let inventory = &observed.inventory;
        if observed.graph.generation != inventory.snapshot_id {
            return Err("route publication generation mismatch".into());
        }
        let mut tracker = InventoryTracker::default();
        tracker.advance(inventory).map_err(|e| e.to_string())?;
        let mut names = BTreeSet::new();
        let mut apps: Vec<_> = inventory.applications.iter().collect();
        apps.sort_by(|a, b| a.id.cmp(&b.id));
        if assignments.len() != apps.len() {
            return Err("route publication requires the complete selected assignment map".into());
        }
        let mut records = Vec::new();
        for app in &apps {
            if app.streams.is_empty() || app.streams.iter().any(|s| s.state != StreamState::Live) {
                return Err("route publication source is unavailable".into());
            }
            if !names.insert(&app.advertised_name) {
                return Err("route publication wire names are ambiguous".into());
            }
            let assignment = *assignments
                .get(&app.id)
                .ok_or("missing selected assignment")?;
            if assignment > 4 {
                return Err("route publication assignment is unsupported".into());
            }
            records.push(Record {
                name: app.advertised_name.clone(),
                assignment: Assignment::from_byte(assignment),
            });
        }
        crate::encode_records(&records).map_err(|e| e.to_string())?;
        let routed: Vec<_> = records
            .iter()
            .filter(|r| r.assignment.byte() != 0)
            .cloned()
            .collect();
        if !routed.is_empty() {
            let plan = planner::plan(inventory, &routed).map_err(|e| e.to_string())?;
            if plan.proposed_routes().count() != routed.len() {
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
        for app in apps {
            let assignment = assignments[&app.id];
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
        }
        Ok(Self {
            inventory: inventory.clone(),
            records,
            destinations: destinations.to_vec(),
            ownership: ownership.clone(),
            core: observed.graph.core_id.clone(),
        })
    }
    pub fn inventory(&self) -> &Inventory {
        &self.inventory
    }
    pub fn records(&self) -> &[Record] {
        &self.records
    }
    /// Revalidate the same fully verified assignments against a fresh native
    /// observation, without extending its inventory generation or authority.
    pub fn verify_observation(&self, observed: &Observation) -> Result<(), String> {
        if observed.inventory != self.inventory || observed.graph.core_id != self.core {
            return Err("playback route inventory changed".into());
        }
        let assignments = self
            .inventory
            .applications
            .iter()
            .map(|app| {
                let record = self
                    .records
                    .iter()
                    .find(|r| r.name == app.advertised_name)
                    .ok_or("verified publication record absent")?;
                Ok((app.id.clone(), record.assignment.byte()))
            })
            .collect::<Result<BTreeMap<_, _>, String>>()?;
        Self::verify_selected(
            observed,
            observed,
            &self.destinations,
            &self.ownership,
            &assignments,
        )?;
        Ok(())
    }
    pub fn destination(&self, assignment: u8) -> Option<&Destination> {
        self.destinations
            .iter()
            .find(|d| d.assignment == assignment)
    }
}
