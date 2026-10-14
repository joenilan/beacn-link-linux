//! Bounded application metadata and explicit selections. No OS writes or USB.
use crate::{
    lifecycle::{InventoryDelta, InventoryTracker},
    planner::{self, Application, ApplicationId, Inventory, Stream, StreamId, StreamState},
    Assignment, Record,
};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_CATALOG_BYTES: usize = 8 * 1024 * 1024;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogApplication {
    pub id: ApplicationId,
    pub pid: u32,
    pub names: BTreeSet<String>,
    pub binaries: BTreeSet<String>,
    pub streams: Vec<Stream>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalog {
    pub generation: u64,
    pub core: String,
    pub applications: BTreeMap<ApplicationId, CatalogApplication>,
}
impl Catalog {
    fn validate(&self) -> Result<(), String> {
        if self.generation == 0
            || self.core.is_empty()
            || self.core.len() > 256
            || !self.core.bytes().all(|b| (0x20..=0x7e).contains(&b))
            || self.applications.len() > planner::MAX_APPLICATIONS
        {
            return Err("invalid catalog domain/bound".into());
        }
        let mut streams = BTreeSet::new();
        for (id, app) in &self.applications {
            let (client, pid) =
                id.0.strip_prefix("pw-client:")
                    .and_then(|s| s.split_once(":pid:"))
                    .ok_or("invalid catalog application identity")?;
            if number::<u64>(client)? == 0
                || number::<u32>(pid)? != app.pid
                || app.pid == 0
                || app.id != *id
                || app.streams.is_empty()
                || app.names.len() > app.streams.len()
                || app.binaries.len() > app.streams.len()
                || app.names.iter().chain(&app.binaries).any(|s| s.len() > 512)
            {
                return Err("inconsistent catalog application".into());
            }
            for stream in &app.streams {
                let serial = number::<u64>(
                    stream
                        .id
                        .0
                        .strip_prefix("pw-node:")
                        .ok_or("invalid catalog stream identity")?,
                )?;
                if serial == 0
                    || !streams.insert(serial)
                    || streams.len() > planner::MAX_STREAMS
                    || stream.state == StreamState::Disappeared
                {
                    return Err("invalid catalog stream set".into());
                }
            }
        }
        Ok(())
    }
}
fn number<T: std::str::FromStr>(text: &str) -> Result<T, String> {
    if text.is_empty() || text.len() > 20 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err("invalid catalog integer".into());
    }
    text.parse()
        .map_err(|_| "catalog integer out of range".into())
}
fn unhex(text: &str) -> Result<String, String> {
    if text.len() > 1024 || text.len() % 2 != 0 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("invalid bounded catalog label".into());
    }
    let bytes = (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).map_err(|_| "invalid hex".to_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| "catalog label is not UTF-8".into())
}
pub fn parse(generation: u64, text: &str) -> Result<Catalog, String> {
    if generation == 0 || text.len() > MAX_CATALOG_BYTES || !text.ends_with('\n') {
        return Err("catalog size/generation/termination refused".into());
    }
    let mut lines = text.lines();
    let fields: Vec<_> = lines
        .next()
        .ok_or("missing catalog header")?
        .split('\t')
        .collect();
    if fields.len() != 4 || fields[0] != "catalog" {
        return Err("invalid catalog header".into());
    }
    let cookie: u32 = number(fields[1])?;
    let server: u32 = number(fields[2])?;
    let name = unhex(fields[3])?;
    if server == 0
        || name.is_empty()
        || name.len() > 128
        || !name.bytes().all(|b| (0x20..=0x7e).contains(&b))
    {
        return Err("invalid catalog core identity".into());
    }
    let mut catalog = Catalog {
        generation,
        core: format!("pw-core:{cookie}:pid:{server}:{name}"),
        applications: BTreeMap::new(),
    };
    let mut streams = BTreeSet::new();
    for (index, line) in lines.enumerate() {
        let f: Vec<_> = line.split('\t').collect();
        if index >= planner::MAX_STREAMS || f.len() != 7 || f[0] != "app" {
            return Err("catalog row count/layout refused".into());
        }
        let client: u64 = number(f[1])?;
        let pid: u32 = number(f[2])?;
        let serial: u64 = number(f[3])?;
        if client == 0 || pid == 0 || serial == 0 || !streams.insert(serial) {
            return Err("zero or duplicate catalog identity".into());
        }
        let state = match f[4] {
            "live" => StreamState::Live,
            "stale" => StreamState::Stale,
            _ => return Err("unknown catalog state".into()),
        };
        let name = unhex(f[5])?;
        let binary = unhex(f[6])?;
        let id = ApplicationId(format!("pw-client:{client}:pid:{pid}"));
        if !catalog.applications.contains_key(&id)
            && catalog.applications.len() >= planner::MAX_APPLICATIONS
        {
            return Err("catalog application limit exceeded".into());
        }
        let app = catalog
            .applications
            .entry(id.clone())
            .or_insert_with(|| CatalogApplication {
                id,
                pid,
                names: BTreeSet::new(),
                binaries: BTreeSet::new(),
                streams: vec![],
            });
        if !name.is_empty() {
            app.names.insert(name);
        }
        if !binary.is_empty() {
            app.binaries.insert(binary);
        }
        app.streams.push(Stream {
            id: StreamId(format!("pw-node:{serial}")),
            state,
        });
    }
    for app in catalog.applications.values_mut() {
        app.streams.sort_by(|a, b| a.id.cmp(&b.id));
    }
    catalog.validate()?;
    Ok(catalog)
}

#[derive(Default)]
pub struct ApplicationService {
    current: Option<Catalog>,
    selected: BTreeMap<ApplicationId, String>,
    tracker: InventoryTracker,
    revision: u64,
    selection_revision: u64,
}
#[derive(Debug)]
pub struct Update {
    pub revoked: Vec<ApplicationId>,
    pub core_changed: bool,
    pub catalog_changed: bool,
    pub lifecycle: InventoryDelta,
}
impl ApplicationService {
    pub fn current(&self) -> Option<&Catalog> {
        self.current.as_ref()
    }
    pub fn selected(&self) -> &BTreeMap<ApplicationId, String> {
        &self.selected
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// Changes only when selected identity/streams/alias or core changes. This
    /// preserves evidence of an intervening edit even if it is later reversed.
    pub fn selection_revision(&self) -> u64 {
        self.selection_revision
    }
    fn projection(
        catalog: &Catalog,
        selected: &BTreeMap<ApplicationId, String>,
        version: u64,
    ) -> Result<Inventory, String> {
        let mut applications = Vec::new();
        let mut aliases = BTreeSet::new();
        for (id, name) in selected {
            if name.is_empty()
                || name.len() > 255
                || !name.bytes().all(|b| (0x20..=0x7e).contains(&b))
            {
                return Err("wire name requires 1..255 printable ASCII bytes".into());
            }
            if !aliases.insert(name) {
                return Err("selected applications require distinct wire names".into());
            }
            let app = catalog
                .applications
                .get(id)
                .ok_or("selected application absent")?;
            if app.streams.is_empty() || app.streams.iter().any(|s| s.state != StreamState::Live) {
                return Err("selected application is not live".into());
            }
            applications.push(Application {
                id: id.clone(),
                advertised_name: name.as_bytes().to_vec(),
                streams: app.streams.clone(),
            });
        }
        let inventory = Inventory {
            snapshot_id: version,
            applications,
        };
        planner::plan(&inventory, &[]).map_err(|e| e.to_string())?;
        let records: Vec<_> = inventory
            .applications
            .iter()
            .map(|a| Record {
                assignment: Assignment::from_byte(0),
                name: a.advertised_name.clone(),
            })
            .collect();
        crate::encode_records(&records).map_err(|e| e.to_string())?;
        Ok(inventory)
    }
    pub fn update(&mut self, catalog: Catalog) -> Result<Update, String> {
        catalog.validate()?;
        if self
            .current
            .as_ref()
            .is_some_and(|old| catalog.generation <= old.generation)
        {
            return Err("catalog generation must increase".into());
        }
        let changed = self
            .current
            .as_ref()
            .is_some_and(|old| old.core != catalog.core);
        let catalog_changed = self
            .current
            .as_ref()
            .is_none_or(|old| old.core != catalog.core || old.applications != catalog.applications);
        let mut selected = self.selected.clone();
        selected.retain(|id, _| {
            !changed
                && catalog.applications.get(id).is_some_and(|a| {
                    !a.streams.is_empty() && a.streams.iter().all(|s| s.state == StreamState::Live)
                })
        });
        let revoked = self
            .selected
            .keys()
            .filter(|id| !selected.contains_key(*id))
            .cloned()
            .collect();
        let revision = self
            .revision
            .checked_add(1)
            .ok_or("service revision exhausted")?;
        let inventory = Self::projection(&catalog, &selected, revision)?;
        let selection_changed = changed
            || self
                .tracker
                .current()
                .is_none_or(|old| old.applications != inventory.applications);
        let lifecycle = self
            .tracker
            .advance(&inventory)
            .map_err(|e| e.to_string())?;
        self.current = Some(catalog);
        self.selected = selected;
        self.revision = revision;
        if selection_changed {
            self.selection_revision = revision;
        }
        Ok(Update {
            revoked,
            core_changed: changed,
            catalog_changed,
            lifecycle,
        })
    }
    pub fn select(&mut self, id: &ApplicationId, name: &str) -> Result<(), String> {
        let catalog = self.current.as_ref().ok_or("no observed catalog")?;
        let mut selected = self.selected.clone();
        selected.insert(id.clone(), name.into());
        let revision = self
            .revision
            .checked_add(1)
            .ok_or("service revision exhausted")?;
        let inventory = Self::projection(catalog, &selected, revision)?;
        self.tracker
            .advance(&inventory)
            .map_err(|e| e.to_string())?;
        self.selected = selected;
        self.revision = revision;
        self.selection_revision = revision;
        Ok(())
    }
    pub fn unselect(&mut self, id: &ApplicationId) -> Result<(), String> {
        if !self.selected.contains_key(id) {
            return Err("application is not selected".into());
        }
        let catalog = self.current.as_ref().ok_or("no observed catalog")?;
        let mut selected = self.selected.clone();
        selected.remove(id);
        let revision = self
            .revision
            .checked_add(1)
            .ok_or("service revision exhausted")?;
        let inventory = Self::projection(catalog, &selected, revision)?;
        self.tracker
            .advance(&inventory)
            .map_err(|e| e.to_string())?;
        self.selected = selected;
        self.revision = revision;
        self.selection_revision = revision;
        Ok(())
    }
    pub fn inventory(&self) -> Option<&Inventory> {
        self.tracker.current()
    }
    pub fn require_current_plan(&self, plan: &planner::Plan) -> Result<(), String> {
        self.tracker
            .require_current_snapshot(plan)
            .map_err(|e| e.to_string())
    }
}
