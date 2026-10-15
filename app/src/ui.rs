use crate::model::{self, Command, Snapshot};
use beacn_link_protocol::planner::ApplicationId;
use eframe::egui::{self, Color32, RichText};
use std::collections::BTreeMap;

const BG: Color32 = Color32::from_rgb(15, 19, 27);
const PANEL: Color32 = Color32::from_rgb(23, 29, 40);
const CARD: Color32 = Color32::from_rgb(29, 37, 50);
const MUTED: Color32 = Color32::from_rgb(149, 163, 183);
const ACCENT: Color32 = Color32::from_rgb(81, 224, 194);
#[derive(Default)]
pub struct Interface {
    pub search: String,
    pub tray_unavailable: bool,
    icon: Option<(crate::icon::Status, egui::TextureHandle)>,
    drafts: BTreeMap<ApplicationId, String>,
    selected_edits: BTreeMap<ApplicationId, String>,
    core: Option<String>,
    pub feedback: Option<String>,
    /// Widget bounds for isolated egui event tests, never OS mouse automation.
    pub widgets: BTreeMap<String, egui::Rect>,
}
pub fn style(ctx: &egui::Context) {
    ctx.set_visuals(egui::Visuals::dark());
    ctx.style_mut(|s| {
        s.visuals.panel_fill = BG;
        s.visuals.window_fill = PANEL;
        s.visuals.override_text_color = Some(Color32::from_rgb(229, 237, 246));
        s.visuals.selection.bg_fill = Color32::from_rgb(35, 103, 94);
        s.spacing.item_spacing = egui::vec2(10., 10.);
        s.text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(15.));
        s.text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(14.));
    });
}
impl Interface {
    pub fn show(&mut self, ctx: &egui::Context, snapshot: &Snapshot) -> Vec<Command> {
        self.widgets.clear();
        let status = crate::icon::Status::from_snapshot(snapshot);
        if self.icon.as_ref().is_none_or(|(old, _)| *old != status) {
            let icon = crate::icon::raster(status, 64);
            let texture = ctx.load_texture(
                "lynx-status",
                egui::ColorImage::from_rgba_unmultiplied([64, 64], &icon.rgba),
                egui::TextureOptions::LINEAR,
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Icon(Some(std::sync::Arc::new(icon))));
            self.icon = Some((status, texture));
        }
        let running = snapshot.link.phase == beacn_link_protocol::link_session::Phase::Running;
        let core = snapshot.catalog.as_ref().map(|c| c.core.clone());
        if self.core != core {
            self.drafts.clear();
            self.selected_edits.clear();
            self.core = core;
        }
        let mut commands = Vec::new();
        let apps = snapshot.catalog.as_ref().map(|c| &c.applications);
        self.drafts
            .retain(|id, _| apps.is_some_and(|apps| apps.contains_key(id)));
        self.selected_edits
            .retain(|id, _| snapshot.selected.contains_key(id));
        egui::TopBottomPanel::top("brand")
            .frame(egui::Frame::new().fill(PANEL).inner_margin(22))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("LinX").size(30.).strong().color(ACCENT));
                    ui.vertical(|ui| {
                        ui.label(RichText::new("BEACN").size(12.).color(MUTED));
                        ui.label("Your Linux audio, connected.");
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(status.label()).color(crate::icon::color(status)));
                        if let Some((_, texture)) = &self.icon {
                            ui.add(
                                egui::Image::new(texture).fit_to_exact_size(egui::vec2(30., 30.)),
                            );
                        }
                    });
                });
            });
        egui::TopBottomPanel::bottom("footer")
            .frame(egui::Frame::new().fill(PANEL).inner_margin(14))
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("Independent Linux companion · Preview")
                            .small()
                            .color(MUTED),
                    );
                    if self.tray_unavailable {
                        ui.label(
                            RichText::new("System tray unavailable on this desktop")
                                .small()
                                .color(MUTED),
                        );
                    }
                    ui.separator();
                    ui.label(
                        RichText::new("Selections last until this window closes")
                            .small()
                            .color(MUTED),
                    );
                });
            });
        egui::SidePanel::left("link-status")
            .exact_width(258.)
            .resizable(false)
            .frame(egui::Frame::new().fill(PANEL).inner_margin(22))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 6.;
                    ui.add_space(4.);
                    ui.label(RichText::new("STUDIO LINK").size(12.).color(MUTED));
                    ui.add_space(10.);
                    ui.label(RichText::new(if running { "Private test running" } else { "Routing is off" }).size(23.).strong());
                    ui.add_space(4.);
                    if !running { ui.label(RichText::new("Select the apps you want to share.").color(MUTED)); }
                    ui.add_space(10.);
                    let review = snapshot.link.phase == beacn_link_protocol::link_session::Phase::NeedsReview;
                    let start = ui.add_enabled(snapshot.private_session && snapshot.connected && !snapshot.selected.is_empty() && !running && !review,
                        egui::Button::new("Start link").min_size(egui::vec2(ui.available_width(), 38.)));
                    self.widgets.insert("start".into(), start.rect);
                    if start.clicked() { commands.push(Command::Start); }
                    start.on_hover_text("Enabled only for an explicitly configured private test core. Desktop routing and USB remain off.");
                    let stop = ui.add_enabled(running, egui::Button::new("Stop link").min_size(egui::vec2(ui.available_width(), 32.)));
                    self.widgets.insert("stop".into(), stop.rect);
                    if stop.clicked() { commands.push(Command::Stop); }
                    ui.add_space(6.);
                    ui.label(RichText::new(if snapshot.private_session { "Private core · scripted peer · no device I/O" } else { "Device connection is coming next." }).small().color(MUTED));
                    ui.label(RichText::new(if snapshot.connected { "Application discovery online" } else { "Application discovery offline" }).small().color(MUTED));
                    ui.add_space(10.);
                    ui.separator();
                    ui.add_space(10.);
                    ui.label(RichText::new("LINK CHANNELS").size(12.).color(MUTED));
                    for i in 1..=4 {
                        ui.add_space(6.);
                        egui::Frame::new()
                            .fill(CARD)
                            .corner_radius(7)
                            .inner_margin(10)
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(format!("{i:02}")).color(ACCENT));
                                    ui.label(format!("Link {i}"));
                                });
                                let names = snapshot.routing.assignments.iter().filter(|(_, pair)| **pair == i).filter_map(|(id, _)| snapshot.selected.get(id)).cloned().collect::<Vec<_>>();
                                ui.label(RichText::new(if names.is_empty() { if running { "Unassigned".into() } else { "No active session".into() } } else { names.join(", ") }).small().color(MUTED));
                            });
                    }
                });
            });
        egui::CentralPanel::default().frame(egui::Frame::new().fill(BG).inner_margin(28)).show(ctx,|ui|{
            ui.horizontal(|ui|{
                ui.vertical(|ui|{ui.heading(RichText::new("Applications").size(28.));ui.label(RichText::new("Choose what appears on your streaming PC.").color(MUTED));});
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{ui.label(RichText::new(format!("{} selected",snapshot.selected.len())).color(ACCENT));});
            });
            ui.add_space(12.);
            ui.horizontal(|ui|{
                let search=ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Search applications").desired_width((ui.available_width()-100.).max(120.)));
                self.widgets.insert("search".into(),search.rect);
                let refresh=ui.add_enabled(snapshot.connected,egui::Button::new("Refresh"));self.widgets.insert("refresh".into(),refresh.rect);
                if refresh.clicked(){commands.push(Command::Refresh);}
            });
            if let Some(message)=self.feedback.as_ref().or(snapshot.message.as_ref()) {
                ui.add_space(4.);ui.label(RichText::new(model::label(message)).color(MUTED));
            }
            ui.add_space(12.);
            let filter=self.search.to_lowercase();let mut rows=apps.map(|a|a.values().collect::<Vec<_>>()).unwrap_or_default();
            rows.sort_by_key(|app|(!snapshot.selected.contains_key(&app.id),model::app_name(app).to_lowercase(),app.id.clone()));
            let count=rows.len();rows.retain(|app|model::app_name(app).to_lowercase().contains(&filter)||app.binaries.iter().any(|b|b.to_lowercase().contains(&filter)));
            if snapshot.busy {ui.label("Looking for applications playing audio…");}
            else if rows.is_empty() {
                egui::Frame::new().fill(CARD).corner_radius(10).inner_margin(24).show(ui,|ui|{
                    ui.set_min_width((ui.available_width()-48.).max(100.));
                    ui.heading(if !snapshot.connected{"Application discovery is unavailable"}else if count==0{"No audio applications yet"}else{"No matching applications"});
                    ui.label(RichText::new(if !snapshot.connected{"Reopen LinX after your Linux audio session is available."}else if count==0{"Play audio in a game, browser or media player. It will appear here automatically."}else{"Try another application name or clear your search."}).color(MUTED));
                });
            }
            egui::ScrollArea::vertical().auto_shrink([false,false]).show(ui,|ui|{
                for app in rows {
                    let selected=snapshot.selected.get(&app.id);let name=model::app_name(app);let enabled=snapshot.connected&&model::live(app);
                    egui::Frame::new().fill(CARD).corner_radius(10).inner_margin(16).stroke(egui::Stroke::new(1.,if selected.is_some(){Color32::from_rgb(44,110,98)}else{CARD})).show(ui,|ui|{
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui|{
                            ui.vertical(|ui|{ui.label(RichText::new(&name).size(18.).strong()).on_hover_text(format!("Process {} · {} audio stream(s)",app.pid,app.streams.len()));ui.label(RichText::new(if selected.is_some(){if running { "Selected · session active" } else { "Selected · ready for a link session" }}else if !model::live(app){"Audio stream unavailable"}else{"Available on this computer"}).small().color(if selected.is_some(){ACCENT}else{MUTED}));});
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                                if selected.is_some() {
                                    let remove=ui.add_enabled(snapshot.connected,egui::Button::new("Remove"));self.widgets.insert(format!("remove:{}",app.id.0),remove.rect);
                                    if remove.clicked(){commands.push(Command::Unselect(app.id.clone()));}
                                }else {
                                    let add=ui.add_enabled(enabled,egui::Button::new("Select").fill(Color32::from_rgb(35,92,82)));self.widgets.insert(format!("select:{}",app.id.0),add.rect);
                                    if add.clicked(){let alias=self.drafts.entry(app.id.clone()).or_insert_with(||model::default_alias(app)).clone();commands.push(Command::Select(app.id.clone(),alias));}
                                }
                            });
                        });
                        ui.add_space(6.);ui.horizontal(|ui|{
                            ui.label(RichText::new("Published name").small().color(MUTED));
                            let draft=if let Some(alias)=selected {self.selected_edits.entry(app.id.clone()).or_insert_with(||alias.clone())}else{self.drafts.entry(app.id.clone()).or_insert_with(||model::default_alias(app))};
                            let edit=ui.add_enabled(snapshot.connected,egui::TextEdit::singleline(draft).char_limit(255).desired_width(230.));self.widgets.insert(format!("alias:{}",app.id.0),edit.rect);
                            if let Some(alias)=selected {
                                let save=ui.add_enabled(enabled&&draft!=alias,egui::Button::new("Save name"));self.widgets.insert(format!("save:{}",app.id.0),save.rect);
                                if save.clicked(){commands.push(Command::Select(app.id.clone(),draft.clone()));}
                            }
                        });
                    });ui.add_space(10.);
                }
            });
        });
        commands
    }
}
