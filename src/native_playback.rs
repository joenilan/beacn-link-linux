//! Opt-in eight-channel playback on the authenticated disposable core only.
//! Owns native non-lingering proxies; no pw-cli, subprocess, capture or USB claim.
use crate::{
    execution::{Backend, Observation},
    executor::{ObjectRef, Ownership},
    lifecycle::InventoryTracker,
    native_audio::PinnedPlayback,
    native_pipewire::NativePipeWire,
    receipts::{verify_sink_release, ReleaseReport, SinkReceipt},
    route_publication::VerifiedRoutePublication,
};
use std::{
    collections::BTreeMap,
    ffi::{c_char, c_int, c_void, CString},
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct Configuration {
    pub runtime: PathBuf,
    pub core: String,
    pub cookie: u32,
    pub server: u32,
    pub owner: String,
    pub pair: u8,
}
/// Sealed target: callers cannot supply arbitrary ALSA paths/card numbers.
pub struct Output(Target);
enum Target {
    Synthetic,
    Studio {
        sysfs: PathBuf,
        descriptors: Vec<u8>,
        proof: PinnedPlayback,
    },
}
impl Output {
    pub fn synthetic() -> Self {
        Self(Target::Synthetic)
    }
    /// Metadata-only preflight. No PCM open, USB claim, resets or driver detach.
    pub fn pin_studio(
        sysfs: &Path,
        expected: &[u8],
        server: u32,
        authorized: bool,
    ) -> Result<Self, String> {
        if !authorized {
            return Err("Studio playback requires explicit authority".into());
        }
        crate::usb_handshake::check_descriptors(expected).map_err(|e| e.to_string())?;
        let selected = sysfs.canonicalize().map_err(|e| e.to_string())?;
        if !selected.starts_with("/sys/devices")
            || fs::read(selected.join("descriptors")).map_err(|e| e.to_string())? != expected
            || fs::read_to_string(selected.join("idVendor"))
                .map_err(|e| e.to_string())?
                .trim()
                != "33ae"
            || fs::read_to_string(selected.join("idProduct"))
                .map_err(|e| e.to_string())?
                .trim()
                != "4003"
            || fs::read_to_string("/sys/class/dmi/id/product_name")
                .unwrap_or_default()
                .to_lowercase()
                .contains("vmware")
        {
            return Err("selected native Studio identity refused".into());
        }
        let proof = PinnedPlayback::pin(&selected, server)
            .map_err(|_| "selected Studio PCM preflight refused")?;
        if proof.card() > 255 {
            return Err("selected Studio card outside native adapter limit".into());
        }
        Ok(Self(Target::Studio {
            sysfs: selected,
            descriptors: expected.to_vec(),
            proof,
        }))
    }
    fn card(&self) -> c_int {
        match &self.0 {
            Target::Synthetic => -1,
            Target::Studio { proof, .. } => proof.card() as c_int,
        }
    }
    fn binding(&self) -> Result<(), String> {
        if let Target::Studio {
            sysfs, descriptors, ..
        } = &self.0
        {
            if fs::read(sysfs.join("descriptors")).map_err(|e| e.to_string())? != *descriptors {
                return Err("Studio descriptor binding changed".into());
            }
        }
        Ok(())
    }
    fn closed(&self) -> Result<(), String> {
        self.binding()?;
        if let Target::Studio { proof, .. } = &self.0 {
            proof
                .closed()
                .map_err(|_| "Studio PCM closure unverified")?;
        }
        Ok(())
    }
    fn running(&mut self) -> Result<(), String> {
        self.binding()?;
        if let Target::Studio { proof, .. } = &mut self.0 {
            proof
                .validate()
                .map_err(|_| "Studio PCM owner/format/clock proof failed")?;
        }
        Ok(())
    }
    fn wait_closed(&self) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            self.binding()?;
            match self.closed() {
                Ok(()) => return Ok(()),
                Err(error) if Instant::now() >= deadline => return Err(error),
                Err(_) => std::thread::sleep(Duration::from_millis(20)),
            }
        }
    }
}
#[derive(Debug, Clone)]
pub struct StartFailure {
    pub message: String,
    pub cleanup: Result<(), String>,
}
#[derive(Clone, Copy, Default)]
#[repr(C)]
struct Receipt {
    id: u32,
    serial: u64,
}
impl From<Receipt> for ObjectRef {
    fn from(r: Receipt) -> Self {
        Self {
            id: r.id,
            serial: r.serial,
        }
    }
}
impl From<ObjectRef> for Receipt {
    fn from(r: ObjectRef) -> Self {
        Self {
            id: r.id,
            serial: r.serial,
        }
    }
}
#[derive(Clone, Copy, Default)]
#[repr(C)]
struct References {
    node: Receipt,
    links: [Receipt; 2],
    ports: [Receipt; 8],
    monitors: [Receipt; 2],
}
unsafe extern "C" {
    fn bcl_playback_create(
        session: *mut c_void,
        source: *const Receipt,
        owner: *const c_char,
        pair: c_int,
        card: c_int,
        result: *mut References,
    ) -> c_int;
    fn bcl_playback_check(
        session: *mut c_void,
        source: *const Receipt,
        owner: *const c_char,
        pair: c_int,
        result: *const References,
    ) -> c_int;
    #[cfg(feature = "native-test-faults")]
    fn bcl_playback_test_fault(session: *mut c_void, stage: c_int) -> c_int;
}
pub struct NativePlayback {
    backend: Option<NativePipeWire>,
    inspector: NativePipeWire,
    config: Configuration,
    source: Receipt,
    refs: References,
    receipt: Option<SinkReceipt>,
    output: Output,
    tracker: InventoryTracker,
    generation: u64,
    cleanup: Option<Result<ReleaseReport, String>>,
    halted: Option<String>,
}
impl Drop for NativePlayback {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
impl NativePlayback {
    pub fn receipt(&self) -> Option<&SinkReceipt> {
        self.receipt.as_ref()
    }
    pub fn start(
        config: Configuration,
        output: Output,
        proof: &VerifiedRoutePublication,
    ) -> Result<Self, StartFailure> {
        Self::start_with_fault(config, output, proof, 0)
    }
    /// Real native partial/uncertain-create boundaries, never a physical target.
    #[cfg(feature = "native-test-faults")]
    pub fn test_start(
        config: Configuration,
        output: Output,
        proof: &VerifiedRoutePublication,
        stage: u8,
    ) -> Result<Self, StartFailure> {
        if !matches!(output.0, Target::Synthetic) || !(1..=4).contains(&stage) {
            return Err(StartFailure {
                message: "playback faults require a synthetic output and stage 1..4".into(),
                cleanup: Ok(()),
            });
        }
        Self::start_with_fault(config, output, proof, stage)
    }
    fn start_with_fault(
        config: Configuration,
        output: Output,
        proof: &VerifiedRoutePublication,
        fault: u8,
    ) -> Result<Self, StartFailure> {
        let setup = (|| {
            if !(1..=4).contains(&config.pair)
                || proof.records().len() != 1
                || proof.records()[0].assignment.byte() != config.pair
            {
                return Err("playback requires one freshly verified selected pair".to_string());
            }
            let source = proof
                .destination(config.pair)
                .ok_or("verified destination absent")?
                .node;
            output.closed()?;
            let inspector = NativePipeWire::connect(
                &config.runtime,
                &config.core,
                config.cookie,
                config.server,
                BTreeMap::new(),
                config.owner.clone(),
            )?;
            let bindings = proof
                .inventory()
                .applications
                .iter()
                .map(|a| {
                    (
                        a.id.0.clone(),
                        String::from_utf8(a.advertised_name.clone()).unwrap(),
                    )
                })
                .collect();
            let mut backend = NativePipeWire::connect(
                &config.runtime,
                &config.core,
                config.cookie,
                config.server,
                bindings,
                config.owner.clone(),
            )?;
            let generation = proof.inventory().snapshot_id;
            let baseline = backend.observe(generation)?;
            proof.verify_observation(&baseline)?;
            let mut tracker = InventoryTracker::default();
            tracker
                .advance(proof.inventory())
                .map_err(|e| e.to_string())?;
            Ok::<_, String>((
                source,
                inspector,
                backend,
                generation,
                baseline.graph,
                tracker,
            ))
        })();
        let (source, inspector, backend, generation, baseline, tracker) =
            setup.map_err(|message| StartFailure {
                message,
                cleanup: Ok(()),
            })?;
        let mut owner = Self {
            backend: Some(backend),
            inspector,
            source: source.into(),
            config,
            refs: References::default(),
            receipt: None,
            output,
            tracker,
            generation,
            cleanup: None,
            halted: None,
        };
        let created = (|| {
            #[cfg(feature = "native-test-faults")]
            if fault != 0
                && unsafe {
                    bcl_playback_test_fault(
                        owner.backend.as_mut().unwrap().playback_session(),
                        fault.into(),
                    )
                } != 0
            {
                return Err("playback fault setup refused".into());
            }
            #[cfg(not(feature = "native-test-faults"))]
            if fault != 0 {
                return Err("playback faults are not enabled".into());
            }
            let token = CString::new(owner.config.owner.as_str()).map_err(|e| e.to_string())?;
            let result = unsafe {
                bcl_playback_create(
                    owner.backend.as_mut().unwrap().playback_session(),
                    &owner.source,
                    token.as_ptr(),
                    owner.config.pair.into(),
                    owner.output.card(),
                    &mut owner.refs,
                )
            };
            if result != 0 {
                return Err(format!(
                    "native playback creation unverified at stage {result}"
                ));
            }
            let observed = owner.backend.as_mut().unwrap().observe(generation)?;
            proof.verify_observation(&observed)?;
            let node = owner.refs.node.into();
            let ports = observed
                .graph
                .ports
                .iter()
                .filter(|p| p.node == node)
                .map(|p| p.object)
                .collect();
            let receipt = SinkReceipt {
                node,
                ports,
                ownership: Ownership {
                    token: owner.config.owner.clone(),
                    links: owner.refs.links.iter().map(|r| (*r).into()).collect(),
                },
            };
            // Removing only acknowledged creations must reproduce the initial
            // graph. Nothing is adopted by an owner tag or discovered name.
            verify_sink_release(&observed.graph, &baseline, &receipt)?;
            owner.receipt = Some(receipt);
            owner.check()?;
            Ok::<_, String>(())
        })();
        if let Err(message) = created {
            // Unknown/partial effects are still owned native proxy resources.
            drop(owner.backend.take());
            let cleanup = (|| {
                let after = owner.inspector.observe(generation)?;
                if after.graph != baseline {
                    return Err("playback startup graph restoration unverified".into());
                }
                owner.output.wait_closed()?;
                Ok(())
            })();
            owner.cleanup = Some(cleanup.clone().map(|_| ReleaseReport::default()));
            return Err(StartFailure { message, cleanup });
        }
        Ok(owner)
    }
    /// Metadata, retained proxy and clock checks only; no payload read.
    pub fn check(&mut self) -> Result<(), String> {
        if let Some(error) = &self.halted {
            return Err(error.clone());
        }
        let result = self.check_current();
        if let Err(error) = &result {
            self.halted = Some(error.clone());
        }
        result
    }
    fn check_current(&mut self) -> Result<(), String> {
        let token = CString::new(self.config.owner.as_str()).map_err(|e| e.to_string())?;
        let backend = self.backend.as_mut().ok_or("playback is stopped")?;
        if unsafe {
            bcl_playback_check(
                backend.playback_session(),
                &self.source,
                token.as_ptr(),
                self.config.pair.into(),
                &self.refs,
            )
        } != 0
        {
            return Err("native playback endpoints/receipts changed".into());
        }
        self.output.running()
    }
    pub fn refresh(&mut self, proof: &VerifiedRoutePublication) -> Result<(), String> {
        if let Some(error) = &self.halted {
            return Err(error.clone());
        }
        let result = self.refresh_current(proof);
        if let Err(error) = &result {
            self.halted = Some(error.clone());
        }
        result
    }
    fn refresh_current(&mut self, proof: &VerifiedRoutePublication) -> Result<(), String> {
        if proof.records().len() != 1
            || proof.records()[0].assignment.byte() != self.config.pair
            || proof.destination(self.config.pair).map(|d| d.node) != Some(self.source.into())
        {
            return Err("playback verified assignment changed".into());
        }
        self.tracker
            .advance(proof.inventory())
            .map_err(|e| e.to_string())?;
        self.generation = proof.inventory().snapshot_id;
        let observed = self
            .backend
            .as_mut()
            .ok_or("playback is stopped")?
            .observe(self.generation)?;
        proof.verify_observation(&observed)?;
        self.check()
    }
    pub fn stop(&mut self) -> Result<ReleaseReport, String> {
        if let Some(result) = &self.cleanup {
            return result.clone();
        }
        let before = self.inspector.observe(self.generation);
        drop(self.backend.take());
        let after = self.inspector.observe(self.generation);
        let result = (|| {
            let before = before?;
            let after = after?;
            if before.inventory != after.inventory {
                return Err("playback inspector inventory changed".into());
            }
            let report = verify_sink_release(
                &before.graph,
                &after.graph,
                self.receipt.as_ref().ok_or("playback receipt absent")?,
            )?;
            self.output.wait_closed()?;
            Ok(report)
        })();
        self.cleanup = Some(result.clone());
        result
    }
    pub fn graph(&mut self) -> Result<Observation, String> {
        self.inspector.observe(self.generation)
    }
}
