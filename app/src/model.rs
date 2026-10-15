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
    pub message: Option<String>,
    pub routing: Status,
}
#[derive(Clone, Debug)]
pub enum Command {
    Select(ApplicationId, String),
    Unselect(ApplicationId),
    Refresh,
}
#[derive(Clone)]
pub struct Request {
    command: Command,
    core: String,
    binding: Option<beacn_link_protocol::catalog::CatalogApplication>,
}
impl Request {
    pub fn from_view(command: Command, snapshot: &Snapshot) -> Result<Self, String> {
        if !snapshot.connected {
            return Err("Application discovery is disconnected.".into());
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
            Command::Refresh => None,
        };
        Ok(Self {
            command,
            core: catalog.core.clone(),
            binding,
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
    pub fn snapshot(&self) -> Snapshot {
        self.snapshot.clone()
    }
    fn sync(&mut self) {
        self.snapshot.catalog = self.service.current().cloned();
        self.snapshot.selected = self.service.selected().clone();
        self.snapshot.revision = self.service.revision();
    }
    pub fn observe(&mut self, catalog: Catalog) -> Result<(), String> {
        let update = self.service.update(catalog)?;
        self.snapshot.connected = true;
        if update.core_changed {
            self.snapshot.message =
                Some("Audio session changed. Select your applications again.".into());
        } else if !update.revoked.is_empty() {
            self.snapshot.message = Some(format!(
                "{} selection(s) removed because an application stopped or changed.",
                update.revoked.len()
            ));
        }
        self.sync();
        Ok(())
    }
    pub fn command(&mut self, command: Command) -> Result<(), String> {
        if !self.snapshot.connected {
            return Err("Application discovery is disconnected.".into());
        }
        let result = match command {
            Command::Select(id, alias) => self.service.select(&id, &alias),
            Command::Unselect(id) => self.service.unselect(&id),
            Command::Refresh => Ok(()),
        };
        self.snapshot.message = Some(match &result {
            Ok(()) => "Selection updated. Routing is off.".into(),
            Err(error) if error.contains("distinct wire names")=>"That published name is already selected. Give each application a different name.".into(),
            Err(error) if error.contains("printable ASCII")=>"Use 1–255 letters, numbers or common English symbols for the published name.".into(),
            Err(error) if error.contains("payload length")=>"These published names are too long together. Shorten a name or select fewer applications.".into(),
            Err(error) => error.clone(),
        });
        self.sync();
        result
    }
    pub fn request(&mut self, request: Request) -> Result<(), String> {
        let current = self.service.current();
        if !self.snapshot.connected
            || current.is_none_or(|c| c.core != request.core)
            || request
                .binding
                .as_ref()
                .is_some_and(|app| current.and_then(|c| c.applications.get(&app.id)) != Some(app))
        {
            let error="Application changed since it was displayed. Review the current list and try again.".to_owned();
            self.snapshot.message = Some(error.clone());
            return Err(error);
        }
        self.command(request.command)
    }
    /// Preserve the last catalog for display but revoke every in-memory selection
    /// on loss of observation. No auto reconnect or same-name rebinding.
    pub fn disconnected(&mut self, error: String) {
        self.service = ApplicationService::default();
        self.snapshot.connected = false;
        self.snapshot.selected.clear();
        self.snapshot.message = Some(format!("Application discovery stopped: {error}"));
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
