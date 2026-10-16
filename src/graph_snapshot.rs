//! Strict bounded metadata bridge into the existing executor types. No OS access.
use crate::executor::*;
use crate::planner::{ApplicationId, Inventory, StreamId};
use crate::snapshot;
use std::collections::BTreeSet;

pub const MAX_GRAPH_TSV_BYTES: usize = 8 * 1024 * 1024;
pub struct PreparationInput {
    pub inventory: Inventory,
    pub expected: GraphSnapshot,
    pub observed: GraphSnapshot,
    pub destinations: Vec<Destination>,
    pub ownership: Ownership,
}
fn number<T: std::str::FromStr>(s: &str) -> Result<T, String> {
    if s.is_empty() || s.len() > 20 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err("invalid unsigned integer".into());
    }
    s.parse()
        .map_err(|_| "unsigned integer out of range".into())
}
fn object(id: &str, serial: &str) -> Result<ObjectRef, String> {
    let o = ObjectRef {
        id: number(id)?,
        serial: number(serial)?,
    };
    if o.id == 0 || o.serial == 0 {
        return Err("zero object identity".into());
    }
    Ok(o)
}
fn empty(generation: u64) -> GraphSnapshot {
    GraphSnapshot {
        generation,
        core_id: String::new(),
        nodes: vec![],
        ports: vec![],
        streams: vec![],
        links: vec![],
    }
}
/// All records are caller metadata; graph tags never populate the receipt ledger.
pub fn preparation_from_tsv(generation: u64, text: &str) -> Result<PreparationInput, String> {
    if text.len() > MAX_GRAPH_TSV_BYTES {
        return Err("graph stdin exceeds 8 MiB".into());
    }
    let mut expected = empty(generation);
    let mut observed = empty(generation);
    let mut inventory_rows = String::new();
    let mut inventory_count = 0;
    let mut destinations = vec![];
    let mut token = None;
    let mut receipts = BTreeSet::new();
    for row in text.lines() {
        let f: Vec<_> = row.split('\t').collect();
        if f.len() > 12 {
            return Err("too many graph fields".into());
        }
        match f.first().copied() {
            Some("inventory") if f.len() == 5 => {
                inventory_count += 1;
                if inventory_count > crate::planner::MAX_STREAMS {
                    return Err("too many inventory rows".into());
                }
                inventory_rows.push_str(&f[1..].join("\t"));
                inventory_rows.push('\n');
            }
            Some("destination") if f.len() == 4 => {
                if destinations.len() == 4 {
                    return Err("too many destinations".into());
                }
                destinations.push(Destination {
                    assignment: number(f[1])?,
                    node: object(f[2], f[3])?,
                });
            }
            Some("owner") if f.len() == 2 && token.is_none() => token = Some(f[1].to_owned()),
            Some("receipt") if f.len() == 3 => {
                if receipts.len() == MAX_LINKS || !receipts.insert(object(f[1], f[2])?) {
                    return Err("duplicate or excessive receipt".into());
                }
            }
            Some("expected" | "observed") => {
                let g = if f[0] == "expected" {
                    &mut expected
                } else {
                    &mut observed
                };
                match f.get(1).copied() {
                    Some("core") if f.len() == 3 && g.core_id.is_empty() && !f[2].is_empty() => {
                        g.core_id = f[2].into()
                    }
                    Some("node") if f.len() == 4 && g.nodes.len() < MAX_NODES => {
                        g.nodes.push(object(f[2], f[3])?)
                    }
                    Some("port") if f.len() == 8 && g.ports.len() < MAX_PORTS => {
                        g.ports.push(Port {
                            object: object(f[2], f[3])?,
                            node: object(f[4], f[5])?,
                            direction: match f[6] {
                                "in" => Direction::Input,
                                "out" => Direction::Output,
                                _ => return Err("invalid direction".into()),
                            },
                            channel: match f[7] {
                                "FL" => Channel::Left,
                                "FR" => Channel::Right,
                                "other" => Channel::Other,
                                _ => return Err("invalid channel".into()),
                            },
                        })
                    }
                    Some("stream")
                        if f.len() == 6 && g.streams.len() < crate::planner::MAX_STREAMS =>
                    {
                        g.streams.push(StreamBinding {
                            application_id: ApplicationId(f[2].into()),
                            stream_id: StreamId(f[3].into()),
                            node: object(f[4], f[5])?,
                        })
                    }
                    Some("link") if f.len() == 9 && g.links.len() < MAX_LINKS => {
                        g.links.push(ExistingLink {
                            object: object(f[2], f[3])?,
                            output: object(f[4], f[5])?,
                            input: object(f[6], f[7])?,
                            owner: if f[8].is_empty() {
                                None
                            } else {
                                Some(f[8].into())
                            },
                        })
                    }
                    _ => return Err("invalid, duplicate or excessive graph row".into()),
                }
            }
            _ => return Err("unknown, duplicate or malformed preparation row".into()),
        }
    }
    validate_graph(&expected).map_err(|e| e.to_string())?;
    validate_graph(&observed).map_err(|e| e.to_string())?;
    let token = token.ok_or("missing owner")?;
    if token.is_empty() || token.len() > 128 || !token.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
        return Err("invalid owner".into());
    }
    Ok(PreparationInput {
        inventory: snapshot::inventory_from_tsv(generation, &inventory_rows)?,
        expected,
        observed,
        destinations,
        ownership: Ownership {
            token,
            links: receipts,
        },
    })
}
