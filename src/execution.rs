//! Execution orchestration over fresh provider evidence and the existing journal.
use crate::{executor::*, lifecycle::InventoryTracker, planner::Inventory, receipts::*};

#[derive(Debug, Clone)]
pub struct Observation {
    pub graph: GraphSnapshot,
    pub inventory: Inventory,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Acknowledged(Acknowledgment),
    /// Provider proves no request was issued / no effect occurred.
    Failed(String),
    /// A request might have taken effect; no retry or recovery until reconciled.
    Unknown(String),
}
pub trait Backend {
    fn observe(&mut self, generation: u64) -> Result<Observation, String>;
    fn perform(&mut self, operation: &Operation) -> Effect;
    /// Ok(None) proves the pending request had no effect. Err remains unknown.
    fn reconcile(&mut self) -> Result<Option<Acknowledgment>, String>;
}

pub struct Execution {
    prepared: PreparedChange,
    initial_inventory: Inventory,
    journal: ReceiptJournal,
    recovery: Option<(PreparedChange, ReceiptJournal)>,
}
impl Execution {
    pub fn new(
        prepared: PreparedChange,
        observation: &Observation,
        ownership: Ownership,
    ) -> Result<Self, String> {
        if observation.inventory.snapshot_id != prepared.generation {
            return Err("inventory generation mismatch".into());
        }
        let journal = ReceiptJournal::new(prepared.clone(), &observation.graph, ownership)
            .map_err(|e| format!("{e:?}"))?;
        Ok(Self {
            prepared,
            initial_inventory: observation.inventory.clone(),
            journal,
            recovery: None,
        })
    }
    pub fn journal(&self) -> &ReceiptJournal {
        &self.journal
    }
    pub fn recovery_journal(&self) -> Option<&ReceiptJournal> {
        self.recovery.as_ref().map(|r| &r.1)
    }
    fn fresh(&self, backend: &mut impl Backend) -> Result<(Observation, InventoryTracker), String> {
        let observation = backend.observe(self.prepared.generation)?;
        if observation.inventory != self.initial_inventory {
            return Err("bound inventory changed; stop and replan".into());
        }
        let mut tracker = InventoryTracker::default();
        tracker
            .advance(&observation.inventory)
            .map_err(|e| e.to_string())?;
        Ok((observation, tracker))
    }
    pub fn run(&mut self, backend: &mut impl Backend) -> Result<(), String> {
        if self.recovery.is_some() {
            return Err("recovery already started".into());
        }
        while self.journal.state() == JournalState::Ready {
            let (observation, tracker) = match self.fresh(backend) {
                Ok(value) => value,
                Err(error) => {
                    self.journal.stop(self.journal.prefix(), false).unwrap();
                    return Err(error);
                }
            };
            Self::execute_step(
                backend,
                &self.prepared,
                &mut self.journal,
                &tracker,
                &observation.graph,
            )?;
        }
        if self.journal.state() == JournalState::Complete {
            Ok(())
        } else {
            Err("execution stopped".into())
        }
    }
    fn execute_step(
        backend: &mut impl Backend,
        prepared: &PreparedChange,
        journal: &mut ReceiptJournal,
        tracker: &InventoryTracker,
        graph: &GraphSnapshot,
    ) -> Result<(), String> {
        let index = journal.prefix();
        if let Err(error) = validate_step(prepared, index, tracker, graph, journal.ownership()) {
            journal.stop(index, false).unwrap();
            return Err(error.to_string());
        }
        match backend.perform(&prepared.steps[index].operation) {
            Effect::Acknowledged(ack) => match journal.acknowledge(index, ack) {
                Ok(()) => Ok(()),
                Err(error) => {
                    journal.stop(index, true).unwrap();
                    Err(format!(
                        "rejected acknowledgment; unknown effect: {error:?}"
                    ))
                }
            },
            Effect::Failed(reason) => {
                journal.stop(index, false).unwrap();
                Err(reason)
            }
            Effect::Unknown(reason) => {
                journal.stop(index, true).unwrap();
                Err(reason)
            }
        }
    }
    pub fn reconcile(&mut self, backend: &mut impl Backend) -> Result<(), String> {
        let journal = self
            .recovery
            .as_mut()
            .map(|r| &mut r.1)
            .unwrap_or(&mut self.journal);
        if journal.state() != JournalState::Unknown {
            return Err("no unknown pending effect".into());
        }
        journal
            .reconcile(backend.reconcile()?)
            .map_err(|e| format!("{e:?}"))
    }
    /// Explicit recovery of the known prefix. It uses actual create receipts and
    /// the same fresh validation and native acknowledgment path as forward work.
    /// A failed/unknown recovery is retained, never silently retried.
    pub fn rollback(&mut self, backend: &mut impl Backend) -> Result<(), String> {
        if self.journal.state() == JournalState::Unknown {
            return Err("pending effect must be reconciled before rollback".into());
        }
        if self.recovery.is_some() {
            return Err("recovery already attempted; inspect its journal".into());
        }
        let (observation, _) = self.fresh(backend)?;
        let steps = self
            .journal
            .rollback()
            .into_iter()
            .enumerate()
            .map(|(index, undo)| {
                let original = &self.prepared.steps[self.journal.prefix() - index - 1];
                let operation = match undo {
                    ReceiptUndo::Restore {
                        previous,
                        output,
                        input,
                    } => Operation::Connect {
                        output,
                        input,
                        owner: previous.owner.unwrap_or_default(),
                    },
                    ReceiptUndo::RemoveCreated(link) => {
                        let Operation::Connect { output, input, .. } = &original.operation else {
                            unreachable!()
                        };
                        Operation::Disconnect {
                            link,
                            output: output.clone(),
                            input: input.clone(),
                        }
                    }
                };
                GuardedStep {
                    stream: original.stream.clone(),
                    operation,
                    undo: original.undo.clone(),
                }
            })
            .collect();
        let recovery = PreparedChange {
            generation: self.prepared.generation,
            core_id: self.prepared.core_id.clone(),
            steps,
        };
        let journal = ReceiptJournal::new(
            recovery.clone(),
            &observation.graph,
            self.journal.ownership().clone(),
        )
        .map_err(|e| format!("{e:?}"))?;
        self.recovery = Some((recovery, journal));
        loop {
            let state = self.recovery.as_ref().unwrap().1.state();
            if state != JournalState::Ready {
                return if state == JournalState::Complete {
                    Ok(())
                } else {
                    Err("recovery stopped".into())
                };
            }
            let (observation, tracker) = match self.fresh(backend) {
                Ok(value) => value,
                Err(error) => {
                    let journal = &mut self.recovery.as_mut().unwrap().1;
                    journal.stop(journal.prefix(), false).unwrap();
                    return Err(error);
                }
            };
            let (prepared, journal) = self.recovery.as_mut().unwrap();
            Self::execute_step(backend, prepared, journal, &tracker, &observation.graph)?;
        }
    }
}
