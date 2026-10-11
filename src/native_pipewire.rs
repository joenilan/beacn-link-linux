//! Opt-in Linux backend for an explicitly owned disposable private core.
//! No desktop apply entry point. The session ledger contains only native creates.
use crate::{
    execution::{Backend, Effect, Observation},
    executor::*,
    graph_snapshot,
    receipts::Acknowledgment,
};
use std::{
    collections::BTreeMap,
    ffi::{c_char, c_int, c_void, CString},
    fs,
    marker::PhantomData,
    os::{
        fd::IntoRawFd,
        unix::{
            fs::{FileTypeExt, MetadataExt},
            net::UnixStream,
        },
    },
    path::Path,
    rc::Rc,
};

/// Exact client configuration accepted by this experimental backend. An absolute
/// path inside the owned runtime avoids loading desktop config and drop-ins.
pub const CLIENT_CONFIG: &str = "context.properties = { support.dbus = false context.num-data-loops = 0 }\ncontext.modules = [ { name = libpipewire-module-protocol-native } ]\ncontext.objects = []\ncontext.exec = []\n";
#[repr(C)]
struct Endpoint {
    id: u32,
    serial: u64,
    node: u32,
    node_serial: u64,
    direction: c_int,
    channel: c_int,
}
impl From<&Port> for Endpoint {
    fn from(p: &Port) -> Self {
        Self {
            id: p.object.id,
            serial: p.object.serial,
            node: p.node.id,
            node_serial: p.node.serial,
            direction: if p.direction == Direction::Output {
                1
            } else {
                0
            },
            channel: if p.channel == Channel::Left { 0 } else { 1 },
        }
    }
}
#[derive(Clone, Copy, Default)]
#[repr(C)]
struct Receipt {
    id: u32,
    serial: u64,
}
unsafe extern "C" {
    fn bcl_open(
        fd: c_int,
        config: *const c_char,
        cookie: u32,
        name: *const c_char,
        pid: u32,
    ) -> *mut c_void;
    fn bcl_close(session: *mut c_void);
    fn bcl_snapshot(session: *mut c_void, buffer: *mut c_char, size: usize) -> c_int;
    fn bcl_perform(
        session: *mut c_void,
        deleting: c_int,
        output: *const Endpoint,
        input: *const Endpoint,
        owner: *const c_char,
        old: *const Receipt,
        result: *mut Receipt,
    ) -> c_int;
    fn bcl_reconcile(session: *mut c_void, result: *mut Receipt) -> c_int;
    fn getuid() -> u32;
}
struct Pending {
    operation: Operation,
    receipt: Option<Receipt>,
}
pub struct NativePipeWire {
    session: *mut c_void,
    bindings: BTreeMap<String, String>,
    ownership: Ownership,
    pending: Option<Pending>,
    last_completed: Option<(Operation, Receipt)>,
    generation: u64,
    // PipeWire loop, callbacks and session ledger must stay on their owner thread.
    _single_thread: PhantomData<Rc<()>>,
}
impl Drop for NativePipeWire {
    fn drop(&mut self) {
        unsafe {
            bcl_close(self.session);
        }
    }
}
impl NativePipeWire {
    /// Requires a direct /tmp/beacn-pw-* mode-0700 runtime, beacn-test-* socket,
    /// exact minimal client.conf without drop-ins, expected cookie and server PID.
    /// The native socket's SO_PEERCRED is checked before handing the FD to PipeWire.
    pub fn connect(
        runtime: &Path,
        name: &str,
        cookie: u32,
        server_pid: u32,
        bindings: BTreeMap<String, String>,
        owner: String,
    ) -> Result<Self, String> {
        let uid = unsafe { getuid() };
        let meta = fs::symlink_metadata(runtime).map_err(|e| e.to_string())?;
        let canonical = fs::canonicalize(runtime).map_err(|e| e.to_string())?;
        if !meta.is_dir()
            || meta.file_type().is_symlink()
            || canonical != runtime
            || runtime.parent() != Some(Path::new("/tmp"))
            || !runtime
                .file_name()
                .and_then(|v| v.to_str())
                .is_some_and(|v| v.starts_with("beacn-pw-"))
            || meta.uid() != uid
            || meta.mode() & 0o777 != 0o700
        {
            return Err("requires an owned mode-0700 /tmp/beacn-pw-* runtime".into());
        }
        if !name.starts_with("beacn-test-")
            || name.len() > 64
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || server_pid == 0
        {
            return Err("requires explicit private core name and server PID".into());
        }
        let config = runtime.join("client.conf");
        let cm = fs::symlink_metadata(&config).map_err(|e| e.to_string())?;
        if !cm.is_file()
            || cm.file_type().is_symlink()
            || cm.uid() != uid
            || cm.mode() & 0o077 != 0
            || cm.len() != CLIENT_CONFIG.len() as u64
            || fs::read(&config).map_err(|e| e.to_string())? != CLIENT_CONFIG.as_bytes()
            || runtime.join("client.conf.d").exists()
        {
            return Err("requires exact private client.conf with no drop-ins".into());
        }
        let socket = runtime.join(name);
        let sm = fs::symlink_metadata(&socket).map_err(|e| e.to_string())?;
        if !sm.file_type().is_socket() || sm.uid() != uid {
            return Err("private socket ownership mismatch".into());
        }
        if bindings.len() > crate::planner::MAX_APPLICATIONS
            || bindings.iter().any(|(id, name)| {
                id.is_empty()
                    || id.len() > 128
                    || name.is_empty()
                    || name.len() > 255
                    || !id
                        .bytes()
                        .chain(name.bytes())
                        .all(|b| (0x20..=0x7e).contains(&b))
            })
            || owner.is_empty()
            || owner.len() > 128
            || !owner.bytes().all(|b| (0x20..=0x7e).contains(&b))
        {
            return Err("invalid explicit bindings or owner".into());
        }
        let stream = UnixStream::connect(socket).map_err(|e| e.to_string())?;
        let config = CString::new(config.to_str().ok_or("invalid config path")?)
            .map_err(|e| e.to_string())?;
        let name = CString::new(name).map_err(|e| e.to_string())?;
        // bcl_open consumes FD on success and failure. All struct layouts are
        // defined by repr(C); callbacks and public ABI definitions live in C.
        let session = unsafe {
            bcl_open(
                stream.into_raw_fd(),
                config.as_ptr(),
                cookie,
                name.as_ptr(),
                server_pid,
            )
        };
        if session.is_null() {
            return Err("private core connection/identity validation failed".into());
        }
        Ok(Self {
            session,
            bindings,
            ownership: Ownership {
                token: owner,
                links: Default::default(),
            },
            pending: None,
            last_completed: None,
            generation: 0,
            _single_thread: PhantomData,
        })
    }
    pub fn ownership(&self) -> &Ownership {
        &self.ownership
    }
    fn snapshot(&mut self, generation: u64) -> Result<Observation, String> {
        let mut buffer = vec![0u8; graph_snapshot::MAX_GRAPH_TSV_BYTES + 1];
        if unsafe { bcl_snapshot(self.session, buffer.as_mut_ptr().cast(), buffer.len()) } != 0 {
            return Err("native observation unavailable or incomplete".into());
        }
        let length = buffer
            .iter()
            .position(|b| *b == 0)
            .ok_or("unterminated native observation")?;
        let raw = std::str::from_utf8(&buffer[..length]).map_err(|e| e.to_string())?;
        let mut graph = String::new();
        let mut inventory = String::new();
        let mut seen = std::collections::BTreeSet::new();
        for row in raw.lines() {
            let f: Vec<_> = row.split('\t').collect();
            if f.first() == Some(&"playback") {
                if f.len() != 6 {
                    return Err("malformed native playback row".into());
                }
                graph.push_str(&format!("stream\t{}\t{}\t{}\t{}\n", f[1], f[2], f[4], f[5]));
                if let Some(name) = self.bindings.get(f[1]) {
                    inventory.push_str(&format!(
                        "inventory\t{}\t{}\t{}\t{}\n",
                        f[1], name, f[2], f[3]
                    ));
                    seen.insert(f[1].to_owned());
                }
            } else {
                graph.push_str(row);
                graph.push('\n');
            }
        }
        if self.bindings.keys().any(|id| !seen.contains(id)) {
            return Err("explicit binding disappeared".into());
        }
        let mut tsv = inventory;
        for side in ["expected", "observed"] {
            for row in graph.lines() {
                tsv.push_str(side);
                tsv.push('\t');
                tsv.push_str(row);
                tsv.push('\n');
            }
        }
        tsv.push_str(&format!("owner\t{}\n", self.ownership.token));
        let parsed = graph_snapshot::preparation_from_tsv(generation, &tsv)?;
        self.generation = generation;
        Ok(Observation {
            graph: parsed.observed,
            inventory: parsed.inventory,
        })
    }
    fn verify(
        &mut self,
        operation: &Operation,
        receipt: Receipt,
    ) -> Result<Acknowledgment, String> {
        let observed = self.snapshot(self.generation)?;
        let object = ObjectRef {
            id: receipt.id,
            serial: receipt.serial,
        };
        match operation {
            Operation::Disconnect { link, .. } => {
                if object != link.object || observed.graph.links.iter().any(|l| l.object == object)
                {
                    return Err("disconnect not verified".into());
                }
                self.ownership.links.remove(&object);
                Ok(Acknowledgment::Disconnected(object))
            }
            Operation::Connect {
                output,
                input,
                owner,
            } => {
                let expected = ExistingLink {
                    object,
                    output: output.object,
                    input: input.object,
                    owner: Some(owner.clone()),
                };
                if !observed.graph.links.contains(&expected)
                    || [output, input]
                        .iter()
                        .any(|p| !observed.graph.ports.contains(p))
                    || observed
                        .graph
                        .links
                        .iter()
                        .filter(|l| l.output == output.object)
                        .count()
                        != 1
                {
                    return Err("created link endpoints/identity/ownership not verified".into());
                }
                self.ownership.links.insert(object);
                Ok(Acknowledgment::Created(expected))
            }
        }
    }
}
impl Backend for NativePipeWire {
    fn observe(&mut self, generation: u64) -> Result<Observation, String> {
        self.snapshot(generation)
    }
    fn perform(&mut self, operation: &Operation) -> Effect {
        if self.pending.is_some() {
            return Effect::Unknown("pending effect needs reconciliation".into());
        }
        let (output, input, owner, old, deleting) = match operation {
            Operation::Connect {
                output,
                input,
                owner,
            } => (output, input, owner, Receipt::default(), 0),
            Operation::Disconnect {
                link,
                output,
                input,
            } => {
                if !self.ownership.links.contains(&link.object)
                    || link.owner.as_deref() != Some(&self.ownership.token)
                {
                    return Effect::Failed("missing native receipt".into());
                }
                (
                    output,
                    input,
                    &self.ownership.token,
                    Receipt {
                        id: link.object.id,
                        serial: link.object.serial,
                    },
                    1,
                )
            }
        };
        if owner != &self.ownership.token
            || output.channel == Channel::Other
            || input.channel != output.channel
        {
            return Effect::Failed("invalid native operation owner/channel".into());
        }
        let owner = match CString::new(owner.as_str()) {
            Ok(v) => v,
            Err(e) => return Effect::Failed(e.to_string()),
        };
        let mut receipt = Receipt::default();
        let status = unsafe {
            bcl_perform(
                self.session,
                deleting,
                &Endpoint::from(output),
                &Endpoint::from(input),
                owner.as_ptr(),
                &old,
                &mut receipt,
            )
        };
        self.last_completed = None;
        match status {
            0 => match self.verify(operation, receipt) {
                Ok(ack) => {
                    self.last_completed = Some((operation.clone(), receipt));
                    Effect::Acknowledged(ack)
                }
                Err(reason) => {
                    self.pending = Some(Pending {
                        operation: operation.clone(),
                        receipt: Some(receipt),
                    });
                    Effect::Unknown(reason)
                }
            },
            1 => Effect::Failed("native operation refused without effect".into()),
            _ => {
                self.pending = Some(Pending {
                    operation: operation.clone(),
                    receipt: None,
                });
                Effect::Unknown("native acknowledgment unresolved".into())
            }
        }
    }
    fn reconcile(&mut self) -> Result<Option<Acknowledgment>, String> {
        let Some(pending) = self.pending.take() else {
            let (operation, receipt) = self
                .last_completed
                .clone()
                .ok_or("no pending native effect")?;
            return self.verify(&operation, receipt).map(Some);
        };
        let mut receipt = pending.receipt.unwrap_or_default();
        let status = if pending.receipt.is_some() {
            0
        } else {
            unsafe { bcl_reconcile(self.session, &mut receipt) }
        };
        let result = match status {
            0 => self.verify(&pending.operation, receipt).map(Some),
            1 => Ok(None),
            _ => Err("native effect still unresolved".into()),
        };
        if result.is_err() {
            self.pending = Some(pending);
        }
        result
    }
}
