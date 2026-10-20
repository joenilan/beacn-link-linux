//! Native control for the Link Console. Named controls dispatch through
//! GPUI's own event handlers, never OS input.
use super::{LinkInterface, key_id};
use beacn_linx_core::link::LinkCommand;
use gpui_kit::{test::TestWindowExt, *};
use serde_json::{Value, json};
use std::io::Write;

pub(super) fn dispatch(
    view: &WeakEntity<LinkInterface>,
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
    view: &WeakEntity<LinkInterface>,
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
                .update(cx, |this, _| this.control_ids().iter().any(|c| c == id))
                .map_err(|e| e.to_string())?;
            if !allowed {
                return Err("Unknown component ID".into());
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
                    if id != "search-input" {
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
            Ok(json!({"dispatched":true,"target":id,"completion":"Read state for worker results"}))
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
            use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
                .map_err(|e| e.to_string())?;
            let stamp = file.metadata().map_err(|e| e.to_string())?;
            if let Err(error) = file.write_all(&bytes) {
                if std::fs::symlink_metadata(&path)
                    .is_ok_and(|m| m.is_file() && (m.dev(), m.ino()) == (stamp.dev(), stamp.ino()))
                {
                    let _ = std::fs::remove_file(&path);
                }
                return Err(error.to_string());
            }
            Ok(json!({"path":path,"format":"ppm","bytes":bytes.len(),"own_window_only":true}))
        }
        "quit" => {
            let _ = view;
            super::LinkCore::stop(cx);
            Ok(json!({"worker_joined":true}))
        }
        "close" => {
            // Same decision as the title bar's close button.
            let closed = super::close_requested(window, cx);
            if closed {
                window.remove_window();
            }
            Ok(json!({"dispatched":true,"closed_to_tray":closed,"minimized":!closed}))
        }
        "pause" | "connect" => {
            let command = if op == "pause" {
                LinkCommand::Pause
            } else {
                LinkCommand::Connect
            };
            view.update(cx, |this, cx| this.send(command, cx))
                .map_err(|e| e.to_string())?;
            Ok(json!({"dispatched":true,"completion":"Read state until phase changes"}))
        }
        "place" | "place-all" => {
            // What a drop on the Sound map sends: "1".."4", "device:<node.name>" or "loose".
            let target = match request["target"].as_str().ok_or("Missing target")? {
                "loose" => None,
                t if t.starts_with("device:") => {
                    Some(beacn_linx_core::link::Target::Device(t["device:".len()..].to_owned()))
                }
                t => Some(beacn_linx_core::link::Target::Channel(
                    t.parse().map_err(|_| "Target is 1..4, device:<name> or loose")?,
                )),
            };
            let command = if op == "place" {
                let key = request["key"].as_str().ok_or("Missing key")?.to_owned();
                LinkCommand::Place(key, target)
            } else {
                LinkCommand::PlaceAll(target)
            };
            view.update(cx, |this, cx| this.send(command, cx))
                .map_err(|e| e.to_string())?;
            Ok(json!({"dispatched":true}))
        }
        "map-state" | "map-input" | "map-key" | "map-minimal" | "map-devices" => crate::sound_map::control(op, request, cx),
        "map" => {
            crate::sound_map::toggle(None, cx);
            Ok(json!({"open":cx.global::<super::LinkCore>().map_window.is_some()}))
        }
        _ => Err(
            "Unknown op. Use state, components, click, text, scroll, resize, screenshot, pause, connect, place, place-all, map or quit."
                .into(),
        ),
    }
}

impl LinkInterface {
    fn control_ids(&self) -> Vec<String> {
        let mut ids = [
            "session",
            "settings",
            "applications",
            "applications-2",
            "applications-3",
            "applications-4",
            "auto-connect",
            "launch-at-login",
            "theme-dark",
            "theme-light",
            "quit-app",
            "accent-mint",
            "accent-blue",
            "accent-violet",
            "accent-amber",
            "accent-rose",
            "search-input",
            "application-list",
            "close-applications",
            "close-settings",
            "applications-scrim",
            "settings-scrim",
            "update-checks",
            "check-updates",
            "update",
            "updates-scrim",
            "close-updates",
            "update-download",
            "update-install",
            "update-restart",
            "manage-apps",
            "outputs",
            "outputs-scrim",
            "close-outputs",
            "outputs-edit",
            "dev-badge",
            "local-banner",
            "use-linx",
            "minimize-window",
            "close-window",
        ]
        .map(str::to_owned)
        .to_vec();
        for channel in 1..=4 {
            ids.push(format!("fader-{channel}"));
            ids.push(format!("mute-{channel}"));
        }
        for output in &self.view.outputs {
            ids.push(format!("output-{}", key_id(&output.name)));
            ids.push(format!("output-visibility-{}", key_id(&output.name)));
        }
        for app in &self.view.apps {
            let id = key_id(&app.key);
            ids.push(format!("app-{id}"));
            for channel in 0..=4 {
                ids.push(format!("route-{channel}-{id}"));
            }
        }
        ids
    }
    fn control_state(&self, window: &Window, cx: &App) -> Value {
        let v = &self.view;
        let components = self
            .control_ids()
            .into_iter()
            .filter_map(|id| {
                window.try_find(SharedString::from(id.clone())).map(|e| {
                    let b = e.bounds();
                    json!({"id":id,"label":e.label(),"disabled":e.disabled().unwrap_or(false),
                        "checked":if id=="auto-connect" {Some(v.auto_connect)} else {e.checked()},
                        "visible":e.visible(),
                        "bounds":{"x":f32::from(b.origin.x),"y":f32::from(b.origin.y),
                                  "width":f32::from(b.size.width),"height":f32::from(b.size.height)}})
                })
            })
            .collect::<Vec<_>>();
        let apps = v
            .apps
            .iter()
            .map(|a| {
                json!({"key":a.key,"id":key_id(&a.key),"name":a.name,"channel":a.channel,
                       "remembered":a.remembered,"running":a.running,"streams":a.streams,
                       "pinned":format!("{:?}", a.pinned)})
            })
            .collect::<Vec<_>>();
        let meters = self
            .meters
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let level = v.levels[i];
                json!({"channel":i+1,"display":m.display.map(|d| json!({
                    "rms_dbfs":d.rms_db,"peak_dbfs":d.peak_db,"hold_dbfs":d.hold_db})),
                    "level":{"known":level.known,"volume":level.volume,"mute":level.mute},
                    "link_out":self.out_meters[i].display.map(|d| json!({
                        "rms_dbfs":d.rms_db,"peak_dbfs":d.peak_db,"hold_dbfs":d.hold_db}))})
            })
            .collect::<Vec<_>>();
        let update = cx.global::<super::LinkCore>().updates.view();
        json!({
            "pid":std::process::id(),
            "mode":"link",
            "update":{"phase":format!("{:?}",update.phase),"mode":format!("{:?}",update.mode),
                "current":update.current,"latest":update.latest,"error":update.error,
                "kind":format!("{:?}",update.kind),"notes":update.notes.len()},
            "phase":format!("{:?}",v.phase),
            "status":v.status(),
            "studio":v.studio,
            "transport":format!("{:?}",v.transport),
            "audio_ready":v.audio_ready,
            "control":format!("{:?}",v.control),
            "host":v.host,
            "control_error":v.control_error,
            "audio_error":v.audio_error,
            "exchanges":v.exchanges,
            "beacn":{
                "last_reply":v.last_reply,
                "host_seen_at_ms":v.beacn.host_seen_at_ms,
                "published":v.beacn.published.iter()
                    .map(|(name, channel)| json!({"name":name,"channel":channel}))
                    .collect::<Vec<_>>(),
                "received":v.beacn.received.iter()
                    .map(|(name, channel)| json!({"name":name,"channel":channel}))
                    .collect::<Vec<_>>(),
                "received_at_ms":v.beacn.received_at_ms,
                "remote":v.beacn.remote.iter()
                    .map(|r| json!({"at_ms":r.at_ms,"key":r.key,"channel":r.channel,"error":r.error}))
                    .collect::<Vec<_>>()
            },
            "link_out":{
                "complete":v.link_out.complete(),
                "sources_ready":v.link_out.sources_ready,
                "transport":format!("{:?}",v.link_out.transport),
                "links":v.link_out.links,
                "profile":v.link_out.profile,
                "profile_owned":v.link_out.profile_owned,
                "error":v.link_out.error
            },
            "message":v.message,
            "feedback":self.feedback,
            "auto_connect":v.auto_connect,
            "applications":apps,
            "meters":meters,
            "components":components,
            "drawers":{"applications":self.catalog_open,"settings":self.settings,"outputs":self.outputs_open,"channel":self.channel},
            "outputs":self.view.outputs.iter().map(|o| json!({"name":o.name,"description":o.description,"default":self.view.default_output.as_deref()==Some(o.name.as_str())})).collect::<Vec<_>>(),
            "search":self.search.read(cx).value().to_string(),
            "appearance":{"light":self.appearance.light,"accent":self.appearance.accent.label()},
            "window_active":window.is_window_active(),
            "size":{"width":f32::from(window.viewport_size().width),
                    "height":f32::from(window.viewport_size().height),
                    "console_width":f32::from(crate::link_ui::console_width(window)),
                    "console_height":f32::from(crate::link_ui::console_height(window))}
        })
    }
}
