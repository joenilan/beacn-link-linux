//! Entirely synthetic inventory and an existing handwritten wire fixture.
use beacn_link_protocol::planner::*;
use beacn_link_protocol::{decode_frame, decode_records, Frame};
fn main() {
    let fixture = include_str!("../fixtures/ascii.frame.hex").trim();
    let bytes: Vec<_> = (0..fixture.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&fixture[i..i + 2], 16).unwrap())
        .collect();
    let Frame::Data(payload) = decode_frame(&bytes).unwrap() else {
        panic!("data expected")
    };
    let inventory = Inventory {
        snapshot_id: 7,
        applications: vec![Application {
            id: ApplicationId("synthetic-game".into()),
            advertised_name: b"game".to_vec(),
            streams: vec![
                Stream {
                    id: StreamId("stream-b".into()),
                    state: StreamState::Live,
                },
                Stream {
                    id: StreamId("stream-a".into()),
                    state: StreamState::Live,
                },
            ],
        }],
    };
    let result = plan(&inventory, &decode_records(&payload).unwrap()).unwrap();
    println!("{result:#?}");
}
