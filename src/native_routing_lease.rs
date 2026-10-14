//! Verified release of one retained private-core executor, independent of source inventory.
use crate::{
    execution::{Backend, Effect, Observation},
    executor::{Operation, Ownership},
    native_pipewire::NativePipeWire,
    receipts::{verify_release, Acknowledgment},
    routing_control::{CleanupReport, OwnedRoutingBackend},
};
use std::{collections::BTreeMap, path::Path};

pub struct NativeRoutingLease {
    backend: Option<NativePipeWire>,
    inspector: NativePipeWire,
    generation: u64,
    cleanup: Option<Result<CleanupReport, String>>,
}
impl NativeRoutingLease {
    /// Both connections use the original explicit private runtime/core/peer gates.
    /// The inspector has no source bindings and is only used for observations.
    pub fn connect(
        runtime: &Path,
        name: &str,
        cookie: u32,
        server: u32,
        bindings: BTreeMap<String, String>,
        owner: String,
    ) -> Result<Self, String> {
        let inspector = NativePipeWire::connect(
            runtime,
            name,
            cookie,
            server,
            BTreeMap::new(),
            owner.clone(),
        )?;
        let backend = NativePipeWire::connect(runtime, name, cookie, server, bindings, owner)?;
        Ok(Self {
            backend: Some(backend),
            inspector,
            generation: 1,
            cleanup: None,
        })
    }
}
impl Backend for NativeRoutingLease {
    fn observe(&mut self, generation: u64) -> Result<Observation, String> {
        self.generation = generation;
        self.backend
            .as_mut()
            .ok_or("owned session is stopped")?
            .observe(generation)
    }
    fn perform(&mut self, operation: &Operation) -> Effect {
        match self.backend.as_mut() {
            Some(backend) => backend.perform(operation),
            None => Effect::Failed("owned session is stopped".into()),
        }
    }
    fn reconcile(&mut self) -> Result<Option<Acknowledgment>, String> {
        self.backend
            .as_mut()
            .ok_or("owned session is stopped")?
            .reconcile()
    }
}
impl OwnedRoutingBackend for NativeRoutingLease {
    fn ownership(&self) -> &Ownership {
        // Controller never requests ownership after it has stopped its backend.
        self.backend
            .as_ref()
            .expect("running owned session")
            .ownership()
    }
    fn stop_owned(&mut self) -> Result<CleanupReport, String> {
        if let Some(result) = &self.cleanup {
            return result.clone();
        }
        let ownership = self
            .backend
            .as_ref()
            .ok_or("owned session already absent")?
            .ownership()
            .clone();
        let before = self.inspector.observe(self.generation);
        // Release retained native proxies even when pre-stop observation failed.
        // This sends no ID/tag-selected delete operation and needs no live source.
        drop(self.backend.take());
        let after = self.inspector.observe(self.generation);
        let result = (|| {
            let before = before?;
            let after = after?;
            if before.inventory != after.inventory {
                return Err("cleanup inspector inventory changed".into());
            }
            verify_release(&before.graph, &after.graph, &ownership)
        })();
        self.cleanup = Some(result.clone());
        result
    }
}
