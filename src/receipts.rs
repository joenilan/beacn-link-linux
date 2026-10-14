//! Pure bookkeeping for ordered native acknowledgments. No native backend.
use crate::executor::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReleaseReport {
    pub acknowledged_links: usize,
    pub observed_links: usize,
    pub already_absent: usize,
}
/// Verify independent observations surrounding release of retained owned handles.
/// Pure comparison, never deletion authority. Missing old receipts are allowed;
/// same numeric IDs with different serials and every unrelated object must remain.
pub fn verify_release(
    before: &GraphSnapshot,
    after: &GraphSnapshot,
    ownership: &Ownership,
) -> Result<ReleaseReport, String> {
    validate_graph(before).map_err(|e| e.to_string())?;
    validate_graph(after).map_err(|e| e.to_string())?;
    if ownership.links.len() > MAX_LINKS
        || ownership.token.is_empty()
        || ownership.token.len() > 128
        || !ownership.token.bytes().all(|b| (0x20..=0x7e).contains(&b))
    {
        return Err("invalid release ledger".into());
    }
    let observed = before
        .links
        .iter()
        .filter(|l| ownership.links.contains(&l.object))
        .count();
    if before.links.iter().any(|l| {
        ownership.links.contains(&l.object) && l.owner.as_deref() != Some(&ownership.token)
    }) {
        return Err("receipt object changed owner before release".into());
    }
    let mut expected = before.clone();
    expected
        .links
        .retain(|l| !ownership.links.contains(&l.object));
    if &expected != after {
        return Err("owned release could not verify the remaining graph exactly".into());
    }
    Ok(ReleaseReport {
        acknowledged_links: ownership.links.len(),
        observed_links: observed,
        already_absent: ownership.links.len() - observed,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Acknowledgment {
    Disconnected(ObjectRef),
    Created(ExistingLink),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalState {
    Ready,
    Complete,
    Failed,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiptError {
    InitialGraph,
    Stopped,
    OutOfOrder,
    Mismatch,
    IdentityCollision,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiptUndo {
    Restore {
        previous: ExistingLink,
        output: Port,
        input: Port,
    },
    RemoveCreated(ExistingLink),
}
#[derive(Debug, Clone)]
pub struct ReceiptJournal {
    prepared: PreparedChange,
    prefix: usize,
    state: JournalState,
    ownership: Ownership,
    occupied: BTreeMap<u32, u64>,
    serials: BTreeSet<u64>,
    created: BTreeMap<usize, ExistingLink>,
}
impl ReceiptJournal {
    /// Input must be an unmodified preparation and its fresh pre-operation graph.
    /// Acknowledgments are trusted caller events, not proof or authorization.
    pub fn new(
        prepared: PreparedChange,
        graph: &GraphSnapshot,
        ownership: Ownership,
    ) -> Result<Self, ReceiptError> {
        if validate_graph(graph).is_err()
            || graph.generation != prepared.generation
            || graph.core_id != prepared.core_id
            || prepared.steps.len() > MAX_LINKS
            || ownership.token.is_empty()
            || ownership.token.len() > 128
            || !ownership.token.bytes().all(|b| (0x20..=0x7e).contains(&b))
            || ownership.links.len() > MAX_LINKS
        {
            return Err(ReceiptError::InitialGraph);
        }
        for receipt in &ownership.links {
            if !graph
                .links
                .iter()
                .any(|l| l.object == *receipt && l.owner.as_deref() == Some(&ownership.token))
            {
                return Err(ReceiptError::InitialGraph);
            }
        }
        let mut occupied = BTreeMap::new();
        let mut serials = BTreeSet::new();
        for o in graph
            .nodes
            .iter()
            .chain(graph.ports.iter().map(|p| &p.object))
            .chain(graph.links.iter().map(|l| &l.object))
        {
            occupied.insert(o.id, o.serial);
            serials.insert(o.serial);
        }
        let state = if prepared.steps.is_empty() {
            JournalState::Complete
        } else {
            JournalState::Ready
        };
        Ok(Self {
            prepared,
            prefix: 0,
            state,
            ownership,
            occupied,
            serials,
            created: BTreeMap::new(),
        })
    }
    pub fn prefix(&self) -> usize {
        self.prefix
    }
    pub fn state(&self) -> JournalState {
        self.state
    }
    pub fn ownership(&self) -> &Ownership {
        &self.ownership
    }
    fn next(&self, step: usize) -> Result<(), ReceiptError> {
        if self.state != JournalState::Ready {
            return Err(ReceiptError::Stopped);
        }
        if step != self.prefix {
            return Err(ReceiptError::OutOfOrder);
        }
        Ok(())
    }
    /// Rejected acknowledgments leave every field unchanged.
    pub fn acknowledge(&mut self, step: usize, ack: Acknowledgment) -> Result<(), ReceiptError> {
        self.next(step)?;
        match (&self.prepared.steps[step].operation, ack) {
            (Operation::Disconnect { link, .. }, Acknowledgment::Disconnected(object)) => {
                if object != link.object
                    || self.occupied.get(&object.id) != Some(&object.serial)
                    || link.owner.as_deref() != Some(&self.ownership.token)
                    || !self.ownership.links.contains(&object)
                {
                    return Err(ReceiptError::Mismatch);
                }
                self.occupied.remove(&object.id);
                self.ownership.links.remove(&object);
            }
            (
                Operation::Connect {
                    output,
                    input,
                    owner,
                },
                Acknowledgment::Created(link),
            ) => {
                if link.output != output.object
                    || link.input != input.object
                    || link.owner.as_deref() != Some(owner)
                    || owner != &self.ownership.token
                    || link.object.id == 0
                    || link.object.serial == 0
                {
                    return Err(ReceiptError::Mismatch);
                }
                if self.occupied.contains_key(&link.object.id)
                    || self.serials.contains(&link.object.serial)
                {
                    return Err(ReceiptError::IdentityCollision);
                }
                self.occupied.insert(link.object.id, link.object.serial);
                self.serials.insert(link.object.serial);
                self.ownership.links.insert(link.object);
                self.created.insert(step, link);
            }
            _ => return Err(ReceiptError::Mismatch),
        }
        self.prefix += 1;
        if self.prefix == self.prepared.steps.len() {
            self.state = JournalState::Complete;
        }
        Ok(())
    }
    /// Failure and unknown outcomes stop progression without acknowledging success.
    pub fn stop(&mut self, step: usize, unknown: bool) -> Result<(), ReceiptError> {
        self.next(step)?;
        self.state = if unknown {
            JournalState::Unknown
        } else {
            JournalState::Failed
        };
        Ok(())
    }
    /// Only a backend's fresh reconciliation may settle an unknown pending effect.
    /// Some acknowledges that exact effect; None proves none survives. Neither
    /// permits forward continuation. Rejected evidence preserves the unknown state.
    pub fn reconcile(&mut self, ack: Option<Acknowledgment>) -> Result<(), ReceiptError> {
        if self.state != JournalState::Unknown {
            return Err(ReceiptError::Stopped);
        }
        if let Some(ack) = ack {
            let mut settled = self.clone();
            settled.state = JournalState::Ready;
            settled.acknowledge(settled.prefix, ack)?;
            settled.state = JournalState::Failed;
            *self = settled;
        } else {
            self.state = JournalState::Failed;
        }
        Ok(())
    }
    /// Only Execution's explicitly requested, freshly guarded recovery may resume.
    /// Forward journals never call this. Retain prefix, receipts and serial history.
    pub(crate) fn resume_recovery(&mut self) -> Result<(), ReceiptError> {
        if self.state != JournalState::Failed {
            return Err(ReceiptError::Stopped);
        }
        self.state = if self.prefix == self.prepared.steps.len() {
            JournalState::Complete
        } else {
            JournalState::Ready
        };
        Ok(())
    }
    /// Describes only known successful work. Unknown pending effects are excluded;
    /// callers must reconcile them before any real recovery operation.
    pub fn rollback(&self) -> Vec<ReceiptUndo> {
        (0..self.prefix)
            .rev()
            .map(|step| match &self.prepared.steps[step].operation {
                Operation::Disconnect {
                    link,
                    output,
                    input,
                } => ReceiptUndo::Restore {
                    previous: link.clone(),
                    output: output.clone(),
                    input: input.clone(),
                },
                Operation::Connect { .. } => {
                    ReceiptUndo::RemoveCreated(self.created[&step].clone())
                }
            })
            .collect()
    }
}
