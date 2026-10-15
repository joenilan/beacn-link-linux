#![allow(dead_code)]
use beacn_link_protocol::{
    execution::{Backend, Effect, Observation},
    executor::*,
    receipts::{verify_release, Acknowledgment},
    routing_control::{CleanupReport, OwnedRoutingBackend},
};
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};
pub struct State {
    pub observation: Observation,
    pub handles: BTreeSet<ObjectRef>,
    pub calls: usize,
    pub stops: usize,
    pub fail_at: Option<usize>,
    pub unknown_at: Option<usize>,
    pub observation_fails: bool,
    pub cleanup_fails: bool,
}
pub struct Fake {
    state: Rc<RefCell<State>>,
    ownership: Ownership,
    next: u32,
}
pub fn backend(observation: Observation, ownership: Ownership) -> (Fake, Rc<RefCell<State>>) {
    let state = Rc::new(RefCell::new(State {
        observation,
        handles: BTreeSet::new(),
        calls: 0,
        stops: 0,
        fail_at: None,
        unknown_at: None,
        observation_fails: false,
        cleanup_fails: false,
    }));
    (
        Fake {
            state: state.clone(),
            ownership,
            next: 3000,
        },
        state,
    )
}
impl Backend for Fake {
    fn observe(&mut self, generation: u64) -> Result<Observation, String> {
        let mut state = self.state.borrow_mut();
        if state.observation_fails {
            return Err("controlled observation failure".into());
        }
        state.observation.graph.generation = generation;
        state.observation.inventory.snapshot_id = generation;
        Ok(state.observation.clone())
    }
    fn perform(&mut self, operation: &Operation) -> Effect {
        let mut state = self.state.borrow_mut();
        state.calls += 1;
        if state.fail_at == Some(state.calls) {
            return Effect::Failed("controlled no effect".into());
        }
        let ack = match operation {
            Operation::Connect {
                output,
                input,
                owner,
            } => {
                self.next += 1;
                let link = ExistingLink {
                    object: ObjectRef {
                        id: self.next,
                        serial: 20000 + self.next as u64,
                    },
                    output: output.object,
                    input: input.object,
                    owner: Some(owner.clone()),
                };
                state.observation.graph.links.push(link.clone());
                state.handles.insert(link.object);
                if state.unknown_at != Some(state.calls) {
                    self.ownership.links.insert(link.object);
                }
                Acknowledgment::Created(link)
            }
            Operation::Disconnect { link, .. } => {
                state
                    .observation
                    .graph
                    .links
                    .retain(|l| l.object != link.object);
                state.handles.remove(&link.object);
                self.ownership.links.remove(&link.object);
                Acknowledgment::Disconnected(link.object)
            }
        };
        if state.unknown_at == Some(state.calls) {
            Effect::Unknown("controlled unacknowledged effect".into())
        } else {
            Effect::Acknowledged(ack)
        }
    }
    fn reconcile(&mut self) -> Result<Option<Acknowledgment>, String> {
        Err("unknown remains unsettled".into())
    }
}
impl OwnedRoutingBackend for Fake {
    fn ownership(&self) -> &Ownership {
        &self.ownership
    }
    fn stop_owned(&mut self) -> Result<CleanupReport, String> {
        let mut state = self.state.borrow_mut();
        state.stops += 1;
        let before = state.observation.graph.clone();
        let handles = state.handles.clone();
        state
            .observation
            .graph
            .links
            .retain(|l| !handles.contains(&l.object));
        state.handles.clear();
        if state.cleanup_fails || state.observation_fails {
            return Err("controlled unverified cleanup after release".into());
        }
        verify_release(&before, &state.observation.graph, &self.ownership)
    }
}
