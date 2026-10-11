//! Pure guarded link-change preparation. No OS access or application backend.
use crate::lifecycle::{InventoryTracker, LifecycleError};
use crate::planner::{self, ApplicationId, Outcome, Plan, StreamId, StreamState};
use crate::{Assignment, Record};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const MAX_NODES: usize = 8192;
pub const MAX_PORTS: usize = 16384;
pub const MAX_LINKS: usize = 16384;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectRef {
    pub id: u32,
    pub serial: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    Input,
    Output,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Channel {
    Left,
    Right,
    Other,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Port {
    pub object: ObjectRef,
    pub node: ObjectRef,
    pub direction: Direction,
    pub channel: Channel,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamBinding {
    pub application_id: ApplicationId,
    pub stream_id: StreamId,
    pub node: ObjectRef,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistingLink {
    pub object: ObjectRef,
    pub output: ObjectRef,
    pub input: ObjectRef,
    pub owner: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphSnapshot {
    pub generation: u64,
    /// Caller-verified server/session identity, never inferred from IDs alone.
    pub core_id: String,
    pub nodes: Vec<ObjectRef>,
    pub ports: Vec<Port>,
    pub streams: Vec<StreamBinding>,
    pub links: Vec<ExistingLink>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Destination {
    pub assignment: u8,
    pub node: ObjectRef,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ownership {
    pub token: String,
    /// Trusted caller ledger of receipts, separate from graph-reported tags.
    pub links: BTreeSet<ObjectRef>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    Disconnect {
        link: ExistingLink,
        output: Port,
        input: Port,
    },
    Connect {
        output: Port,
        input: Port,
        owner: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Undo {
    /// Reconnect endpoints; a recreated link receives a new identity.
    Restore {
        previous: ExistingLink,
        output: Port,
        input: Port,
    },
    /// Requires an actual successful-create receipt before any removal.
    RemoveCreated {
        forward_step: usize,
        output: Port,
        input: Port,
        owner: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardedStep {
    pub stream: StreamBinding,
    pub operation: Operation,
    pub undo: Undo,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedChange {
    pub generation: u64,
    pub core_id: String,
    pub steps: Vec<GuardedStep>,
}
impl PreparedChange {
    /// Undo only an acknowledged successful prefix, in reverse order.
    /// Data only: callers must freshly validate endpoints and created receipts.
    pub fn rollback_for_prefix(&self, applied: usize) -> Result<Vec<Undo>, PrepareError> {
        if applied > self.steps.len() {
            return Err(PrepareError::InvalidPrefix);
        }
        Ok(self.steps[..applied]
            .iter()
            .rev()
            .map(|s| s.undo.clone())
            .collect())
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrepareError {
    Lifecycle(LifecycleError),
    InvalidGraph(&'static str),
    InvalidMapping,
    UnknownAssignment(u8),
    MissingDestination(u8),
    BlockedResolution(Vec<u8>),
    PlanContents,
    GenerationMismatch,
    CoreChanged,
    MissingObject(ObjectRef),
    ReusedObject {
        expected: ObjectRef,
        observed: ObjectRef,
    },
    ChangedPort(ObjectRef),
    StreamChanged(StreamId),
    AmbiguousStereo(ObjectRef),
    AmbiguousRoute(ObjectRef),
    RouteChanged(StreamId),
    UnownedLink(ObjectRef),
    InvalidPrefix,
    InvalidStep,
}
impl fmt::Display for PrepareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "routing preparation: {self:?}")
    }
}
impl std::error::Error for PrepareError {}

fn label(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128 && s.bytes().all(|b| (0x20..=0x7e).contains(&b))
}
fn object<'a>(
    objects: impl IntoIterator<Item = &'a ObjectRef>,
    expected: ObjectRef,
) -> Result<(), PrepareError> {
    match objects.into_iter().find(|o| o.id == expected.id) {
        Some(o) if *o == expected => Ok(()),
        Some(o) => Err(PrepareError::ReusedObject {
            expected,
            observed: *o,
        }),
        None => Err(PrepareError::MissingObject(expected)),
    }
}
fn port(g: &GraphSnapshot, expected: ObjectRef) -> Result<&Port, PrepareError> {
    object(g.ports.iter().map(|p| &p.object), expected)?;
    Ok(g.ports
        .iter()
        .find(|p| p.object == expected)
        .expect("checked port"))
}
fn validate_graph(g: &GraphSnapshot) -> Result<(), PrepareError> {
    if !label(&g.core_id)
        || g.nodes.len() > MAX_NODES
        || g.ports.len() > MAX_PORTS
        || g.links.len() > MAX_LINKS
        || g.streams.len() > planner::MAX_STREAMS
    {
        return Err(PrepareError::InvalidGraph("labels or limits"));
    }
    let mut ids = BTreeSet::new();
    let mut serials = BTreeSet::new();
    for o in g
        .nodes
        .iter()
        .chain(g.ports.iter().map(|p| &p.object))
        .chain(g.links.iter().map(|l| &l.object))
    {
        if o.id == 0 || o.serial == 0 || !ids.insert(o.id) || !serials.insert(o.serial) {
            return Err(PrepareError::InvalidGraph(
                "duplicate or zero object identity",
            ));
        }
    }
    for p in &g.ports {
        object(&g.nodes, p.node)?;
    }
    let mut streams = BTreeSet::new();
    let mut nodes = BTreeSet::new();
    for s in &g.streams {
        if !label(&s.application_id.0)
            || !label(&s.stream_id.0)
            || !streams.insert(&s.stream_id)
            || !nodes.insert(s.node)
        {
            return Err(PrepareError::InvalidGraph("stream bindings"));
        }
        object(&g.nodes, s.node)?;
    }
    for l in &g.links {
        if l.owner.as_ref().is_some_and(|s| !label(s)) {
            return Err(PrepareError::InvalidGraph("owner label"));
        }
        if port(g, l.output)?.direction != Direction::Output
            || port(g, l.input)?.direction != Direction::Input
        {
            return Err(PrepareError::InvalidGraph("link direction"));
        }
    }
    Ok(())
}
fn stereo(
    g: &GraphSnapshot,
    node: ObjectRef,
    direction: Direction,
) -> Result<Vec<Port>, PrepareError> {
    object(&g.nodes, node)?;
    let mut ps: Vec<_> = g
        .ports
        .iter()
        .filter(|p| p.node == node && p.direction == direction)
        .cloned()
        .collect();
    ps.sort_by_key(|p| p.channel);
    if ps.len() != 2 || ps[0].channel != Channel::Left || ps[1].channel != Channel::Right {
        return Err(PrepareError::AmbiguousStereo(node));
    }
    Ok(ps)
}
fn same_ports(expected: &[Port], observed: &[Port]) -> Result<(), PrepareError> {
    for p in expected {
        object(observed.iter().map(|p| &p.object), p.object)?;
        if observed.iter().find(|q| q.object == p.object) != Some(p) {
            return Err(PrepareError::ChangedPort(p.object));
        }
    }
    Ok(())
}
fn binding<'a>(
    g: &'a GraphSnapshot,
    app: &ApplicationId,
    stream: &StreamId,
) -> Result<&'a StreamBinding, PrepareError> {
    g.streams
        .iter()
        .find(|s| &s.application_id == app && &s.stream_id == stream)
        .ok_or_else(|| PrepareError::StreamChanged(stream.clone()))
}
fn outgoing(g: &GraphSnapshot, node: ObjectRef) -> Vec<ExistingLink> {
    let ids: BTreeSet<_> = g
        .ports
        .iter()
        .filter(|p| p.node == node && p.direction == Direction::Output)
        .map(|p| p.object)
        .collect();
    let mut links: Vec<_> = g
        .links
        .iter()
        .filter(|l| ids.contains(&l.output))
        .cloned()
        .collect();
    links.sort_by_key(|l| l.object);
    links
}
fn live(tracker: &InventoryTracker, binding: &StreamBinding) -> Result<(), PrepareError> {
    if !tracker.current().is_some_and(|i| {
        i.applications.iter().any(|a| {
            a.id == binding.application_id
                && a.streams
                    .iter()
                    .any(|s| s.id == binding.stream_id && s.state == StreamState::Live)
        })
    }) {
        return Err(PrepareError::StreamChanged(binding.stream_id.clone()));
    }
    Ok(())
}
fn current(
    tracker: &InventoryTracker,
    generation: u64,
    core: &str,
    g: &GraphSnapshot,
) -> Result<(), PrepareError> {
    tracker
        .require_current_snapshot(&Plan {
            snapshot_id: generation,
            resolutions: vec![],
        })
        .map_err(PrepareError::Lifecycle)?;
    if g.generation != generation {
        return Err(PrepareError::GenerationMismatch);
    }
    if g.core_id != core {
        return Err(PrepareError::CoreChanged);
    }
    Ok(())
}

/// All-or-nothing dry-run preparation. No OS reads, routing or authorization.
/// Expected graph anchors identities; observed graph is a fresh caller observation.
pub fn prepare(
    tracker: &InventoryTracker,
    plan: &Plan,
    expected: &GraphSnapshot,
    observed: &GraphSnapshot,
    destinations: &[Destination],
    ownership: &Ownership,
) -> Result<PreparedChange, PrepareError> {
    tracker
        .require_current_snapshot(plan)
        .map_err(PrepareError::Lifecycle)?;
    current(tracker, plan.snapshot_id, &expected.core_id, expected)?;
    current(tracker, plan.snapshot_id, &expected.core_id, observed)?;
    validate_graph(expected)?;
    validate_graph(observed)?;
    if !label(&ownership.token) || ownership.links.len() > MAX_LINKS || destinations.len() > 4 {
        return Err(PrepareError::InvalidMapping);
    }
    let mut mapped = BTreeMap::new();
    let mut dest_nodes = BTreeSet::new();
    for d in destinations {
        if !(1..=4).contains(&d.assignment) {
            return Err(PrepareError::UnknownAssignment(d.assignment));
        }
        if mapped.insert(d.assignment, d.node).is_some() || !dest_nodes.insert(d.node) {
            return Err(PrepareError::InvalidMapping);
        }
        same_ports(
            &stereo(expected, d.node, Direction::Input)?,
            &stereo(observed, d.node, Direction::Input)?,
        )?;
    }
    let mut records = Vec::new();
    for r in &plan.resolutions {
        let Outcome::Proposed(route) = &r.outcome else {
            return Err(PrepareError::BlockedResolution(r.name.clone()));
        };
        if r.occurrences == 0 || r.occurrences > planner::MAX_RECORDS.saturating_sub(records.len())
        {
            return Err(PrepareError::PlanContents);
        }
        records.extend((0..r.occurrences).map(|_| Record {
            assignment: Assignment::from_byte(route.link.assignment()),
            name: r.name.clone(),
        }));
    }
    let inv = tracker
        .current()
        .ok_or_else(|| PrepareError::Lifecycle(LifecycleError::NoSnapshot))?;
    if planner::plan(inv, &records).map_err(|_| PrepareError::PlanContents)? != *plan {
        return Err(PrepareError::PlanContents);
    }
    let mut selected = BTreeMap::new();
    for route in plan.proposed_routes() {
        let target = *mapped
            .get(&route.link.assignment())
            .ok_or(PrepareError::MissingDestination(route.link.assignment()))?;
        for sid in &route.stream_ids {
            if selected
                .insert(sid.clone(), (route.application_id.clone(), target))
                .is_some()
            {
                return Err(PrepareError::PlanContents);
            }
        }
    }
    let mut disconnects = Vec::new();
    let mut connects = Vec::new();
    for (sid, (app, target)) in selected {
        let source = binding(expected, &app, &sid)?.clone();
        live(tracker, &source)?;
        object(&observed.nodes, source.node)?;
        if binding(observed, &app, &sid)? != &source {
            return Err(PrepareError::StreamChanged(sid));
        }
        let outputs = stereo(expected, source.node, Direction::Output)?;
        same_ports(&outputs, &stereo(observed, source.node, Direction::Output)?)?;
        let inputs = stereo(observed, target, Direction::Input)?;
        let before = outgoing(expected, source.node);
        let after = outgoing(observed, source.node);
        for old in &before {
            object(observed.links.iter().map(|l| &l.object), old.object)?;
        }
        if before != after {
            return Err(PrepareError::RouteChanged(sid));
        }
        for (out, input) in outputs.into_iter().zip(inputs) {
            let old: Vec<_> = after.iter().filter(|l| l.output == out.object).collect();
            if old.len() > 1 {
                return Err(PrepareError::AmbiguousRoute(out.object));
            }
            if let Some(link) = old.first() {
                if link.input == input.object {
                    continue;
                } // Preserve an already-correct foreign link.
                if link.owner.as_deref() != Some(&ownership.token)
                    || !ownership.links.contains(&link.object)
                {
                    return Err(PrepareError::UnownedLink(link.object));
                }
                let prior = port(observed, link.input)?.clone();
                same_ports(&[port(expected, link.input)?.clone()], &[prior.clone()])?;
                object(&expected.nodes, prior.node)?;
                object(&observed.nodes, prior.node)?;
                disconnects.push(GuardedStep {
                    stream: source.clone(),
                    operation: Operation::Disconnect {
                        link: (*link).clone(),
                        output: out.clone(),
                        input: prior.clone(),
                    },
                    undo: Undo::Restore {
                        previous: (*link).clone(),
                        output: out.clone(),
                        input: prior,
                    },
                });
            }
            connects.push((source.clone(), out, input));
        }
    }
    let mut steps = disconnects;
    for (stream, output, input) in connects {
        let forward_step = steps.len();
        steps.push(GuardedStep {
            stream,
            operation: Operation::Connect {
                output: output.clone(),
                input: input.clone(),
                owner: ownership.token.clone(),
            },
            undo: Undo::RemoveCreated {
                forward_step,
                output,
                input,
                owner: ownership.token.clone(),
            },
        });
    }
    Ok(PreparedChange {
        generation: plan.snapshot_id,
        core_id: expected.core_id.clone(),
        steps,
    })
}

/// Revalidate one forward step against supplied current data immediately before
/// a future backend acts. Earlier acknowledged disconnects must already be
/// reflected in this fresh graph before a Connect step can validate.
pub fn validate_step(
    prepared: &PreparedChange,
    index: usize,
    tracker: &InventoryTracker,
    g: &GraphSnapshot,
    ownership: &Ownership,
) -> Result<(), PrepareError> {
    current(tracker, prepared.generation, &prepared.core_id, g)?;
    validate_graph(g)?;
    let step = prepared.steps.get(index).ok_or(PrepareError::InvalidStep)?;
    live(tracker, &step.stream)?;
    object(&g.nodes, step.stream.node)?;
    if binding(g, &step.stream.application_id, &step.stream.stream_id)? != &step.stream {
        return Err(PrepareError::StreamChanged(step.stream.stream_id.clone()));
    }
    let (output, input) = match &step.operation {
        Operation::Disconnect { output, input, .. } | Operation::Connect { output, input, .. } => {
            (output, input)
        }
    };
    for p in [output, input] {
        object(&g.nodes, p.node)?;
        if port(g, p.object)? != p {
            return Err(PrepareError::ChangedPort(p.object));
        }
    }
    let existing: Vec<_> = g
        .links
        .iter()
        .filter(|l| l.output == output.object)
        .collect();
    match &step.operation {
        Operation::Disconnect { link, .. } => {
            object(g.links.iter().map(|l| &l.object), link.object)?;
            if existing.as_slice() != [link] {
                return Err(PrepareError::RouteChanged(step.stream.stream_id.clone()));
            }
            if link.owner.as_deref() != Some(&ownership.token)
                || !ownership.links.contains(&link.object)
            {
                return Err(PrepareError::UnownedLink(link.object));
            }
        }
        Operation::Connect { owner, .. } => {
            if owner != &ownership.token || !label(owner) {
                return Err(PrepareError::InvalidMapping);
            }
            if !existing.is_empty() {
                return Err(PrepareError::RouteChanged(step.stream.stream_id.clone()));
            }
        }
    }
    Ok(())
}
