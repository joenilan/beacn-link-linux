//! Sound map: every application that plays sound as a puck orbiting where it
//! plays (a Link In of the BEACN Studio, a device on this PC, or the default
//! output), moved by drag and drop. The map opens over the whole screen on
//! the overlay layer, with the desktop dimmed behind it, so a shortcut can
//! bring it up for a quick move. "Minimal" shows the same routing as lanes.
use crate::link_ui::{key_id, LinkCore};
use beacn_linx_core::link::{LinkApp, LinkCommand, LinkView, Phase, Target};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

/// The map is laid out on a 1280x800 plane, scaled to fit the screen.
const PLANE: (f32, f32) = (1280., 800.);
const HUB: P = P(450., 440.);
const PORTS: [P; 4] = [P(290., 280.), P(610., 280.), P(290., 610.), P(610., 610.)];
const PORT_R: f32 = 42.;
const ORB: P = P(800., 725.);
/// "Not linked" area (orbit view).
const LOOSE: (f32, f32, f32, f32) = (30., 130., 140., 640.);
/// How long a move shows as made before PipeWire confirms it.
const SETTLE: Duration = Duration::from_millis(2500);
/// Fade in and out, like Plasma's own overlays.
const FADE: Duration = Duration::from_millis(650);
/// How dark the tint over the blurred desktop gets.
const DIM: f32 = 0.32;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
struct P(f32, f32);
impl P {
    fn dist(self, o: P) -> f32 {
        ((self.0 - o.0).powi(2) + (self.1 - o.1).powi(2)).sqrt()
    }
    fn lerp(self, o: P, t: f32) -> P {
        P(self.0 + (o.0 - self.0) * t, self.1 + (o.1 - self.1) * t)
    }
}

/// Where an application plays, as the map shows it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Spot {
    Port(u8),
    Device(String),
    Loose,
    /// The BEACN Studio as a whole: back to each application's own channel.
    Studio,
}

#[derive(Clone)]
enum Shape {
    Circle(P, f32),
    Rect(P, P),
}

/// A place to drop applications.
#[derive(Clone)]
struct Zone {
    spot: Spot,
    shape: Shape,
    label: String,
    color: u32,
    icon: &'static str,
}
impl Zone {
    fn center(&self) -> P {
        match self.shape {
            Shape::Circle(c, _) => c,
            Shape::Rect(a, b) => P((a.0 + b.0) / 2., (a.1 + b.1) / 2.),
        }
    }
    fn hit(&self, p: P) -> bool {
        match self.shape {
            Shape::Circle(c, r) => p.dist(c) < r + 34.,
            Shape::Rect(a, b) => p.0 >= a.0 && p.0 <= b.0 && p.1 >= a.1 && p.1 <= b.1,
        }
    }
}

enum Grab {
    Apps(Vec<String>),
    Everything,
}
struct Drag {
    grab: Grab,
    start: P,
    at: P,
    moved: bool,
}

pub struct SoundMap {
    view: LinkView,
    viewed: Instant,
    minimal: bool,
    /// Displayed puck centers, easing toward their slots.
    shown: BTreeMap<String, P>,
    selected: BTreeSet<String>,
    drag: Option<Drag>,
    lasso: Option<(P, P)>,
    /// A press on empty space that may become a click (closes the map).
    pressed_empty: bool,
    /// Moves sent but not yet seen in the graph.
    pending: BTreeMap<String, (Spot, Instant)>,
    toast: Option<(String, Instant)>,
    levels: [f32; 4],
    started: Instant,
    /// Fading out; the window closes when done.
    closing: Option<Instant>,
    /// The next frame while nothing moves.
    tick: Option<Task<()>>,
    /// Scale and origin of the plane in the window, from the last frame.
    frame: (f32, f32, f32),
    focus: FocusHandle,
}

fn alpha(color: u32, a: f32) -> Hsla {
    let mut c: Hsla = rgb(color).into();
    c.a = a;
    c
}

const TEXT: u32 = 0xe5e7e8;
const MUTED: u32 = 0xa0a6ad;
const LOOSE_COLOR: u32 = 0x9aa1a8;
/// Colors for devices on this PC, in list order.
const DEVICE_COLORS: [u32; 5] = [0xef9f76, 0x8fd3e0, 0xe5a5c4, 0xc8d98b, 0xb4b9f0];

fn device_icon(name: &str, description: &str) -> &'static str {
    let text = format!("{name} {description}").to_lowercase();
    if ["hdmi", "displayport", "monitor"].iter().any(|w| text.contains(w)) {
        "console/monitor.svg"
    } else if ["headphone", "headset", "earbud", "airpods"].iter().any(|w| text.contains(w)) {
        "console/headphones.svg"
    } else {
        "console/speaker.svg"
    }
}

impl SoundMap {
    pub fn new(minimal: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self {
            view: cx.global::<LinkCore>().worker.view(),
            viewed: Instant::now(),
            minimal,
            shown: BTreeMap::new(),
            selected: BTreeSet::new(),
            drag: None,
            lasso: None,
            pressed_empty: false,
            pending: BTreeMap::new(),
            toast: None,
            levels: [0.; 4],
            started: Instant::now(),
            closing: None,
            tick: None,
            frame: (1., 0., 0.),
            focus,
        }
    }

    fn channels(&self) -> [u32; 4] {
        crate::appearance::Appearance::default().colors().channels
    }

    /// Window position to plane position.
    fn plane(&self, at: Point<Pixels>) -> P {
        let (s, ox, oy) = self.frame;
        P((f32::from(at.x) - ox) / s, (f32::from(at.y) - oy) / s)
    }

    fn spot_of(&self, app: &LinkApp) -> Spot {
        if let Some((spot, at)) = self.pending.get(&app.key) {
            if at.elapsed() < SETTLE {
                return spot.clone();
            }
        }
        let present = |name: &String| self.view.outputs.iter().any(|o| &o.name == name);
        match &app.pinned {
            // An unplugged device: shown where it plays meanwhile.
            Some(Target::Device(name)) if present(name) => Spot::Device(name.clone()),
            Some(Target::Device(_)) if self.view.local_output => self
                .view
                .default_output
                .clone()
                .map_or(Spot::Loose, Spot::Device),
            Some(Target::Device(_)) => app.remembered.map_or(Spot::Loose, Spot::Port),
            Some(Target::Channel(c)) => Spot::Port(*c),
            None if self.view.local_output => self
                .view
                .default_output
                .clone()
                .map_or(Spot::Loose, Spot::Device),
            None if app.channel > 0 => Spot::Port(app.channel),
            None => app.remembered.map_or(Spot::Loose, Spot::Port),
        }
    }

    /// Devices on this PC shown on the map: not hidden, or in use.
    fn devices(&self, cx: &App) -> Vec<(String, String)> {
        let hidden = &cx.global::<LinkCore>().hidden_outputs;
        let used: BTreeSet<String> = self
            .view
            .apps
            .iter()
            .filter_map(|a| match self.spot_of(a) {
                Spot::Device(d) => Some(d),
                _ => None,
            })
            .collect();
        self.view
            .outputs
            .iter()
            .filter(|o| o.description != beacn_linx_core::link::LINX_OUTPUT)
            .filter(|o| !hidden.contains(&o.name) || used.contains(&o.name))
            .map(|o| (o.name.clone(), o.description.clone()))
            .collect()
    }

    fn zones(&self, cx: &App) -> Vec<Zone> {
        let colors = self.channels();
        let devices = self.devices(cx);
        let mut zones = Vec::new();
        if self.minimal {
            for c in 0..4u8 {
                let x = 32. + c as f32 * 184.;
                zones.push(Zone {
                    spot: Spot::Port(c + 1),
                    shape: Shape::Rect(P(x, 156.), P(x + 172., 626.)),
                    label: format!("Link In {}", c + 1),
                    color: colors[c as usize],
                    icon: "",
                });
            }
            let n = devices.len().max(1) as f32;
            let w = ((448. - 16. * (n - 1.)) / n).max(90.);
            for (i, (name, description)) in devices.iter().enumerate() {
                let x = 800. + i as f32 * (w + 16.);
                zones.push(Zone {
                    spot: Spot::Device(name.clone()),
                    shape: Shape::Rect(P(x, 156.), P(x + w, 626.)),
                    label: description.clone(),
                    color: DEVICE_COLORS[i % DEVICE_COLORS.len()],
                    icon: device_icon(name, description),
                });
            }
            zones.push(Zone {
                spot: Spot::Studio,
                shape: Shape::Rect(P(32., 118.), P(756., 150.)),
                label: "BEACN Studio (LinX)".into(),
                color: colors[0],
                icon: "",
            });
            zones.push(Zone {
                spot: Spot::Loose,
                shape: Shape::Rect(P(32., 648.), P(1248., 768.)),
                label: "Not linked · follows the default output".into(),
                color: LOOSE_COLOR,
                icon: "",
            });
        } else {
            for c in 0..4u8 {
                zones.push(Zone {
                    spot: Spot::Port(c + 1),
                    shape: Shape::Circle(PORTS[c as usize], PORT_R),
                    label: format!("Link In {}", c + 1),
                    color: colors[c as usize],
                    icon: "",
                });
            }
            let n = devices.len();
            let cols = if n > 3 { 2 } else { 1 };
            let rows = n.div_ceil(cols).max(1);
            let r = if n > 3 { 44. } else { 54. };
            let gap = (480. / rows as f32).min(230.);
            for (i, (name, description)) in devices.iter().enumerate() {
                let (col, row) = (i % cols, i / cols);
                let x = if cols == 1 { 1075. } else { [985., 1160.][col] };
                let y = 450. + (row as f32 - (rows as f32 - 1.) / 2.) * gap;
                zones.push(Zone {
                    spot: Spot::Device(name.clone()),
                    shape: Shape::Circle(P(x, y), r),
                    label: description.clone(),
                    color: DEVICE_COLORS[i % DEVICE_COLORS.len()],
                    icon: device_icon(name, description),
                });
            }
            zones.push(Zone {
                spot: Spot::Studio,
                shape: Shape::Rect(P(HUB.0 - 100., HUB.1 - 55.), P(HUB.0 + 100., HUB.1 + 55.)),
                label: "BEACN Studio".into(),
                color: colors[0],
                icon: "",
            });
            let (x, y, w, h) = LOOSE;
            zones.push(Zone {
                spot: Spot::Loose,
                shape: Shape::Rect(P(x, y), P(x + w, y + h)),
                label: "Not linked".into(),
                color: LOOSE_COLOR,
                icon: "",
            });
        }
        zones
    }

    /// Slot centers for every application, by zone.
    fn slots(&self, zones: &[Zone]) -> BTreeMap<String, P> {
        let mut groups: BTreeMap<Spot, Vec<&LinkApp>> = BTreeMap::new();
        for app in &self.view.apps {
            let mut spot = self.spot_of(app);
            if !zones.iter().any(|z| z.spot == spot) {
                spot = Spot::Loose;
            }
            groups.entry(spot).or_default().push(app);
        }
        let mut out = BTreeMap::new();
        for zone in zones {
            let Some(apps) = groups.get(&zone.spot) else { continue };
            for (i, app) in apps.iter().enumerate() {
                let p = match (&zone.shape, self.minimal) {
                    (Shape::Rect(a, b), true) if zone.spot == Spot::Loose => {
                        let per = (((b.0 - a.0) - 32.) / 162.).max(1.) as usize;
                        P(a.0 + 91. + (i % per) as f32 * 162., a.1 + 62. + (i / per) as f32 * 44.)
                    }
                    (Shape::Rect(a, b), true) => {
                        P((a.0 + b.0) / 2., a.1 + 66. + i as f32 * 50.)
                    }
                    (Shape::Rect(a, b), false) => {
                        let rows = ((b.1 - a.1 - 90.) / 86.).max(1.) as usize;
                        let cols = apps.len().div_ceil(rows).max(1) as f32;
                        let col = (i / rows) as f32;
                        let x = (a.0 + b.0) / 2. + (col - (cols - 1.) / 2.) * 62.;
                        P(x, a.1 + 50. + (i % rows) as f32 * 86.)
                    }
                    (Shape::Circle(c, r), _) => {
                        let base = match zone.spot {
                            Spot::Port(_) => (c.1 - HUB.1).atan2(c.0 - HUB.0),
                            _ => std::f32::consts::PI,
                        };
                        let ring = i / 5;
                        let in_ring = (apps.len() - ring * 5).min(5) as f32;
                        let j = (i % 5) as f32;
                        let a = base + (j - (in_ring - 1.) / 2.) * 0.62;
                        let radius = r + 58. + ring as f32 * 58.;
                        P(c.0 + a.cos() * radius, c.1 + a.sin() * radius)
                    }
                };
                out.insert(app.key.clone(), p);
            }
        }
        out
    }

    fn zone_name(zones: &[Zone], spot: &Spot) -> String {
        zones
            .iter()
            .find(|z| &z.spot == spot)
            .map_or_else(|| "Not linked".into(), |z| z.label.clone())
    }

    fn send(&mut self, command: LinkCommand, cx: &mut Context<Self>) -> bool {
        match cx.global::<LinkCore>().worker.send(command) {
            Ok(()) => true,
            Err(error) => {
                self.toast = Some((error, Instant::now()));
                false
            }
        }
    }

    fn drop_on(&mut self, spot: Spot, zones: &[Zone], cx: &mut Context<Self>) {
        let Some(drag) = self.drag.take() else { return };
        if spot == Spot::Studio {
            return self.back_to_studio(drag.grab, cx);
        }
        let target = match &spot {
            Spot::Port(c) => Some(Target::Channel(*c)),
            Spot::Device(name) => Some(Target::Device(name.clone())),
            Spot::Loose | Spot::Studio => None,
        };
        let name = Self::zone_name(zones, &spot);
        match drag.grab {
            Grab::Everything => {
                if self.send(LinkCommand::PlaceAll(target), cx) {
                    for app in &self.view.apps {
                        self.pending.insert(app.key.clone(), (spot.clone(), Instant::now()));
                    }
                    self.toast = Some((format!("Everything → {name}"), Instant::now()));
                }
            }
            Grab::Apps(keys) => {
                let moving: Vec<String> = keys
                    .into_iter()
                    .filter(|k| {
                        self.view
                            .apps
                            .iter()
                            .find(|a| &a.key == k)
                            .is_some_and(|a| self.spot_of(a) != spot)
                    })
                    .collect();
                let mut moved = Vec::new();
                for key in moving {
                    if self.send(LinkCommand::Place(key.clone(), target.clone()), cx) {
                        self.pending.insert(key.clone(), (spot.clone(), Instant::now()));
                        moved.push(key);
                    }
                }
                let who = match moved.as_slice() {
                    [] => return,
                    [one] => self
                        .view
                        .apps
                        .iter()
                        .find(|a| &a.key == one)
                        .map_or_else(|| one.clone(), |a| a.name.clone()),
                    many => format!("{} apps", many.len()),
                };
                self.toast = Some((format!("{who} → {name}"), Instant::now()));
            }
        }
    }

    /// Dropped on the Studio: everything (like choosing BEACN Studio as the
    /// sound output) or the dragged applications return to their own Link
    /// channels; one that never had a channel goes to Link In 1.
    fn back_to_studio(&mut self, grab: Grab, cx: &mut Context<Self>) {
        let home = |app: &LinkApp| app.remembered.unwrap_or(1);
        match grab {
            Grab::Everything => {
                let Some(linx) = self
                    .view
                    .outputs
                    .iter()
                    .find(|o| o.description == beacn_linx_core::link::LINX_OUTPUT)
                    .map(|o| o.name.clone())
                else {
                    self.toast = Some(("BEACN Studio is not available.".into(), Instant::now()));
                    return;
                };
                if self.send(LinkCommand::DefaultOutput(linx), cx) {
                    for app in &self.view.apps {
                        let spot = app.remembered.map_or(Spot::Loose, Spot::Port);
                        self.pending.insert(app.key.clone(), (spot, Instant::now()));
                    }
                    self.toast = Some(("Everything → BEACN Studio, each on its own channel".into(), Instant::now()));
                }
            }
            Grab::Apps(keys) => {
                let apps: Vec<LinkApp> = self
                    .view
                    .apps
                    .iter()
                    .filter(|a| keys.contains(&a.key) && self.spot_of(a) != Spot::Port(home(a)))
                    .cloned()
                    .collect();
                let mut moved = Vec::new();
                for app in &apps {
                    let channel = home(app);
                    if self.send(LinkCommand::Place(app.key.clone(), Some(Target::Channel(channel))), cx) {
                        self.pending.insert(app.key.clone(), (Spot::Port(channel), Instant::now()));
                        moved.push((app.name.clone(), channel));
                    }
                }
                self.toast = match moved.as_slice() {
                    [] => return,
                    [(name, channel)] => Some((format!("{name} → Link In {channel}"), Instant::now())),
                    many => Some((format!("{} apps → back to their channels", many.len()), Instant::now())),
                };
            }
        }
    }

    fn close(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.closing.is_none() {
            self.closing = Some(Instant::now());
            self.drag = None;
            self.lasso = None;
            cx.notify();
        }
    }

    /// 0 (hidden) to 1 (shown), eased.
    fn shown_amount(&self) -> f32 {
        let fade = FADE.as_secs_f32();
        let t = match self.closing {
            Some(at) => 1. - (at.elapsed().as_secs_f32() / fade).min(1.),
            None => (self.started.elapsed().as_secs_f32() / fade).min(1.),
        };
        1. - (1. - t).powi(3)
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if self.closing.is_some() {
            return;
        }
        let p = self.plane(event.position);
        self.selected.clear();
        self.lasso = Some((p, p));
        self.pressed_empty = true;
        cx.notify();
    }

    fn grab_app(&mut self, key: String, event: &MouseDownEvent, cx: &mut Context<Self>) {
        cx.stop_propagation();
        let p = self.plane(event.position);
        if event.modifiers.control || event.modifiers.shift {
            if !self.selected.remove(&key) {
                self.selected.insert(key.clone());
            }
        } else if !self.selected.contains(&key) {
            self.selected = [key.clone()].into();
        }
        let keys = if self.selected.contains(&key) {
            self.selected.iter().cloned().collect()
        } else {
            vec![key]
        };
        self.drag = Some(Drag { grab: Grab::Apps(keys), start: p, at: p, moved: false });
        cx.notify();
    }

    fn grab_everything(&mut self, event: &MouseDownEvent, cx: &mut Context<Self>) {
        cx.stop_propagation();
        let p = self.plane(event.position);
        self.selected.clear();
        self.drag = Some(Drag { grab: Grab::Everything, start: p, at: p, moved: false });
        cx.notify();
    }

    fn mouse_move(&mut self, event: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let p = self.plane(event.position);
        if let Some(drag) = &mut self.drag {
            drag.at = p;
            drag.moved |= p.dist(drag.start) > 4.;
            cx.notify();
        } else if let Some((start, _)) = self.lasso {
            self.lasso = Some((start, p));
            if p.dist(start) > 4. {
                self.pressed_empty = false;
            }
            cx.notify();
        }
    }

    fn mouse_up(&mut self, event: &MouseUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.closing.is_some() {
            return;
        }
        let p = self.plane(event.position);
        let zones = self.zones(cx);
        if let Some(drag) = &self.drag {
            if drag.moved {
                if let Some(zone) = zones.iter().find(|z| z.hit(p)) {
                    let spot = zone.spot.clone();
                    self.drop_on(spot, &zones, cx);
                }
            }
            self.drag = None;
        } else if let Some((a, b)) = self.lasso.take() {
            if self.pressed_empty && !zones.iter().any(|z| z.hit(p)) {
                // A click on empty space puts the map away.
                self.close(window, cx);
                return;
            }
            let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
            let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
            self.selected = self
                .shown
                .iter()
                .filter(|(_, p)| p.0 >= x0 && p.0 <= x1 && p.1 >= y0 && p.1 <= y1)
                .map(|(k, _)| k.clone())
                .collect();
        }
        self.pressed_empty = false;
        cx.notify();
    }

    fn hover(&self, zones: &[Zone]) -> Option<Spot> {
        let drag = self.drag.as_ref().filter(|d| d.moved)?;
        zones.iter().find(|z| z.hit(drag.at)).map(|z| z.spot.clone())
    }

    fn refresh(&mut self, cx: &App) {
        if self.viewed.elapsed() >= Duration::from_millis(100) {
            self.view = cx.global::<LinkCore>().worker.view();
            self.viewed = Instant::now();
            let apps = &self.view.apps;
            self.pending.retain(|key, (_, at)| at.elapsed() < SETTLE && apps.iter().any(|a| &a.key == key));
            self.selected.retain(|key| apps.iter().any(|a| &a.key == key));
        }
        for (i, level) in self.levels.iter_mut().enumerate() {
            let used = self.view.apps.iter().any(|a| a.channel as usize == i + 1);
            let peak = self.view.meters[i]
                .as_ref()
                .filter(|_| used && !self.view.local_output)
                .and_then(|m| m.read())
                .map_or(0., |r| r.peak[0].max(r.peak[1]).clamp(0., 1.));
            *level = if peak > *level { peak } else { *level * 0.9 + peak * 0.1 };
        }
    }
}

/// Cubic bezier point for a cable that leaves `a` horizontally.
fn bezier(a: P, b: P, t: f32) -> P {
    let m = (a.0 + b.0) / 2.;
    let (c1, c2) = (P(m, a.1), P(m, b.1));
    let u = 1. - t;
    let w = [u * u * u, 3. * u * u * t, 3. * u * t * t, t * t * t];
    P(
        w[0] * a.0 + w[1] * c1.0 + w[2] * c2.0 + w[3] * b.0,
        w[0] * a.1 + w[1] * c1.1 + w[2] * c2.1 + w[3] * b.1,
    )
}

/// A cable to paint: from, to, color, opacity, flow speed (0 = still).
type Cable = (P, P, u32, f32, f32);

fn paint_cables(cables: Vec<Cable>, frame: (f32, f32, f32), phase: f32, window: &mut Window) {
    let (s, ox, oy) = frame;
    let at = |p: P| point(px(ox + p.0 * s), px(oy + p.1 * s));
    for (a, b, color, opacity, flow) in cables {
        let m = (a.0 + b.0) / 2.;
        let mut path = PathBuilder::stroke(px(2. * s));
        path.move_to(at(a));
        path.cubic_bezier_to(at(b), at(P(m, a.1)), at(P(m, b.1)));
        if let Ok(path) = path.build() {
            window.paint_path(path, alpha(color, opacity * 0.35));
        }
        // Beads travel along the cable toward where the sound goes.
        let n = ((a.dist(b) / 26.).ceil() as usize).clamp(3, 60);
        for k in 0..n {
            let t = ((k as f32 + phase * flow) / n as f32).fract();
            let p = bezier(a, b, t);
            let r = 2.6 * s;
            window.paint_quad(
                fill(Bounds::centered_at(at(p), size(px(r * 2.), px(r * 2.))), alpha(color, opacity))
                    .corner_radii(px(r)),
            );
        }
    }
}

impl Render for SoundMap {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if std::mem::take(&mut cx.global_mut::<LinkCore>().map_close_requested) {
            self.close(window, cx);
        }
        if self.closing.is_some_and(|at| at.elapsed() >= FADE) {
            window.defer(cx, |window, _| window.remove_window());
        }
        self.refresh(cx);
        let fade = self.shown_amount();
        let viewport = window.viewport_size();
        let (vw, vh) = (f32::from(viewport.width), f32::from(viewport.height));
        let s = (vw / (PLANE.0 + 40.)).min(vh / (PLANE.1 + 40.)).clamp(0.5, 1.4);
        self.frame = (s, (vw - PLANE.0 * s) / 2., (vh - PLANE.1 * s) / 2.);
        let (_, ox, oy) = self.frame;
        let x = |v: f32| px(ox + v * s);
        let y = |v: f32| px(oy + v * s);
        let d = |v: f32| px(v * s);

        let zones = self.zones(cx);
        let slots = self.slots(&zones);
        let hover = self.hover(&zones);
        let colors = self.channels();
        let phase = self.started.elapsed().as_secs_f32();

        // Ease every puck toward its slot; dragged ones follow the pointer.
        let (dragged, everything, delta) = match &self.drag {
            Some(Drag { grab: Grab::Apps(keys), start, at, moved: true }) => {
                (keys.clone(), false, P(at.0 - start.0, at.1 - start.1))
            }
            Some(Drag { grab: Grab::Everything, start, at, moved: true }) => {
                (vec![], true, P(at.0 - start.0, at.1 - start.1))
            }
            _ => (vec![], false, P(0., 0.)),
        };
        let mut shown = BTreeMap::new();
        for (key, slot) in &slots {
            let was = self.shown.get(key).copied().unwrap_or(*slot);
            let p = if dragged.contains(key) {
                // Pulled a little toward a hovered target, like a magnet.
                let mut p = P(slot.0 + delta.0, slot.1 + delta.1);
                if let Some(zone) = hover.as_ref().and_then(|h| zones.iter().find(|z| &z.spot == h)) {
                    p = p.lerp(zone.center(), 0.18);
                }
                p
            } else {
                was.lerp(*slot, 0.2)
            };
            shown.insert(key.clone(), p);
        }
        self.shown = shown;
        // Every frame while something moves; otherwise 30 per second for
        // the flowing cables and levels.
        let settling = self.shown.iter().any(|(k, p)| slots.get(k).is_some_and(|s| p.dist(*s) > 0.5));
        if self.drag.is_some() || self.lasso.is_some() || self.closing.is_some() || fade < 1. || settling {
            window.request_animation_frame();
        } else if self.tick.is_none() {
            self.tick = Some(cx.spawn(async move |this, cx| {
                cx.background_executor().timer(Duration::from_millis(33)).await;
                let _ = this.update(cx, |this, cx| {
                    this.tick = None;
                    cx.notify();
                });
            }));
        }
        let orb = if everything {
            let p = P(ORB.0 + delta.0, ORB.1 + delta.1);
            hover
                .as_ref()
                .and_then(|h| zones.iter().find(|z| &z.spot == h))
                .map_or(p, |z| p.lerp(z.center(), 0.18))
        } else {
            ORB
        };

        let spot_of: BTreeMap<String, Spot> =
            self.view.apps.iter().map(|a| (a.key.clone(), self.spot_of(a))).collect();
        let on_studio = spot_of.values().filter(|s| matches!(s, Spot::Port(_))).count();
        let dark = on_studio == 0;
        let count = |spot: &Spot| spot_of.values().filter(|s| *s == spot).count();
        let zone_color = |spot: &Spot| zones.iter().find(|z| &z.spot == spot).map_or(LOOSE_COLOR, |z| z.color);

        // Cables (orbit view only).
        let mut cables: Vec<Cable> = Vec::new();
        if !self.minimal {
            for (c, port) in PORTS.iter().enumerate() {
                let live = count(&Spot::Port(c as u8 + 1)) > 0 && !self.view.local_output;
                let level = self.levels[c];
                cables.push((*port, HUB, colors[c], if live { 0.35 + level * 0.65 } else { 0.08 }, if live { 0.6 + level * 1.6 } else { 0. }));
            }
            for app in &self.view.apps {
                let Some(p) = self.shown.get(&app.key).copied() else { continue };
                if everything {
                    cables.push((p, orb, TEXT, 0.7, 2.));
                    continue;
                }
                let spot = if dragged.contains(&app.key) {
                    match &hover {
                        Some(h) => h.clone(),
                        None => continue,
                    }
                } else {
                    spot_of[&app.key].clone()
                };
                if spot == Spot::Loose {
                    continue;
                }
                let Some(zone) = zones.iter().find(|z| z.spot == spot) else { continue };
                let level = match spot {
                    Spot::Port(c) => self.levels[c as usize - 1],
                    _ => 0.3,
                };
                let flow = if app.running { 0.8 + level * 1.5 } else { 0. };
                cables.push((p, zone.center(), zone.color, if app.running { 0.85 } else { 0.4 }, flow));
            }
        }

        let frame = self.frame;
        let mut root = div()
            .id("sound-map")
            .track_focus(&self.focus)
            .size_full()
            .relative()
            .overflow_hidden()
            .bg(hsla(0., 0., 0.02, DIM * fade))
            .opacity(fade)
            .text_color(rgb(TEXT))
            .font_family("Segoe UI")
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    this.close(window, cx);
                }
            }))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_move(cx.listener(Self::mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
            .child(
                canvas(|_, _, _| (), move |_, _, window, _| paint_cables(cables, frame, phase, window))
                    .absolute()
                    .size_full(),
            );

        // Header.
        let summary = format!(
            "{} on BEACN · {} on this PC · {} not linked",
            on_studio,
            spot_of.values().filter(|s| matches!(s, Spot::Device(_))).count(),
            count(&Spot::Loose)
        );
        root = root.child(
            div()
                .absolute()
                .left(x(32.))
                .top(y(20.))
                .w(d(1216.))
                .flex()
                .items_start()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(d(4.))
                        .child(div().text_size(d(26.)).font_weight(FontWeight::BOLD).child("Sound map"))
                        .child(div().text_size(d(13.)).text_color(rgb(MUTED)).child(
                            "Drag an app onto a channel or a device. Drag across empty space to pick up several. Drop the orb to move everything. Esc closes.",
                        )),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(d(10.))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .child(div().text_size(d(11.)).text_color(rgb(MUTED)).child(summary))
                        .child(
                            div()
                                .id("map-minimal")
                                .px(d(12.))
                                .py(d(7.))
                                .rounded(d(8.))
                                .border_1()
                                .border_color(rgb(0x404247))
                                .bg(alpha(0x2d3035, 0.85))
                                .text_size(d(12.))
                                .cursor_pointer()
                                .hover(|b| b.bg(rgb(0x363a40)))
                                .child(if self.minimal { "Orbit view" } else { "Minimal view" })
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.minimal = !this.minimal;
                                    cx.global_mut::<LinkCore>().map_minimal = this.minimal;
                                    cx.notify();
                                }))
                                .test_support(),
                        )
                        .child(
                            div()
                                .id("map-close")
                                .size(d(32.))
                                .rounded(d(8.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .border_1()
                                .border_color(rgb(0x404247))
                                .bg(alpha(0x2d3035, 0.85))
                                .cursor_pointer()
                                .hover(|b| b.bg(rgb(0x363a40)))
                                .child(svg().path("console/close.svg").size(d(16.)).text_color(rgb(TEXT)))
                                .on_click(cx.listener(|this, _, window, cx| this.close(window, cx)))
                                .test_support(),
                        ),
                ),
        );

        if self.view.phase != Phase::Running {
            root = root.child(
                div()
                    .absolute()
                    .left(x(340.))
                    .top(y(380.))
                    .w(d(600.))
                    .text_center()
                    .text_size(d(16.))
                    .text_color(rgb(MUTED))
                    .child("Connect LinX to place applications."),
            );
            return root;
        }

        if !self.minimal {
            // The Studio hub: also a drop target (back to every channel).
            let hot = hover == Some(Spot::Studio);
            let glow = if hot { 1. } else { self.levels.iter().copied().fold(0., f32::max) };
            root = root.child(
                div()
                    .absolute()
                    .left(x(HUB.0 - 100.))
                    .top(y(HUB.1 - 55.))
                    .w(d(200.))
                    .h(d(110.))
                    .rounded(d(22.))
                    .border(d(if hot { 2. } else { 1. }))
                    .border_color(rgb(if hot {
                        colors[0]
                    } else if dark {
                        0x2d3035
                    } else {
                        0x4a4d53
                    }))
                    .bg(rgb(if dark && !hot { 0x1c1f23 } else { 0x2d3035 }))
                    .when(!dark || hot, |hub| {
                        hub.shadow(vec![BoxShadow {
                            color: alpha(colors[0], 0.15 + glow * 0.35),
                            offset: point(px(0.), px(0.)),
                            blur_radius: d(20. + glow * 40.),
                            spread_radius: d(glow * 6.),
                            inset: false,
                        }])
                    })
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(d(4.))
                    .child(
                        div()
                            .text_size(d(17.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(if dark { 0x747c82 } else { TEXT }))
                            .child("BEACN Studio"),
                    )
                    .child(div().text_size(d(11.)).text_color(rgb(if dark { 0xe58a8a } else { MUTED })).child(
                        if hot {
                            "back to every channel".to_owned()
                        } else if dark {
                            "idle · faders locked".to_owned()
                        } else {
                            format!("LinX · {on_studio} app{}", if on_studio == 1 { "" } else { "s" })
                        },
                    )),
            );
        }

        // Zones.
        for zone in &zones {
            let hot = hover.as_ref() == Some(&zone.spot);
            if zone.spot == Spot::Studio {
                // Orbit view: the hub above. Lanes: a bar over the channels.
                if let (true, Shape::Rect(a, b)) = (self.minimal, &zone.shape) {
                    root = root.child(
                        div()
                            .absolute()
                            .left(x(a.0))
                            .top(y(a.1))
                            .w(d(b.0 - a.0))
                            .h(d(b.1 - a.1))
                            .rounded(d(10.))
                            .border(d(1.5))
                            .border_color(if hot { rgb(zone.color).into() } else { alpha(0x404247, 0.8) })
                            .bg(alpha(if hot { 0x2a2d32 } else { 0x232529 }, 0.85))
                            .flex()
                            .items_center()
                            .gap(d(8.))
                            .px(d(12.))
                            .child(div().size(d(8.)).rounded_full().bg(rgb(if dark { 0xe58a8a } else { zone.color })))
                            .child(div().text_size(d(13.)).font_weight(FontWeight::SEMIBOLD).child(zone.label.clone()))
                            .child(div().text_size(d(11.)).text_color(rgb(MUTED)).child(if hot {
                                "drop: back to their own channels"
                            } else if dark {
                                "idle · faders locked"
                            } else {
                                "drop here to send back to their channels"
                            })),
                    );
                }
                continue;
            }
            let n = count(&zone.spot);
            let sub = match (&zone.spot, n) {
                (Spot::Loose, _) if !self.minimal => "follows default".to_owned(),
                (_, 0) => "empty".to_owned(),
                (_, 1) => "1 app".to_owned(),
                (_, n) => format!("{n} apps"),
            };
            match zone.shape {
                Shape::Circle(c, r) => {
                    let level = match zone.spot {
                        Spot::Port(p) => self.levels[p as usize - 1],
                        _ => 0.,
                    };
                    let is_port = matches!(zone.spot, Spot::Port(_));
                    let size = if hot { r * 2.2 } else { r * 2. };
                    let mut el = div()
                        .absolute()
                        .left(x(c.0 - size / 2.))
                        .top(y(c.1 - size / 2.))
                        .size(d(size))
                        .rounded_full()
                        .border(d(3.))
                        .border_color(rgb(zone.color))
                        .bg(rgb(0x232529))
                        .flex()
                        .items_center()
                        .justify_center()
                        .shadow(vec![BoxShadow {
                            color: alpha(zone.color, if hot { 0.55 } else { 0.1 + level * 0.5 }),
                            offset: point(px(0.), px(0.)),
                            blur_radius: d(if hot { 40. } else { 8. + level * 30. }),
                            spread_radius: d(if hot { 8. } else { level * 6. }),
                            inset: false,
                        }]);
                    if is_port && dark {
                        el = el.opacity(0.55);
                    }
                    el = if is_port {
                        el.child(
                            div()
                                .text_size(d(26.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(zone.color))
                                .child(match zone.spot {
                                    Spot::Port(p) => p.to_string(),
                                    _ => String::new(),
                                }),
                        )
                    } else {
                        el.child(svg().path(zone.icon).size(d(r * 0.75)).text_color(rgb(zone.color)))
                    };
                    root = root.child(el);
                    let label_y = if is_port && c.1 < HUB.1 { c.1 - r - 92. } else { c.1 + r + 10. };
                    let label_y = if is_port && c.1 < HUB.1 { label_y.max(84.) } else { label_y };
                    root = root.child(zone_label(&zone.label, &sub, x(c.0 - 100.), y(label_y), d(200.), s));
                }
                Shape::Rect(a, b) => {
                    let el = div()
                        .absolute()
                        .left(x(a.0))
                        .top(y(a.1))
                        .w(d(b.0 - a.0))
                        .h(d(b.1 - a.1))
                        .rounded(d(if self.minimal { 16. } else { 20. }))
                        .border(d(1.5))
                        .when(!self.minimal, |e| e.border_dashed())
                        .border_color(if hot { rgb(zone.color).into() } else { alpha(0x404247, 1.) })
                        .bg(if self.minimal {
                            alpha(if hot { 0x2a2d32 } else { 0x232529 }, 0.92)
                        } else {
                            alpha(LOOSE_COLOR, if hot { 0.14 } else { 0.03 })
                        });
                    root = root.child(el);
                    if self.minimal {
                        let dim = dark && matches!(zone.spot, Spot::Port(_));
                        root = root.child(
                            div()
                                .absolute()
                                .left(x(a.0 + 14.))
                                .top(y(a.1 + 12.))
                                .w(d(b.0 - a.0 - 28.))
                                .flex()
                                .items_center()
                                .gap(d(8.))
                                .when(dim, |e| e.opacity(0.5))
                                .child(div().size(d(10.)).rounded(d(3.)).bg(rgb(zone.color)))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(d(13.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(zone.label.clone()),
                                )
                                .when(n > 0, |e| e.child(div().text_size(d(11.)).text_color(rgb(MUTED)).child(n.to_string()))),
                        );
                    } else {
                        root = root.child(zone_label(&zone.label, &sub, x(a.0 - 30.), y(b.1 - 46.), d(b.0 - a.0 + 60.), s));
                    }
                }
            }
        }

        // Applications.
        for app in &self.view.apps {
            let Some(p) = self.shown.get(&app.key).copied() else { continue };
            let spot = &spot_of[&app.key];
            let lifted = dragged.contains(&app.key) || everything;
            let color = if everything {
                TEXT
            } else if let (true, Some(h)) = (lifted, &hover) {
                zone_color(h)
            } else {
                zone_color(spot)
            };
            let selected = self.selected.contains(&app.key);
            let key = app.key.clone();
            let initial = app.name.chars().next().unwrap_or('?').to_uppercase().to_string();
            let el = if self.minimal {
                div()
                    .id(SharedString::from(format!("map-app-{}", key_id(&app.key))))
                    .absolute()
                    .left(x(p.0 - 75.))
                    .top(y(p.1 - 20.))
                    .w(d(150.))
                    .h(d(40.))
                    .rounded(d(12.))
                    .border(d(1.5))
                    .border_color(if lifted || selected { rgb(color) } else { rgb(0x404247) })
                    .bg(rgb(0x2d3035))
                    .flex()
                    .items_center()
                    .gap(d(10.))
                    .pl(d(6.))
                    .pr(d(12.))
                    .when(!app.running, |e| e.opacity(0.6))
                    .child(
                        div()
                            .size(d(28.))
                            .flex_shrink_0()
                            .rounded(d(8.))
                            .bg(rgb(color))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(d(13.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0x1c1f23))
                            .child(initial),
                    )
                    .child(div().flex_1().min_w_0().truncate().text_size(d(13.)).child(app.name.clone()))
            } else {
                let size = if lifted { 56. } else { 50. };
                // The label is not part of the puck: crowded orbits put
                // labels over neighbours, and only the circle may grab.
                root = root.child(
                    div()
                        .absolute()
                        .left(x(p.0 - 55.))
                        .top(y(p.1 + size / 2. + 5.))
                        .w(d(110.))
                        .flex()
                        .justify_center()
                        .child(
                            div()
                                .max_w(d(110.))
                                .truncate()
                                .px(d(7.))
                                .py(d(1.))
                                .rounded(d(6.))
                                .bg(alpha(0x1c1f23, 0.85))
                                .text_size(d(12.))
                                .text_color(rgb(0xc9cdd1))
                                .child(app.name.clone()),
                        ),
                );
                div()
                    .id(SharedString::from(format!("map-app-{}", key_id(&app.key))))
                    .absolute()
                    .left(x(p.0 - size / 2.))
                    .top(y(p.1 - size / 2.))
                    .size(d(size))
                    .rounded_full()
                    .border(d(2.5))
                    .border_color(rgb(color))
                    .bg(rgb(0x2d3035))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(d(19.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .when(!app.running, |e| e.opacity(0.6))
                    .shadow(vec![BoxShadow {
                        color: if selected { rgb(0xa6bfe7).into() } else { alpha(0x000000, 0.45) },
                        offset: point(px(0.), d(if selected { 0. } else { 5. })),
                        blur_radius: d(if selected { 0. } else { 14. }),
                        spread_radius: d(if selected { 4. } else { 0. }),
                        inset: false,
                    }])
                    .child(initial)
            };
            root = root.child(
                el.cursor_grab()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                            this.grab_app(key.clone(), event, cx)
                        }),
                    )
                    .test_support(),
            );
        }

        // Everything.
        let (orb_size, label) = if self.minimal { (0., "") } else { (64., "Everything") };
        root = if self.minimal {
            let at = if everything { P(1150. + delta.0, 112. + delta.1) } else { P(1150., 112.) };
            root.child(
                div()
                    .id("map-everything")
                    .absolute()
                    .left(x(at.0 - 70.))
                    .top(y(at.1 - 20.))
                    .w(d(140.))
                    .h(d(40.))
                    .rounded(d(12.))
                    .border(d(1.5))
                    .border_dashed()
                    .border_color(rgb(TEXT))
                    .bg(rgb(0x2d3035))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(d(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_grab()
                    .child("All apps")
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, e: &MouseDownEvent, _, cx| this.grab_everything(e, cx)))
                    .test_support(),
            )
        } else {
            root.child(
                div()
                    .id("map-everything")
                    .absolute()
                    .left(x(orb.0 - orb_size / 2.))
                    .top(y(orb.1 - orb_size / 2.))
                    .size(d(orb_size))
                    .rounded_full()
                    .border(d(2.))
                    .border_dashed()
                    .border_color(rgb(TEXT))
                    .bg(rgb(0x363a40))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_grab()
                    .shadow(vec![BoxShadow {
                        color: alpha(0xc1a9d8, if everything { 0.7 } else { 0.25 + (phase * 2.).sin().abs() * 0.15 }),
                        offset: point(px(0.), px(0.)),
                        blur_radius: d(if everything { 60. } else { 30. }),
                        spread_radius: d(2.),
                        inset: false,
                    }])
                    .child(svg().path("console/orbit.svg").size(d(28.)).text_color(rgb(TEXT)))
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, e: &MouseDownEvent, _, cx| this.grab_everything(e, cx)))
                    .test_support(),
            )
            .child(
                div()
                    .absolute()
                    .left(x(ORB.0 - 80.))
                    .top(y(ORB.1 + 38.))
                    .w(d(160.))
                    .text_center()
                    .text_size(d(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(MUTED))
                    .child(label),
            )
        };

        if let Some((a, b)) = self.lasso.filter(|(a, b)| a.dist(*b) > 4.) {
            root = root.child(
                div()
                    .absolute()
                    .left(x(a.0.min(b.0)))
                    .top(y(a.1.min(b.1)))
                    .w(d((a.0 - b.0).abs()))
                    .h(d((a.1 - b.1).abs()))
                    .rounded(d(8.))
                    .border(d(1.5))
                    .border_color(rgb(0xa6bfe7))
                    .bg(alpha(0xa6bfe7, 0.1)),
            );
        }

        if let Some((text, at)) = &self.toast {
            if at.elapsed() < Duration::from_millis(2600) {
                root = root.child(
                    div()
                        .absolute()
                        .left(x(390.))
                        .top(y(90.))
                        .w(d(500.))
                        .flex()
                        .justify_center()
                        .child(
                            div()
                                .px(d(16.))
                                .py(d(8.))
                                .rounded_full()
                                .bg(rgb(TEXT))
                                .text_color(rgb(0x1c1f23))
                                .text_size(d(13.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(text.clone()),
                        ),
                );
            }
        }
        if let Some(message) = &self.view.message {
            root = root.child(
                div()
                    .absolute()
                    .left(x(32.))
                    .top(y(776.))
                    .text_size(d(12.))
                    .text_color(rgb(0xe58a8a))
                    .child(message.clone()),
            );
        }
        root
    }
}

fn zone_label(label: &str, sub: &str, left: Pixels, top: Pixels, width: Pixels, s: f32) -> Div {
    div()
        .absolute()
        .left(left)
        .top(top)
        .w(width)
        .flex()
        .flex_col()
        .items_center()
        .child(
            div()
                .max_w(width)
                .truncate()
                .text_size(px(13. * s))
                .font_weight(FontWeight::SEMIBOLD)
                .child(label.to_owned()),
        )
        .child(div().text_size(px(11. * s)).text_color(rgb(MUTED)).child(sub.to_owned()))
}

/// Open the map over the screen, or put it away when it is open.
pub(crate) fn toggle(token: Option<String>, cx: &mut App) {
    if let Some(handle) = cx.global::<LinkCore>().map_window {
        if cx.windows().contains(&handle) {
            // Fades out, then closes.
            cx.global_mut::<LinkCore>().map_close_requested = true;
            let _ = handle.update(cx, |_, window, _| window.refresh());
            return;
        }
    }
    cx.global_mut::<LinkCore>().map_close_requested = false;
    if let Some(token) = token {
        gpui_linux::set_pending_activation_token(token);
    }
    let minimal = cx.global::<LinkCore>().map_minimal;
    let bounds = cx
        .primary_display()
        .map(|d| d.bounds())
        .unwrap_or_else(|| Bounds::new(point(px(0.), px(0.)), size(px(1920.), px(1080.))));
    // No window frame: its shadow margin would offset the overlay.
    let build = move |window: &mut Window, cx: &mut App| cx.new(|cx| SoundMap::new(minimal, window, cx));
    let overlay = WindowOptions {
        kind: WindowKind::LayerShell(layer_shell::LayerShellOptions {
            namespace: "linx-sound-map".into(),
            layer: layer_shell::Layer::Overlay,
            anchor: layer_shell::Anchor::TOP
                | layer_shell::Anchor::BOTTOM
                | layer_shell::Anchor::LEFT
                | layer_shell::Anchor::RIGHT,
            keyboard_interactivity: layer_shell::KeyboardInteractivity::Exclusive,
            ..Default::default()
        }),
        // Anchored to every edge with no size: the compositor stretches it
        // over the whole monitor it opens on.
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(point(px(0.), px(0.)), size(px(0.), px(0.))))),
        // The desktop shows through, blurred, as behind Plasma's overlays.
        window_background: WindowBackgroundAppearance::Blurred,
        app_id: Some("io.github.linx".into()),
        focus: true,
        ..Default::default()
    };
    let handle = match cx.open_window(overlay, build) {
        Ok(handle) => Some(handle),
        Err(error) => {
            // No overlay layer (GNOME): a transparent fullscreen window.
            eprintln!("LinX sound map: {error}; using a fullscreen window");
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Fullscreen(bounds)),
                    window_background: WindowBackgroundAppearance::Transparent,
                    window_decorations: Some(WindowDecorations::Client),
                    titlebar: Some(TitlebarOptions {
                        title: Some("LinX Sound map".into()),
                        appears_transparent: true,
                        ..Default::default()
                    }),
                    app_id: Some("io.github.linx".into()),
                    ..Default::default()
                },
                build,
            )
            .map_err(|e| eprintln!("LinX sound map unavailable: {e}"))
            .ok()
        }
    };
    cx.global_mut::<LinkCore>().map_window = handle.map(|h| h.into());
    cx.activate(true);
}

/// Same-user diagnostics: inspect the open map and feed it real pointer and
/// key events, so drags are tested through the map's own input path.
#[cfg(feature = "native-control")]
pub(crate) fn control(op: &str, request: &serde_json::Value, cx: &mut App) -> Result<serde_json::Value, String> {
    use serde_json::json;
    let handle = cx
        .global::<LinkCore>()
        .map_window
        .and_then(|w| w.downcast::<SoundMap>())
        .ok_or("The Sound map is not open.")?;
    let spot_name = |spot: &Spot| match spot {
        Spot::Port(c) => format!("port:{c}"),
        Spot::Device(d) => format!("device:{d}"),
        Spot::Loose => "loose".into(),
        Spot::Studio => "studio".into(),
    };
    if op == "map-input" || op == "map-key" {
        // Delivered as the system would: to the window, while the map itself
        // is free for its own handlers.
        let window_handle: AnyWindowHandle = handle.into();
        return window_handle
            .update(cx, |_, window, cx| match op {
            "map-input" => {
                    let events = request["events"].as_array().ok_or("Missing events")?;
                    for e in events {
                        let position = point(
                            px(e["x"].as_f64().ok_or("Missing x")? as f32),
                            px(e["y"].as_f64().ok_or("Missing y")? as f32),
                        );
                        let modifiers = Modifiers { control: e["ctrl"].as_bool().unwrap_or(false), ..Default::default() };
                        let input = match e["kind"].as_str() {
                            Some("down") => PlatformInput::MouseDown(MouseDownEvent {
                                button: MouseButton::Left,
                                position,
                                modifiers,
                                click_count: 1,
                                first_mouse: false,
                            }),
                            Some("move") => PlatformInput::MouseMove(MouseMoveEvent {
                                position,
                                pressed_button: Some(MouseButton::Left),
                                modifiers,
                            }),
                            Some("up") => PlatformInput::MouseUp(MouseUpEvent {
                                button: MouseButton::Left,
                                position,
                                modifiers,
                                click_count: 1,
                            }),
                            _ => return Err("kind is down, move or up".to_string()),
                        };
                        window.dispatch_event(input, cx);
                        // Lay out again so the next event hits what is now drawn.
                        gpui_kit::test::TestWindowExt::render_frame(window, cx);
                    }
                    Ok(json!({"dispatched": events.len()}))
                }
                "map-key" => {
                    let key = request["key"].as_str().ok_or("Missing key")?;
                    let keystroke = Keystroke::parse(key).map_err(|e| e.to_string())?;
                    window.dispatch_event(
                        PlatformInput::KeyDown(KeyDownEvent { keystroke, is_held: false, prefer_character_input: false }),
                        cx,
                    );
                    Ok(json!({"dispatched": true}))
                }
                _ => unreachable!(),
            })
            .map_err(|e| e.to_string())?;
    }
    handle
        .update(cx, |map, window, cx| match op {
            "map-state" => {
                let (s, ox, oy) = map.frame;
                let at = |p: P| json!({"x": ox + p.0 * s, "y": oy + p.1 * s});
                let zones = map.zones(cx);
                let orb = if map.minimal { P(1150., 112.) } else { ORB };
                Ok(json!({
                    "minimal": map.minimal,
                    "closing": map.closing.is_some(),
                    "viewport": {"width": f32::from(window.viewport_size().width), "height": f32::from(window.viewport_size().height)},
                    "zones": zones.iter().map(|z| json!({"spot": spot_name(&z.spot), "label": z.label, "center": at(z.center())})).collect::<Vec<_>>(),
                    "apps": map.view.apps.iter().filter_map(|a| {
                        let p = *map.shown.get(&a.key)?;
                        Some(json!({"key": a.key, "spot": spot_name(&map.spot_of(a)), "at": at(p)}))
                    }).collect::<Vec<_>>(),
                    "everything": at(orb),
                    "empty": at(P(700., 760.)),
                    "selected": map.selected.iter().collect::<Vec<_>>(),
                    "toast": map.toast.as_ref().map(|(t, _)| t.clone()),
                }))
            }
            "map-minimal" => {
                map.minimal = !map.minimal;
                cx.global_mut::<LinkCore>().map_minimal = map.minimal;
                cx.notify();
                Ok(json!({"minimal": map.minimal}))
            }
            _ => Err("Unknown map op".to_string()),
        })
        .map_err(|e| e.to_string())?
}
