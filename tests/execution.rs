use beacn_link_protocol::{
    execution::*, executor::*, graph_snapshot::preparation_from_tsv, lifecycle::InventoryTracker,
    planner, receipts::*, Assignment, Record,
};
use std::collections::VecDeque;

fn setup() -> (Observation, Ownership) {
    let mut text = "inventory\tapp\tgame\ts\tlive\nowner\tours\n".to_owned();
    for side in ["expected", "observed"] {
        for row in [
            "core\tpw-core:123",
            "node\t10\t100",
            "node\t20\t200",
            "node\t30\t300",
            "port\t11\t110\t10\t100\tout\tFL",
            "port\t12\t120\t10\t100\tout\tFR",
            "port\t21\t210\t20\t200\tin\tFL",
            "port\t22\t220\t20\t200\tin\tFR",
            "port\t31\t310\t30\t300\tin\tFL",
            "port\t32\t320\t30\t300\tin\tFR",
            "stream\tapp\ts\t10\t100",
        ] {
            text.push_str(&format!("{side}\t{row}\n"));
        }
    }
    let data = preparation_from_tsv(7, &text).unwrap();
    (
        Observation {
            graph: data.observed,
            inventory: data.inventory,
        },
        data.ownership,
    )
}
fn prepared(observation: &Observation, ownership: &Ownership, assignment: u8) -> PreparedChange {
    let mut tracker = InventoryTracker::default();
    tracker.advance(&observation.inventory).unwrap();
    let plan = planner::plan(
        &observation.inventory,
        &[Record {
            assignment: Assignment::from_byte(assignment),
            name: b"game".to_vec(),
        }],
    )
    .unwrap();
    prepare(
        &tracker,
        &plan,
        &observation.graph,
        &observation.graph,
        &[Destination {
            assignment,
            node: observation.graph.nodes[assignment as usize],
        }],
        ownership,
    )
    .unwrap()
}
struct Fake {
    observation: Observation,
    effects: VecDeque<u8>,
    pending: Option<Acknowledgment>,
    unresolved: bool,
    calls: usize,
    serial: u64,
}
impl Fake {
    fn new(observation: Observation) -> Self {
        Self {
            observation,
            effects: VecDeque::new(),
            pending: None,
            unresolved: false,
            calls: 0,
            serial: 1000,
        }
    }
}
impl Backend for Fake {
    fn observe(&mut self, _: u64) -> Result<Observation, String> {
        Ok(self.observation.clone())
    }
    fn perform(&mut self, operation: &Operation) -> Effect {
        self.calls += 1;
        let action = self.effects.pop_front().unwrap_or(0);
        if action == 1 {
            return Effect::Failed("controlled no effect".into());
        }
        let ack = match operation {
            Operation::Connect {
                output,
                input,
                owner,
            } => {
                self.serial += 1;
                let link = ExistingLink {
                    object: ObjectRef {
                        id: 40 + (self.serial % 2) as u32,
                        serial: self.serial,
                    },
                    output: output.object,
                    input: input.object,
                    owner: Some(owner.clone()),
                };
                self.observation.graph.links.push(link.clone());
                Acknowledgment::Created(link)
            }
            Operation::Disconnect { link, .. } => {
                self.observation
                    .graph
                    .links
                    .retain(|l| l.object != link.object);
                Acknowledgment::Disconnected(link.object)
            }
        };
        if action == 2 {
            self.pending = Some(ack);
            Effect::Unknown("controlled lost acknowledgment".into())
        } else {
            Effect::Acknowledged(ack)
        }
    }
    fn reconcile(&mut self) -> Result<Option<Acknowledgment>, String> {
        if self.unresolved {
            Err("still unknown".into())
        } else {
            Ok(self.pending.take())
        }
    }
}
#[test]
fn forward_move_and_recovery_use_actual_receipts_and_new_restoration_serials() {
    let (o, own) = setup();
    let mut backend = Fake::new(o.clone());
    let mut a = Execution::new(prepared(&o, &own, 1), &o, own).unwrap();
    a.run(&mut backend).unwrap();
    let before = backend.observation.clone();
    let before_links = before.graph.links.clone();
    let mut b = Execution::new(
        prepared(&before, a.journal().ownership(), 2),
        &before,
        a.journal().ownership().clone(),
    )
    .unwrap();
    b.run(&mut backend).unwrap();
    assert_eq!(b.journal().prefix(), 4);
    assert!(backend
        .observation
        .graph
        .links
        .iter()
        .all(|l| l.input.id == 31 || l.input.id == 32));
    b.rollback(&mut backend).unwrap();
    assert_eq!(
        b.recovery_journal().unwrap().state(),
        JournalState::Complete
    );
    for old in before_links {
        let restored = backend
            .observation
            .graph
            .links
            .iter()
            .find(|l| l.output == old.output)
            .unwrap();
        assert_eq!(restored.input, old.input);
        assert_ne!(restored.object.serial, old.object.serial);
    }
    assert!(b.rollback(&mut backend).is_err());
}
#[test]
fn partial_failure_restores_only_known_prefix_and_preserves_foreign_links() {
    let (o, own) = setup();
    let mut backend = Fake::new(o.clone());
    backend.effects = VecDeque::from([0, 1]);
    let unrelated_node = ObjectRef {
        id: 80,
        serial: 800,
    };
    let unrelated_port = ObjectRef {
        id: 81,
        serial: 810,
    };
    backend.observation.graph.nodes.push(unrelated_node);
    backend.observation.graph.ports.push(Port {
        object: unrelated_port,
        node: unrelated_node,
        direction: Direction::Output,
        channel: Channel::Left,
    });
    let foreign = ExistingLink {
        object: ObjectRef {
            id: 90,
            serial: 900,
        },
        output: unrelated_port,
        input: o.graph.ports[3].object,
        owner: None,
    };
    backend.observation.graph.links.push(foreign.clone());
    let initial = backend.observation.clone();
    let mut run = Execution::new(prepared(&initial, &own, 1), &initial, own).unwrap();
    assert!(run.run(&mut backend).is_err());
    assert_eq!(run.journal().prefix(), 1);
    assert_eq!(run.journal().state(), JournalState::Failed);
    assert!(run.run(&mut backend).is_err());
    assert_eq!(backend.calls, 2);
    run.rollback(&mut backend).unwrap();
    assert_eq!(backend.observation.graph.links, vec![foreign]);
}
#[test]
fn unknown_effect_blocks_progress_and_recovery_until_reconciled() {
    let (o, own) = setup();
    let mut backend = Fake::new(o.clone());
    backend.effects = VecDeque::from([2]);
    let mut run = Execution::new(prepared(&o, &own, 1), &o, own).unwrap();
    assert!(run.run(&mut backend).is_err());
    assert_eq!(run.journal().prefix(), 0);
    assert!(run.rollback(&mut backend).is_err());
    backend.unresolved = true;
    assert!(run.reconcile(&mut backend).is_err());
    assert_eq!(run.journal().state(), JournalState::Unknown);
    backend.unresolved = false;
    run.reconcile(&mut backend).unwrap();
    assert_eq!(run.journal().prefix(), 1);
    assert_eq!(run.journal().state(), JournalState::Failed);
    run.rollback(&mut backend).unwrap();
    assert!(backend.observation.graph.links.is_empty());
}
#[test]
fn identity_reuse_and_inventory_change_refuse_without_native_delete() {
    let (o, own) = setup();
    let mut backend = Fake::new(o.clone());
    let mut run = Execution::new(prepared(&o, &own, 1), &o, own).unwrap();
    run.run(&mut backend).unwrap();
    backend.observation.graph.links[0].object.serial += 10;
    assert!(run.rollback(&mut backend).is_err());
    assert_eq!(backend.calls, 2);
    let (o, own) = setup();
    let mut backend = Fake::new(o.clone());
    let mut run = Execution::new(prepared(&o, &own, 1), &o, own).unwrap();
    backend.observation.inventory.applications[0].streams[0].state = planner::StreamState::Stale;
    assert!(run.run(&mut backend).is_err());
    assert_eq!(backend.calls, 0);
    assert_eq!(run.journal().state(), JournalState::Failed);
}
#[test]
fn failed_recovery_keeps_its_prefix_and_cannot_be_restarted() {
    let (o, own) = setup();
    let mut backend = Fake::new(o.clone());
    let mut run = Execution::new(prepared(&o, &own, 1), &o, own).unwrap();
    run.run(&mut backend).unwrap();
    backend.effects = VecDeque::from([0, 2]);
    assert!(run.rollback(&mut backend).is_err());
    assert_eq!(run.recovery_journal().unwrap().prefix(), 1);
    assert_eq!(
        run.recovery_journal().unwrap().state(),
        JournalState::Unknown
    );
    assert!(run.rollback(&mut backend).is_err());
    run.reconcile(&mut backend).unwrap();
    assert_eq!(run.recovery_journal().unwrap().prefix(), 2);
    assert_eq!(
        run.recovery_journal().unwrap().state(),
        JournalState::Failed
    );
    assert!(backend.observation.graph.links.is_empty());
}
