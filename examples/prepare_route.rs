//! Synthetic dry-run example: no discovery, device access or route application.
use beacn_link_protocol::executor::*;
use beacn_link_protocol::lifecycle::InventoryTracker;
use beacn_link_protocol::planner::*;
use beacn_link_protocol::{Assignment, Record};
use std::collections::BTreeSet;

fn object(id: u32) -> ObjectRef {
    ObjectRef {
        id,
        serial: u64::from(id) * 10,
    }
}
fn port(id: u32, node: u32, direction: Direction, channel: Channel) -> Port {
    Port {
        object: object(id),
        node: object(node),
        direction,
        channel,
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = ApplicationId("synthetic-app".into());
    let stream = StreamId("synthetic-stream".into());
    let inventory = Inventory {
        snapshot_id: 1,
        applications: vec![Application {
            id: app.clone(),
            advertised_name: b"game".to_vec(),
            streams: vec![Stream {
                id: stream.clone(),
                state: StreamState::Live,
            }],
        }],
    };
    let mut tracker = InventoryTracker::default();
    tracker.advance(&inventory)?;
    let proposed = plan(
        &inventory,
        &[Record {
            name: b"game".to_vec(),
            assignment: Assignment::from_byte(2),
        }],
    )?;
    let graph = GraphSnapshot {
        generation: 1,
        core_id: "synthetic-core".into(),
        nodes: vec![object(10), object(20), object(30)],
        ports: vec![
            port(11, 10, Direction::Output, Channel::Left),
            port(12, 10, Direction::Output, Channel::Right),
            port(21, 20, Direction::Input, Channel::Left),
            port(22, 20, Direction::Input, Channel::Right),
            port(31, 30, Direction::Input, Channel::Left),
            port(32, 30, Direction::Input, Channel::Right),
        ],
        streams: vec![StreamBinding {
            application_id: app,
            stream_id: stream,
            node: object(10),
        }],
        links: vec![
            ExistingLink {
                object: object(40),
                output: object(11),
                input: object(21),
                owner: Some("demo".into()),
            },
            ExistingLink {
                object: object(41),
                output: object(12),
                input: object(22),
                owner: Some("demo".into()),
            },
        ],
    };
    let ownership = Ownership {
        token: "demo".into(),
        links: BTreeSet::from([object(40), object(41)]),
    };
    let prepared = prepare(
        &tracker,
        &proposed,
        &graph,
        &graph,
        &[Destination {
            assignment: 2,
            node: object(30),
        }],
        &ownership,
    )?;
    println!(
        "DRY RUN: {} guarded operations; actual application disabled",
        prepared.steps.len()
    );
    for (index, step) in prepared.steps.iter().enumerate() {
        println!("{index}: {:?}; undo {:?}", step.operation, step.undo);
    }
    println!(
        "Rollback after 3 acknowledged steps: {} inverse descriptions",
        prepared.rollback_for_prefix(3)?.len()
    );
    Ok(())
}
