//! Ordered, complete inventory transitions and a pure plan-generation guard.
use crate::planner::{self, ApplicationId, Inventory, InventoryError, Plan, Stream, StreamId};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamChange {
    pub application_id: ApplicationId,
    /// Latest state; a missing stream is reported once as Disappeared.
    pub stream: Stream,
    pub previous_state: Option<planner::StreamState>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryDelta {
    pub previous_snapshot_id: Option<u64>,
    pub snapshot_id: u64,
    pub appeared_applications: Vec<ApplicationId>,
    pub disappeared_applications: Vec<ApplicationId>,
    pub streams: Vec<StreamChange>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleError {
    InvalidInventory(InventoryError),
    OutOfOrderSnapshot {
        previous: u64,
        received: u64,
    },
    StreamOwnerChanged {
        stream_id: StreamId,
        previous: ApplicationId,
        current: ApplicationId,
    },
    NoSnapshot,
    PlanSnapshotMismatch {
        planned: u64,
        current: u64,
    },
}
impl fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "inventory lifecycle: {self:?}")
    }
}
impl std::error::Error for LifecycleError {}

#[derive(Debug, Default)]
pub struct InventoryTracker {
    current: Option<Inventory>,
}
fn streams(inventory: &Inventory) -> BTreeMap<StreamId, (ApplicationId, Stream)> {
    inventory
        .applications
        .iter()
        .flat_map(|app| {
            app.streams
                .iter()
                .map(|stream| (stream.id.clone(), (app.id.clone(), stream.clone())))
        })
        .collect()
}
impl InventoryTracker {
    pub fn current(&self) -> Option<&Inventory> {
        self.current.as_ref()
    }

    /// Caller supplies complete snapshots from one identity/session domain.
    /// Versions must strictly increase here; failures preserve the last snapshot.
    /// No name binding, history polling, OS reads or route writes occur.
    pub fn advance(&mut self, next: &Inventory) -> Result<InventoryDelta, LifecycleError> {
        planner::plan(next, &[]).map_err(LifecycleError::InvalidInventory)?;
        if let Some(previous) = &self.current {
            if next.snapshot_id <= previous.snapshot_id {
                return Err(LifecycleError::OutOfOrderSnapshot {
                    previous: previous.snapshot_id,
                    received: next.snapshot_id,
                });
            }
        }
        let previous_id = self.current.as_ref().map(|inventory| inventory.snapshot_id);
        let previous_apps: BTreeSet<_> = self
            .current
            .as_ref()
            .map(|inventory| {
                inventory
                    .applications
                    .iter()
                    .map(|app| app.id.clone())
                    .collect()
            })
            .unwrap_or_default();
        let next_apps: BTreeSet<_> = next.applications.iter().map(|app| app.id.clone()).collect();
        let previous_streams = self.current.as_ref().map(streams).unwrap_or_default();
        let next_streams = streams(next);
        let ids: BTreeSet<_> = previous_streams
            .keys()
            .chain(next_streams.keys())
            .cloned()
            .collect();
        let mut changes = Vec::new();
        for id in ids {
            let before = previous_streams.get(&id);
            let after = next_streams.get(&id);
            match (before, after) {
                (Some((old_owner, _)), Some((new_owner, _))) if old_owner != new_owner => {
                    return Err(LifecycleError::StreamOwnerChanged {
                        stream_id: id,
                        previous: old_owner.clone(),
                        current: new_owner.clone(),
                    });
                }
                (Some((_, old)), Some((owner, new))) if old.state != new.state => {
                    changes.push(StreamChange {
                        application_id: owner.clone(),
                        stream: new.clone(),
                        previous_state: Some(old.state),
                    });
                }
                (None, Some((owner, new))) => changes.push(StreamChange {
                    application_id: owner.clone(),
                    stream: new.clone(),
                    previous_state: None,
                }),
                (Some((owner, old)), None) => changes.push(StreamChange {
                    application_id: owner.clone(),
                    stream: Stream {
                        id: old.id.clone(),
                        state: planner::StreamState::Disappeared,
                    },
                    previous_state: Some(old.state),
                }),
                _ => {}
            }
        }
        let delta = InventoryDelta {
            previous_snapshot_id: previous_id,
            snapshot_id: next.snapshot_id,
            appeared_applications: next_apps.difference(&previous_apps).cloned().collect(),
            disappeared_applications: previous_apps.difference(&next_apps).cloned().collect(),
            streams: changes,
        };
        // Missing streams are delta tombstones only; never mixed into the new
        // inventory, which would collide with explicitly rebound reopened apps.
        self.current = Some(next.clone());
        Ok(delta)
    }

    /// Checks only a plan's generation against the latest accepted inventory.
    /// This does not authenticate a plan or authorize applying any route.
    pub fn require_current_snapshot(&self, plan: &Plan) -> Result<(), LifecycleError> {
        let current = self.current.as_ref().ok_or(LifecycleError::NoSnapshot)?;
        if plan.snapshot_id != current.snapshot_id {
            return Err(LifecycleError::PlanSnapshotMismatch {
                planned: plan.snapshot_id,
                current: current.snapshot_id,
            });
        }
        Ok(())
    }
}
