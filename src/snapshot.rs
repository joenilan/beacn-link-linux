//! Strict, bounded text bridge into the existing pure planner.
use crate::planner::{self, Application, ApplicationId, Inventory, Stream, StreamId, StreamState};
use std::collections::BTreeMap;

pub const MAX_TSV_BYTES: usize = 2 * 1024 * 1024;

fn printable(text: &str, max: usize) -> bool {
    !text.is_empty() && text.len() <= max && text.bytes().all(|b| (0x20..=0x7e).contains(&b))
}

/// Rows are application-id TAB explicit-wire-name TAB stream-id TAB state.
/// No discovery or OS access occurs here; IDs are opaque snapshot-local labels.
pub fn inventory_from_tsv(snapshot_id: u64, text: &str) -> Result<Inventory, String> {
    if text.len() > MAX_TSV_BYTES {
        return Err("inventory exceeds 2 MiB limit".into());
    }
    let mut apps: BTreeMap<String, Application> = BTreeMap::new();
    for (line, row) in text.lines().enumerate() {
        if line >= planner::MAX_STREAMS {
            return Err("inventory exceeds 4096 stream rows".into());
        }
        let fields: Vec<_> = row.split('\t').collect();
        if fields.len() != 4 {
            return Err(format!(
                "inventory line {} requires four tab-separated fields",
                line + 1
            ));
        }
        let (id, name, stream_id) = (fields[0], fields[1], fields[2]);
        if !printable(id, 128) || !printable(name, 255) || !printable(stream_id, 128) {
            return Err(format!(
                "inventory line {} has an invalid ASCII field",
                line + 1
            ));
        }
        let state = match fields[3] {
            "live" => StreamState::Live,
            "stale" => StreamState::Stale,
            "disappeared" => StreamState::Disappeared,
            _ => return Err(format!("inventory line {} has an unknown state", line + 1)),
        };
        if !apps.contains_key(id) && apps.len() >= planner::MAX_APPLICATIONS {
            return Err("inventory exceeds 256 applications".into());
        }
        let app = apps.entry(id.into()).or_insert_with(|| Application {
            id: ApplicationId(id.into()),
            advertised_name: name.as_bytes().to_vec(),
            streams: Vec::new(),
        });
        if app.advertised_name != name.as_bytes() {
            return Err(format!(
                "inventory line {} changes an application's wire name",
                line + 1
            ));
        }
        app.streams.push(Stream {
            id: StreamId(stream_id.into()),
            state,
        });
    }
    let inventory = Inventory {
        snapshot_id,
        applications: apps.into_values().collect(),
    };
    // Reuse core identity ownership/name validation before returning any snapshot.
    planner::plan(&inventory, &[]).map_err(|e| e.to_string())?;
    Ok(inventory)
}
