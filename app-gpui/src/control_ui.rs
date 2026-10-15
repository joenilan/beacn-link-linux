//! Named controls dispatch through GPUI's own event handlers, never OS input.
use super::Interface;
use gpui_kit::{test::TestWindowExt, *};
use serde_json::{Value, json};
use std::io::Write;

pub(super) fn dispatch(
    view: &WeakEntity<Interface>,
    request: crate::control::Request,
    window: &mut Window,
    cx: &mut App,
) {
    if request.expired() {
        return;
    }
    let result = execute(view, &request.value, window, cx);
    let quit = request.op() == "quit" && result.is_ok();
    let response = match result {
        Ok(value) => json!({"ok":true,"value":value}),
        Err(error) => json!({"ok":false,"error":error}),
    };
    let _ = request.respond(response);
    if quit {
        cx.quit();
    }
}

fn execute(
    view: &WeakEntity<Interface>,
    request: &Value,
    window: &mut Window,
    cx: &mut App,
) -> Result<Value, String> {
    let op = request["op"].as_str().ok_or("Missing op")?;
    window.render_frame(cx);
    match op {
        "state" | "components" => view
            .update(cx, |this, cx| this.control_state(window, cx))
            .map_err(|e| e.to_string()),
        "click" | "text" | "scroll" => {
            let id = request["target"]
                .as_str()
                .ok_or("Missing target component ID")?;
            let allowed = view
                .update(cx, |this, _| {
                    this.control_ids().iter().any(|candidate| candidate == id)
                })
                .map_err(|e| e.to_string())?;
            if !allowed {
                return Err("Unknown component ID".into());
            }
            if !view.update(cx,|this,cx|this.control_enabled(id,cx)).map_err(|e|e.to_string())? {
                return Err("Component is disabled".into());
            }
            let element = window
                .try_find(SharedString::from(id.to_owned()))
                .ok_or("Component is not rendered; open its drawer first")?;
            if !element.visible() {
                return Err("Component is not visible; scroll or expand its drawer first".into());
            }
            if element.disabled() == Some(true) {
                return Err("Component is disabled".into());
            }
            match op {
                "click" => window.click(SharedString::from(id.to_owned()), cx),
                "text" => {
                    if id != "search-input" && !id.starts_with("name-") {
                        return Err("Component is not an editable text field".into());
                    }
                    let value = request["text"].as_str().ok_or("Missing text")?;
                    if value.len() > 1024 || value.chars().any(|c| c.is_control()) {
                        return Err("Text exceeds bound or contains control characters".into());
                    }
                    window.click(SharedString::from(id.to_owned()), cx);
                    window.press("ctrl-a", cx);
                    if value.is_empty() {
                        window.press("backspace", cx);
                    } else {
                        window.input(value, cx);
                    }
                }
                _ => {
                    let delta = request["delta"]
                        .as_f64()
                        .filter(|d| d.is_finite() && d.abs() <= 2000.)
                        .ok_or("Scroll delta must be within -2000..2000")?;
                    window.scroll(
                        SharedString::from(id.to_owned()),
                        ScrollDelta::Pixels(point(px(0.), px(delta as f32))),
                        cx,
                    );
                }
            }
            Ok(
                json!({"dispatched":true,"target":id,"completion":"Read state for asynchronous worker results"}),
            )
        }
        "resize" => {
            let w = request["width"]
                .as_f64()
                .filter(|w| *w >= 360. && *w <= 1600.)
                .ok_or("Width must be 360..1600")?;
            let h = request["height"]
                .as_f64()
                .filter(|h| *h >= 400. && *h <= 1200.)
                .ok_or("Height must be 400..1200")?;
            window.resize(size(px(w as f32), px(h as f32)));
            Ok(json!({"dispatched":true}))
        }
        "screenshot" => {
            let path =
                crate::control::screenshot_path().ok_or("Control capture is not configured")?;
            let bytes = crate::frame::ppm(window)?;
            // A controller must collect/remove the previous frame before the
            // next request; never overwrite an existing or replaced artifact.
            use std::os::unix::fs::{MetadataExt,OpenOptionsExt};
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
                .map_err(|e| e.to_string())?;
            let stamp=file.metadata().map_err(|e|e.to_string())?;
            if let Err(error)=file.write_all(&bytes) {
                if std::fs::symlink_metadata(&path).is_ok_and(|m|m.is_file() && (m.dev(),m.ino())==(stamp.dev(),stamp.ino())) {
                    let _=std::fs::remove_file(&path);
                }
                return Err(error.to_string());
            }
            Ok(json!({"path":path,"format":"ppm","bytes":bytes.len(),"own_window_only":true}))
        }
        "quit" => {
            view.update(cx, |this, _| this.worker.stop())
                .map_err(|e| e.to_string())?;
            Ok(json!({"worker_joined":true}))
        }
        "pause" => {
            view.update(cx,|this,cx|this.send(beacn_linx_core::model::Command::Stop,cx)).map_err(|e|e.to_string())?;
            Ok(json!({"dispatched":true,"completion":"Read state until startup is Paused"}))
        }
        _ => Err(
            "Unknown op. Use state, components, click, text, scroll, resize, screenshot, pause or quit."
                .into(),
        ),
    }
}

impl Interface {
    fn control_enabled(&self, id: &str, cx: &App) -> bool {
        use beacn_link_protocol::link_session::Phase;
        if id == "mini-view" {
            return false;
        }
        if id == "session" {
            let waiting = self.snapshot.waiting.is_some()
                || self.snapshot.startup == beacn_linx_core::startup::State::WaitingAudio;
            return self.snapshot.link.phase == Phase::Running
                || waiting
                || (self.snapshot.session_available
                    && self.snapshot.connected
                    && !self.snapshot.selected.is_empty()
                    && self.snapshot.link.phase != Phase::NeedsReview);
        }
        if let Some(rest) = id.strip_prefix("route-") {
            let Some((pair, raw)) = rest.split_once('-') else {
                return false;
            };
            if !matches!(pair, "1" | "2" | "3" | "4")
                || !self.studio_app
                || self.snapshot.link.phase == Phase::NeedsReview
            {
                return false;
            }
            let key = beacn_link_protocol::planner::ApplicationId(raw.into());
            let Some(app) = self
                .snapshot
                .catalog
                .as_ref()
                .and_then(|c| c.applications.get(&key))
            else {
                return false;
            };
            return self.snapshot.connected
                && beacn_linx_core::model::live(app)
                && app.streams.len() == 1
                && self.snapshot.waiting.as_ref().is_none_or(|w| w.id == key)
                && (self.snapshot.selected.is_empty()
                    || (self.snapshot.selected.len() == 1
                        && self.snapshot.selected.contains_key(&key)));
        }
        for prefix in ["select-", "save-", "name-"] {
            if let Some(raw) = id.strip_prefix(prefix) {
                let key = beacn_link_protocol::planner::ApplicationId(raw.into());
                let Some(app) = self
                    .snapshot
                    .catalog
                    .as_ref()
                    .and_then(|c| c.applications.get(&key))
                else {
                    return false;
                };
                let waiting_remove = prefix == "select-"
                    && self.snapshot.waiting.as_ref().is_some_and(|w| w.id == key);
                if !self.snapshot.connected
                    || (!beacn_linx_core::model::live(app) && !waiting_remove)
                {
                    return false;
                }
                if prefix == "select-" {
                    return !self.studio_app
                        || self.snapshot.selected.contains_key(&key)
                        || self.snapshot.waiting.as_ref().is_some_and(|w| w.id == key);
                }
                if prefix == "save-" {
                    let changed = self.names.get(&key).is_some_and(|input| {
                        self.snapshot
                            .selected
                            .get(&key)
                            .is_none_or(|alias| input.read(cx).value().as_ref() != alias)
                    });
                    return changed
                        && (!self.studio_app
                            || (!self.snapshot.selected.contains_key(&key)
                                && self.snapshot.selected.is_empty()
                                && app.streams.len() == 1
                                && self.snapshot.link.phase != Phase::NeedsReview
                                && self.snapshot.waiting.as_ref().is_none_or(|w| w.id == key)));
                }
            }
        }
        true
    }
    fn control_ids(&self) -> Vec<String> {
        let mut ids = [
            "session",
            "settings",
            "applications",
            "applications-2",
            "applications-3",
            "applications-4",
            "applications-settings",
            "auto-connect",
            "theme-dark",
            "theme-light",
            "accent-mint",
            "accent-blue",
            "accent-violet",
            "accent-amber",
            "accent-rose",
            "search-input",
            "application-list",
            "mini-view",
        ]
        .map(str::to_owned)
        .to_vec();
        if let Some(catalog) = &self.snapshot.catalog {
            for id in catalog.applications.keys() {
                if !self.studio_app
                    || self.snapshot.selected.contains_key(id)
                    || self.snapshot.waiting.as_ref().is_some_and(|w| w.id == *id)
                {
                    ids.push(format!("select-{}", id.0));
                }
                for prefix in ["details", "name", "save"] {
                    ids.push(format!("{prefix}-{}", id.0));
                }
                for pair in 1..=4 {
                    ids.push(format!("route-{pair}-{}", id.0));
                }
            }
        }
        ids
    }
    fn control_state(&self, window: &Window, cx: &App) -> Value {
        let s = &self.snapshot;
        let meter = (s.waiting.is_none()
            && s.startup != beacn_linx_core::startup::State::WaitingAudio)
            .then(|| self.meter)
            .flatten()
            .map(|(pair, levels)| {
                json!({
                    "pair":pair,"rms_dbfs":levels.rms_db,"peak_dbfs":levels.peak_db,
                    "hold_dbfs":levels.hold_db,
                    "sequence":self.meter_sample.map(|r|r.sequence),
                    "frames":self.meter_sample.map(|r|r.frames),
                    "peak_linear":self.meter_sample.map(|r|r.peak),
                    "rms_linear":self.meter_sample.map(|r|r.rms),
                    "aggregate_only":true
                })
            });
        let components=self.control_ids().into_iter().filter_map(|id|window.try_find(SharedString::from(id.clone())).map(|e| {
            let b=e.bounds();json!({"id":id,"label":e.label(),"value":e.value(),"disabled":e.disabled().unwrap_or(!self.control_enabled(&id,cx)),"checked":if id=="auto-connect" {Some(s.auto_connect)} else {e.checked()},"visible":e.visible(),"bounds":{"x":f32::from(b.origin.x),"y":f32::from(b.origin.y),"width":f32::from(b.size.width),"height":f32::from(b.size.height)}})
        })).collect::<Vec<_>>();
        let apps=s.catalog.as_ref().into_iter().flat_map(|c|c.applications.values()).map(|a|json!({"id":a.id.0,"name":beacn_linx_core::model::app_name(a),"pid":a.pid,"streams":a.streams.iter().map(|t|json!({"id":t.id.0,"state":format!("{:?}",t.state)})).collect::<Vec<_>>(),"selected":s.selected.get(&a.id),"assignment":s.routing.assignments.get(&a.id)})).collect::<Vec<_>>();
        let waiting = s
            .waiting
            .as_ref()
            .map(|w| json!({"id":w.id.0,"name":w.name,"pair":w.pair}));
        json!({"pid":std::process::id(),"revision":s.revision,"connected":s.connected,"phase":format!("{:?}",s.link.phase),"startup":format!("{:?}",s.startup),"error":s.link.error,"message":s.message,"last_reply":s.link.last_reply,"cycles":s.link.completed_cycles,"meter":meter,"waiting":waiting,"transport_cleanup":format!("{:?}",s.link.transport_cleanup),"desktop_cleanup":format!("{:?}",s.desktop_cleanup),"auto_connect":s.auto_connect,"applications":apps,"components":components,"drawers":{"applications":self.catalog_open,"settings":self.settings},"search":self.search.read(cx).value().to_string(),"appearance":{"light":self.appearance.light,"accent":self.appearance.accent.label()},"size":{"width":f32::from(window.viewport_size().width),"height":f32::from(window.viewport_size().height)}})
    }
}
