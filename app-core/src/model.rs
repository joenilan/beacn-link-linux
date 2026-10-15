use beacn_link_protocol::{
    catalog::{ApplicationService, Catalog},
    planner::{ApplicationId, StreamState},
    routing_control::Status,
};
use std::collections::BTreeMap;

#[derive(Clone, Default)]
pub struct Snapshot {
    pub catalog: Option<Catalog>,
    pub selected: BTreeMap<ApplicationId, String>,
    pub revision: u64,
    pub connected: bool,
    pub busy: bool,
    pub discovery_error: bool,
    pub message: Option<String>,
    pub routing: Status,
    pub link: beacn_link_protocol::link_session::Status,
    pub private_session: bool,
    pub session_available: bool,
    pub auto_connect: bool,
    pub startup: crate::startup::State,
    pub preference_error: Option<String>,
    pub desktop_cleanup: Option<Result<(), String>>,
    pub meter: Option<std::sync::Arc<beacn_link_protocol::metering::Mailbox>>,
}
#[derive(Clone, Debug)]
pub enum Command {
    Select(ApplicationId, String),
    Assign(ApplicationId, String, u8),
    Unselect(ApplicationId),
    Refresh,
    Start,
    Stop,
    AutoConnect(bool),
}
#[derive(Clone)]
pub struct Request {
    command: Command,
    core: String,
    binding: Option<beacn_link_protocol::catalog::CatalogApplication>,
    selected: Option<BTreeMap<ApplicationId, String>>,
    roster: Vec<beacn_link_protocol::catalog::CatalogApplication>,
}
impl Request {
    pub fn from_view(command: Command, snapshot: &Snapshot) -> Result<Self, String> {
        if !snapshot.connected && !matches!(command, Command::Stop | Command::AutoConnect(_)) {
            return Err("Application discovery is disconnected.".into());
        }
        if matches!(command, Command::Stop | Command::AutoConnect(_)) {
            return Ok(Self {
                command,
                core: String::new(),
                binding: None,
                selected: None,
                roster: vec![],
            });
        }
        let catalog = snapshot
            .catalog
            .as_ref()
            .ok_or("No observed application list.")?;
        let binding = match &command {
            Command::Select(id, _) | Command::Unselect(id) => Some(
                catalog
                    .applications
                    .get(id)
                    .ok_or("Application is no longer available.")?
                    .clone(),
            ),
            Command::Assign(id, name, pair) => {
                if !(1..=4).contains(pair) {
                    return Err("Assignment must target Link In 1–4.".into());
                }
                if snapshot.selected.len() > 1
                    || snapshot
                        .selected
                        .iter()
                        .any(|(selected_id, selected_name)| {
                            selected_id != id || selected_name != name
                        })
                {
                    return Err(
                        "Assign one application at a time without changing its published name."
                            .into(),
                    );
                }
                let app = catalog
                    .applications
                    .get(id)
                    .ok_or("Application is no longer available.")?;
                if app.streams.len() != 1 || !crate::model::live(app) {
                    return Err("Assignment requires one live application stream.".into());
                }
                Some(app.clone())
            }
            Command::Refresh | Command::Start | Command::Stop | Command::AutoConnect(_) => None,
        };
        let selected = match &command {
            Command::Start | Command::Assign(..) => Some(snapshot.selected.clone()),
            _ => None,
        };
        let roster = if selected.is_some() {
            let mut roster = snapshot
                .selected
                .keys()
                .map(|id| {
                    catalog
                        .applications
                        .get(id)
                        .cloned()
                        .ok_or("Selected application disappeared.")
                })
                .collect::<Result<Vec<_>, _>>()?;
            if let Command::Assign(id, _, _) = &command {
                let target = catalog
                    .applications
                    .get(id)
                    .ok_or("Application is no longer available.")?;
                if !roster.iter().any(|app| app.id == target.id) {
                    roster.push(target.clone());
                }
            }
            roster
        } else {
            vec![]
        };
        Ok(Self {
            roster,
            command,
            core: catalog.core.clone(),
            binding,
            selected,
        })
    }
}
/// Sole selection owner. Every command is preceded by a fresh native observation
/// in the worker; this model never opens a backend or grants apply authority.
#[derive(Default)]
pub struct Model {
    service: ApplicationService,
    snapshot: Snapshot,
}
impl Model {
    pub fn meter(&mut self, meter: Option<std::sync::Arc<beacn_link_protocol::metering::Mailbox>>) {
        self.snapshot.meter = meter;
    }
    pub fn desktop_cleanup(&mut self, cleanup: Option<Result<(), String>>) {
        self.snapshot.desktop_cleanup = cleanup;
    }
    pub fn startup(&mut self, supervisor: &crate::startup::Supervisor, error: Option<String>) {
        self.snapshot.auto_connect = supervisor.automatic;
        self.snapshot.startup = supervisor.state;
        self.snapshot.preference_error = error;
    }
    pub fn service(&self) -> &ApplicationService {
        &self.service
    }
    pub fn session(
        &mut self,
        status: beacn_link_protocol::link_session::Status,
        available: bool,
        private: bool,
    ) {
        if status.phase != beacn_link_protocol::link_session::Phase::Running {
            self.snapshot.meter = None;
        }
        if status.phase != self.snapshot.link.phase {
            self.snapshot.message = status.error.clone().or_else(|| match status.phase {
                beacn_link_protocol::link_session::Phase::Running => Some(
                    if private {
                        "Private test session active. Changing the selection stops this session."
                    } else {
                        "Link session active."
                    }
                    .into(),
                ),
                beacn_link_protocol::link_session::Phase::Stopped => {
                    Some("Link stopped. Owned routes released.".into())
                }
                beacn_link_protocol::link_session::Phase::NeedsReview => {
                    Some("Cleanup needs review. Restart is blocked.".into())
                }
                _ => None,
            });
        }
        self.snapshot.routing = status.routes.clone();
        self.snapshot.link = status;
        self.snapshot.private_session = private;
        self.snapshot.session_available = available;
    }
    pub fn notice(&mut self, error: String) {
        self.snapshot.message = Some(error);
    }
    pub fn snapshot(&self) -> Snapshot {
        self.snapshot.clone()
    }
    fn sync(&mut self) {
        self.snapshot.catalog = self.service.current().cloned();
        self.snapshot.selected = self.service.selected().clone();
        self.snapshot.revision = self.service.revision();
    }
    pub fn observe(&mut self, catalog: Catalog) -> Result<(), String> {
        // Metadata only, emitted solely on a revoked selection. Retain both
        // identities so a stream restart is distinguishable from a UI Remove.
        let previous = self.service.current().cloned();
        let update = self.service.update(catalog)?;
        self.snapshot.connected = true;
        self.snapshot.busy = false;
        self.snapshot.discovery_error = false;
        if update.core_changed {
            self.snapshot.message =
                Some("Audio session changed. Select your applications again.".into());
        } else if !update.revoked.is_empty() {
            for id in &update.revoked {
                eprintln!(
                    "LinX selection revoked: id={:?} before={:?} after={:?}",
                    id,
                    previous
                        .as_ref()
                        .and_then(|c| c.applications.get(id))
                        .map(|a| &a.streams),
                    self.service
                        .current()
                        .and_then(|c| c.applications.get(id))
                        .map(|a| &a.streams)
                );
            }
            self.snapshot.message = Some(format!(
                "{} selection(s) removed because an application stopped or changed.",
                update.revoked.len()
            ));
        }
        self.sync();
        Ok(())
    }
    pub fn command(&mut self, command: Command) -> Result<(), String> {
        eprintln!("LinX selection command: {command:?}");
        if !self.snapshot.connected {
            return Err("Application discovery is disconnected.".into());
        }
        let result = match command {
            Command::Select(id, alias) => self.service.select(&id, &alias),
            Command::Unselect(id) => self.service.unselect(&id),
            Command::Refresh => Ok(()),
            Command::Assign(..) | Command::Start | Command::Stop | Command::AutoConnect(_) => {
                Err("Session commands require the routing owner.".into())
            }
        };
        self.snapshot.message = Some(match &result {
            Ok(()) => "Selection updated.".into(),
            Err(error) if error.contains("distinct wire names")=>"That published name is already selected. Give each application a different name.".into(),
            Err(error) if error.contains("printable ASCII")=>"Use 1–255 letters, numbers or common English symbols for the published name.".into(),
            Err(error) if error.contains("payload length")=>"These published names are too long together. Shorten a name or select fewer applications.".into(),
            Err(error) => error.clone(),
        });
        self.sync();
        result
    }
    pub fn accept(&mut self, request: Request) -> Result<Command, String> {
        if matches!(request.command, Command::Stop | Command::AutoConnect(_)) {
            return Ok(request.command);
        }
        let current = self.service.current();
        if !self.snapshot.connected
            || current.is_none_or(|c| c.core != request.core)
            || request
                .binding
                .as_ref()
                .is_some_and(|app| current.and_then(|c| c.applications.get(&app.id)) != Some(app))
            || request
                .roster
                .iter()
                .any(|app| current.and_then(|c| c.applications.get(&app.id)) != Some(app))
            || request
                .selected
                .as_ref()
                .is_some_and(|selected| selected != self.service.selected())
        {
            let error="Application changed since it was displayed. Review the current list and try again.".to_owned();
            self.snapshot.message = Some(error.clone());
            return Err(error);
        }
        Ok(request.command)
    }
    pub fn request(&mut self, request: Request) -> Result<(), String> {
        let command = self.accept(request)?;
        self.command(command)
    }
    /// Preserve the last catalog for display but revoke every in-memory selection
    /// on loss of observation. Reconnection never rebinds old selections by name.
    pub fn disconnected(&mut self, error: String) {
        self.service = ApplicationService::default();
        self.snapshot.connected = false;
        self.snapshot.discovery_error = true;
        self.snapshot.selected.clear();
        self.snapshot.message = Some(format!(
            "Application discovery unavailable; retrying: {error}"
        ));
        self.snapshot.busy = false;
    }
}
pub fn label(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}
pub fn app_name(app: &beacn_link_protocol::catalog::CatalogApplication) -> String {
    app.names
        .iter()
        .next()
        .or_else(|| app.binaries.iter().next())
        .map(|s| label(s))
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("Application {}", app.pid))
}
pub fn live(app: &beacn_link_protocol::catalog::CatalogApplication) -> bool {
    !app.streams.is_empty() && app.streams.iter().all(|s| s.state == StreamState::Live)
}
pub fn default_alias(app: &beacn_link_protocol::catalog::CatalogApplication) -> String {
    let name = app_name(app);
    if name.len() <= 80 && name.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
        name
    } else {
        format!("App {}", app.pid)
    }
}
