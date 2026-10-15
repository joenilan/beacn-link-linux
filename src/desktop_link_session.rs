//! Explicit bounded desktop owner. Private receipt/zero semantics stay separate.
use crate::{
    catalog::ApplicationService,
    lifecycle::InventoryTracker,
    link_session::{OpenFailure, Options, OwnedControlTransport, Phase},
    native_desktop_target::{DesktopRoutePublication, DesktopTarget},
    planner::{self, Inventory, Outcome},
    session::{PollReply, Session},
};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

/// A native lease must mint its own fresh sealed proof and release only its handles.
pub trait DesktopLease {
    fn publication(&mut self, inventory: &Inventory) -> Result<DesktopRoutePublication, String>;
    fn apply_first_pair(&mut self) -> Result<(), String>;
    fn stop(&mut self) -> Result<(), String>;
}
/// One selected existing desktop stream and one explicitly selected Studio.
/// Construction opens only the catalog-pinned desktop connection; audio waits
/// for a received, planner-resolved first-pair assignment.
pub struct StudioDesktopLease {
    target: DesktopTarget,
    sysfs: PathBuf,
    expected: Vec<u8>,
    hardware: crate::executor::ObjectRef,
}
impl StudioDesktopLease {
    pub fn connect(
        service: &ApplicationService,
        sysfs: &Path,
        expected: &[u8],
        hardware: crate::executor::ObjectRef,
        authorized: bool,
    ) -> Result<Self, String> {
        if !authorized {
            return Err("Desktop Studio lease requires explicit authority.".into());
        }
        let selected = service.selected();
        if selected.len() != 1 {
            return Err("Desktop qualification needs one selected application.".into());
        }
        let (id, name) = selected.first_key_value().unwrap();
        crate::usb_handshake::check_descriptors(expected).map_err(|e| e.to_string())?;
        let target = DesktopTarget::connect_named(
            service.current().ok_or("Catalog absent.")?,
            id,
            name,
            true,
        )?;
        Ok(Self {
            target,
            sysfs: sysfs.to_owned(),
            expected: expected.to_vec(),
            hardware,
        })
    }
    pub fn server_pid(&self) -> u32 {
        self.target.server_pid()
    }
    pub fn physical_stats(&self) -> Option<(u32, u64)> {
        self.target.physical_stats()
    }
}
impl DesktopLease for StudioDesktopLease {
    fn publication(&mut self, inventory: &Inventory) -> Result<DesktopRoutePublication, String> {
        self.target.publication(inventory)
    }
    fn apply_first_pair(&mut self) -> Result<(), String> {
        self.target.start()?;
        self.target
            .attach_studio(&self.sysfs, &self.expected, self.hardware, true)
    }
    fn stop(&mut self) -> Result<(), String> {
        self.target.stop()
    }
}
#[derive(Debug, Clone)]
pub struct Status {
    pub phase: Phase,
    pub completed_cycles: usize,
    pub cycle_limit: usize,
    pub assignment: Option<u8>,
    pub last_reply: Option<&'static str>,
    pub error: Option<String>,
    pub route_cleanup: Option<Result<(), String>>,
    pub transport_cleanup: Option<Result<(), String>>,
}
impl Default for Status {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            completed_cycles: 0,
            cycle_limit: 0,
            assignment: None,
            last_reply: None,
            error: None,
            route_cleanup: None,
            transport_cleanup: None,
        }
    }
}
struct Active<L, T> {
    routes: L,
    control: Session<T>,
    core: String,
    selection_revision: u64,
    tracker: InventoryTracker,
    options: Options,
}
pub struct DesktopLinkSession<L: DesktopLease, T: OwnedControlTransport> {
    active: Option<Active<L, T>>,
    status: Status,
}
impl<L: DesktopLease, T: OwnedControlTransport> Default for DesktopLinkSession<L, T> {
    fn default() -> Self {
        Self {
            active: None,
            status: Status::default(),
        }
    }
}
impl<L: DesktopLease, T: OwnedControlTransport> Drop for DesktopLinkSession<L, T> {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
impl<L: DesktopLease, T: OwnedControlTransport> DesktopLinkSession<L, T> {
    pub fn status(&self) -> &Status {
        &self.status
    }
    pub fn routes(&self) -> Option<&L> {
        self.active.as_ref().map(|a| &a.routes)
    }
    pub fn start(
        &mut self,
        service: &ApplicationService,
        options: Options,
        factory: impl FnOnce() -> Result<L, String>,
        transport: impl FnOnce(&L, &Inventory, Options) -> Result<T, OpenFailure>,
    ) -> Result<(), String> {
        if matches!(self.status.phase, Phase::Running | Phase::NeedsReview) {
            return Err("Desktop session running or cleanup unverified.".into());
        }
        options.validate()?;
        let inventory = service.inventory().ok_or("Desktop selection absent.")?;
        if inventory.applications.len() != 1 || inventory.applications[0].streams.len() != 1 {
            return Err("Desktop session needs one selected live stream.".into());
        }
        let core = service
            .current()
            .ok_or("Desktop catalog absent.")?
            .core
            .clone();
        let mut tracker = InventoryTracker::default();
        tracker.advance(inventory).map_err(|e| e.to_string())?;
        self.status = Status {
            cycle_limit: options.cycles,
            ..Status::default()
        };
        let mut routes = match factory() {
            Ok(routes) => routes,
            Err(error) => {
                self.status.phase = Phase::Stopped;
                self.status.error = Some(error.clone());
                return Err(error);
            }
        };
        let proof = match routes.publication(inventory) {
            Ok(p)
                if p.valid()
                    && p.inventory() == inventory
                    && p.records().len() == 1
                    && p.records()[0].assignment.byte() == 0 =>
            {
                p
            }
            result => {
                let error = result
                    .err()
                    .unwrap_or("Desktop initial proof is not original output.".into());
                self.status.error = Some(error.clone());
                self.status.route_cleanup = Some(routes.stop());
                self.status.phase = if self.status.route_cleanup.as_ref().unwrap().is_ok() {
                    Phase::Stopped
                } else {
                    Phase::NeedsReview
                };
                return Err(error);
            }
        };
        let opened = match transport(&routes, proof.inventory(), options) {
            Ok(t) => t,
            Err(f) => {
                self.status.error = Some(f.message.clone());
                self.status.transport_cleanup = Some(f.cleanup);
                self.status.route_cleanup = Some(routes.stop());
                self.status.phase = if self.status.route_cleanup.as_ref().unwrap().is_ok()
                    && self.status.transport_cleanup.as_ref().unwrap().is_ok()
                {
                    Phase::Stopped
                } else {
                    Phase::NeedsReview
                };
                return Err(f.message);
            }
        };
        self.active = Some(Active {
            routes,
            control: Session::new_with_poll_policy(
                opened,
                crate::usb_handshake::MAX_TIMEOUT,
                options.poll_policy,
            )
            .expect("fixed timeout"),
            core,
            selection_revision: service.selection_revision(),
            tracker,
            options,
        });
        self.status.phase = Phase::Running;
        self.status.assignment = Some(0);
        if let Err(e) = self
            .active
            .as_mut()
            .unwrap()
            .control
            .initialize_with_read_timeout(options.initialization_timeout)
        {
            return self.fail(e.to_string());
        }
        Ok(())
    }
    fn current(&self, service: &ApplicationService) -> Result<(), String> {
        let a = self
            .active
            .as_ref()
            .ok_or("Desktop session is not running.")?;
        if service.current().map(|c| &c.core) != Some(&a.core)
            || service.selection_revision() != a.selection_revision
            || service.inventory().map(|i| &i.applications)
                != a.tracker.current().map(|i| &i.applications)
        {
            return Err(
                "Desktop selected roster or core changed; explicit recovery required.".into(),
            );
        }
        Ok(())
    }
    pub fn check(&mut self, service: &ApplicationService) -> Result<(), String> {
        if self.status.phase != Phase::Running {
            return Ok(());
        }
        let result = (|| {
            self.current(service)?;
            self.active
                .as_mut()
                .unwrap()
                .routes
                .publication(service.inventory().unwrap())?;
            Ok(())
        })();
        match result {
            Ok(()) => Ok(()),
            Err(e) => self.fail(e),
        }
    }
    /// One serial native/USB cycle, with the same 500-ms diagnostic cadence as
    /// existing attended checks. Run on the service worker, never the UI thread.
    pub fn tick(&mut self, service: &ApplicationService) -> Result<(), String> {
        if self.status.phase != Phase::Running {
            return Err("Desktop session is not running.".into());
        }
        let result = (|| {
            self.current(service)?;
            let inventory = service.inventory().unwrap();
            let a = self.active.as_mut().unwrap();
            a.tracker.advance(inventory).map_err(|e| e.to_string())?;
            let proof = a.routes.publication(inventory)?;
            a.control
                .refresh_desktop_publication(&proof)
                .map_err(|e| e.to_string())?;
            a.control
                .publish(proof.records())
                .map_err(|e| e.to_string())?;
            std::thread::sleep(Duration::from_millis(500));
            let reply = a.control.poll().map_err(|e| e.to_string())?;
            let fresh = a.routes.publication(inventory)?;
            let mut assignment = fresh.records()[0].assignment.byte();
            let label = match reply {
                PollReply::NoPayload { .. } => "no payload",
                PollReply::RejectedMarker { .. } => "rejected marker",
                PollReply::Records { records, .. } => {
                    let records = records
                        .iter()
                        .map(|r| a.options.names.for_planning(r))
                        .collect::<Vec<_>>();
                    let plan = planner::plan(inventory, &records).map_err(|e| e.to_string())?;
                    a.tracker
                        .require_current_snapshot(&plan)
                        .map_err(|e| e.to_string())?;
                    if records.is_empty() {
                        "no assignments"
                    } else if plan
                        .resolutions
                        .iter()
                        .any(|r| !matches!(r.outcome, Outcome::Proposed(_)))
                    {
                        "assignment blocked"
                    } else {
                        let routes = plan.proposed_routes().collect::<Vec<_>>();
                        if routes.len() != 1 || routes[0].link.assignment() != 1 {
                            return Err(
                                "Desktop physical qualification supports first Link In only."
                                    .into(),
                            );
                        }
                        if assignment == 0 {
                            a.routes.apply_first_pair()?;
                            assignment = a.routes.publication(inventory)?.records()[0]
                                .assignment
                                .byte();
                            if assignment != 1 {
                                return Err("Desktop applied assignment not verified.".into());
                            }
                        }
                        "assignment applied and verified"
                    }
                }
            };
            Ok((assignment, label))
        })();
        match result {
            Ok((assignment, label)) => {
                self.status.assignment = Some(assignment);
                self.status.last_reply = Some(label);
                self.status.completed_cycles += 1;
                if self.status.completed_cycles == self.status.cycle_limit {
                    self.stop()?;
                }
                Ok(())
            }
            Err(e) => self.fail(e),
        }
    }
    fn fail<R>(&mut self, error: String) -> Result<R, String> {
        self.status.error = Some(error.clone());
        let cleanup = self.stop();
        Err(match cleanup {
            Ok(()) => error,
            Err(e) => format!("{error}; {e}"),
        })
    }
    pub fn stop(&mut self) -> Result<(), String> {
        if let Some(mut a) = self.active.take() {
            // Physical and desktop output are restored before releasing USB.
            self.status.route_cleanup = Some(a.routes.stop());
            a.control.close();
            self.status.transport_cleanup = Some(a.control.into_transport().release());
            self.status.assignment = None;
            self.status.phase = if self.status.route_cleanup.as_ref().unwrap().is_ok()
                && self.status.transport_cleanup.as_ref().unwrap().is_ok()
            {
                Phase::Stopped
            } else {
                Phase::NeedsReview
            };
        }
        if self.status.phase == Phase::NeedsReview {
            Err("Desktop cleanup unverified; restart blocked.".into())
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
