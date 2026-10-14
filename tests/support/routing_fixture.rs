use beacn_link_protocol::{
    catalog::{self, ApplicationService},
    execution::Observation,
    executor::*,
    planner::ApplicationId,
    session::PollReply,
    Assignment, Record,
};
use std::collections::BTreeMap;

pub const ROWS: &str = "app\t10\t123\t20\tlive\t6d7076\t6d7076\napp\t10\t123\t22\tlive\t6d7076\t6d7076\napp\t11\t124\t21\tlive\t67616d65\t67616d65\n";
pub fn catalog(generation: u64, rows: &str) -> catalog::Catalog {
    catalog::parse(
        generation,
        &format!("catalog\t42\t99\t70697065776972652d30\n{rows}"),
    )
    .unwrap()
}
pub fn id(n: u8) -> ApplicationId {
    ApplicationId(format!("pw-client:{}:pid:{}", 10 + n, 123 + n))
}
pub fn assignments(a: u8, b: u8) -> BTreeMap<ApplicationId, u8> {
    [(id(0), a), (id(1), b)].into_iter().collect()
}
pub fn reply(a: u8, b: u8) -> PollReply {
    PollReply::Records {
        records: vec![
            Record {
                name: b"Player\0opaque".to_vec(),
                assignment: Assignment::from_byte(a),
            },
            Record {
                name: b"Game".to_vec(),
                assignment: Assignment::from_byte(b),
            },
        ],
        ignored_tail: 0,
    }
}
pub fn setup() -> (ApplicationService, Observation, Vec<Destination>, Ownership) {
    let mut service = ApplicationService::default();
    service.update(catalog(1, ROWS)).unwrap();
    service.select(&id(0), "Player").unwrap();
    service.select(&id(1), "Game").unwrap();
    let inventory = service.inventory().unwrap().clone();
    let mut graph = GraphSnapshot {
        generation: inventory.snapshot_id,
        core_id: "pw-core:42".into(),
        nodes: vec![],
        ports: vec![],
        streams: vec![],
        links: vec![],
    };
    let mut next = 10;
    for app in &inventory.applications {
        for stream in &app.streams {
            let node = ObjectRef {
                id: next,
                serial: stream
                    .id
                    .0
                    .strip_prefix("pw-node:")
                    .unwrap()
                    .parse()
                    .unwrap(),
            };
            graph.nodes.push(node);
            graph.streams.push(StreamBinding {
                application_id: app.id.clone(),
                stream_id: stream.id.clone(),
                node,
            });
            for (offset, channel) in [(1, Channel::Left), (2, Channel::Right)] {
                graph.ports.push(Port {
                    object: ObjectRef {
                        id: next + offset,
                        serial: 10000 + (next + offset) as u64,
                    },
                    node,
                    direction: Direction::Output,
                    channel,
                });
            }
            next += 10;
        }
    }
    let mut destinations = vec![];
    for assignment in 1..=4 {
        let node = ObjectRef {
            id: 100 + assignment as u32 * 10,
            serial: 1000 + assignment as u64,
        };
        graph.nodes.push(node);
        for (offset, channel) in [(1, Channel::Left), (2, Channel::Right)] {
            graph.ports.push(Port {
                object: ObjectRef {
                    id: node.id + offset,
                    serial: 10000 + (node.id + offset) as u64,
                },
                node,
                direction: Direction::Input,
                channel,
            });
        }
        destinations.push(Destination { assignment, node });
    }
    (
        service,
        Observation { inventory, graph },
        destinations,
        Ownership {
            token: "ours".into(),
            links: Default::default(),
        },
    )
}
