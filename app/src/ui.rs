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
                        ui.label(
                            RichText::new(if snapshot.busy {
                                "Opening audio session"
                            } else if snapshot.connected {
                                "Discovery connected"
                            } else {
                                "Discovery offline"
                            })
                            .color(if snapshot.connected {
                                ACCENT
                            } else {
                                MUTED
                            }),
                        );
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(12., 12.), egui::Sense::hover());
                        ui.painter().circle_filled(
                            rect.center(),
                            3.,
                            if snapshot.connected { ACCENT } else { MUTED },
                        );
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
                    ui.add_space(8.);
                    ui.label(RichText::new("STUDIO LINK").size(12.).color(MUTED));
                    ui.add_space(10.);
                    ui.label(RichText::new("Routing is off").size(23.).strong());
                    ui.add_space(4.);
                    ui.label(
                        RichText::new(
                            "Prepare the applications you want to share with your streaming PC.",
                        )
                        .color(MUTED),
                    );
                    ui.add_space(10.);
                    let start = ui.add_enabled(
                        false,
                        egui::Button::new("Start link")
                            .min_size(egui::vec2(ui.available_width(), 38.)),
                    );
                    self.widgets.insert("start".into(), start.rect);
                    start.on_hover_text(
                        "Device connection and desktop routing are not enabled in this preview.",
                    );
                    ui.add_space(6.);
                    ui.label(
                        RichText::new("Device connection is coming next.")
                            .small()
                            .color(MUTED),
                    );
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
                                ui.label(RichText::new("No active session").small().color(MUTED));
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
                    ui.label(RichText::new(if !snapshot.connected{"Reopen Beacn LinX after your Linux audio session is available."}else if count==0{"Play audio in a game, browser or media player. It will appear here automatically."}else{"Try another application name or clear your search."}).color(MUTED));
                });
            }
            egui::ScrollArea::vertical().auto_shrink([false,false]).show(ui,|ui|{
                for app in rows {
                    let selected=snapshot.selected.get(&app.id);let name=model::app_name(app);let enabled=snapshot.connected&&model::live(app);
                    egui::Frame::new().fill(CARD).corner_radius(10).inner_margin(16).stroke(egui::Stroke::new(1.,if selected.is_some(){Color32::from_rgb(44,110,98)}else{CARD})).show(ui,|ui|{
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui|{
                            ui.vertical(|ui|{ui.label(RichText::new(&name).size(18.).strong()).on_hover_text(format!("Process {} · {} audio stream(s)",app.pid,app.streams.len()));ui.label(RichText::new(if selected.is_some(){"Selected · ready for a future link session"}else if !model::live(app){"Audio stream unavailable"}else{"Available on this computer"}).small().color(if selected.is_some(){ACCENT}else{MUTED}));});
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
