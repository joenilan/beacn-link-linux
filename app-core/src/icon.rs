//! One lynx silhouette and one status owner for window, interface and tray.
use crate::model::Snapshot;
use beacn_link_protocol::link_session::Phase;
pub struct IconData {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Status {
    #[default]
    Disconnected,
    Connecting,
    WaitingAudio,
    Connected,
    Error,
    PrivateTest,
}
impl Status {
    pub fn from_snapshot(view: &Snapshot) -> Self {
        if view.discovery_error
            || view.link.phase == Phase::NeedsReview
            || view.link.error.is_some()
        {
            Self::Error
        } else if view.waiting.is_some() {
            Self::WaitingAudio
        } else if view.link.phase == Phase::Running {
            if view.private_session {
                Self::PrivateTest
            } else if view.link.completed_cycles > 0 {
                Self::Connected
            } else {
                Self::Connecting
            }
        } else {
            Self::Disconnected
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Disconnected => "Link disconnected",
            Self::Connecting => "Link connecting",
            Self::WaitingAudio => "Waiting for audio",
            Self::Connected => "Link connected",
            Self::Error => "Link needs attention",
            Self::PrivateTest => "Private test session",
        }
    }
    pub fn color(self) -> [u8; 4] {
        match self {
            Self::Disconnected => [149, 163, 183, 255],
            Self::Connecting => [246, 190, 76, 255],
            Self::WaitingAudio => [246, 190, 76, 255],
            Self::Connected => [81, 224, 194, 255],
            Self::Error => [248, 104, 113, 255],
            Self::PrivateTest => [112, 172, 255, 255],
        }
    }
}
// Symmetric pointed ears, long tufts, cheek ruff and tapered chin; negative eyes/nose.
const FACE: &[(f32, f32)] = &[
    (8., 0.),
    (17., 14.),
    (24., 18.),
    (32., 16.),
    (40., 18.),
    (47., 14.),
    (56., 0.),
    (56., 24.),
    (60., 32.),
    (59., 38.),
    (64., 42.),
    (57., 44.),
    (60., 49.),
    (49., 50.),
    (44., 57.),
    (32., 64.),
    (20., 57.),
    (15., 50.),
    (4., 49.),
    (7., 44.),
    (0., 42.),
    (5., 38.),
    (4., 32.),
    (8., 24.),
];
const LEFT_EYE: &[(f32, f32)] = &[(15., 32.), (27., 35.), (25., 39.), (18., 37.)];
const RIGHT_EYE: &[(f32, f32)] = &[(49., 32.), (37., 35.), (39., 39.), (46., 37.)];
const NOSE: &[(f32, f32)] = &[(28., 46.), (36., 46.), (32., 50.)];
fn inside(x: f32, y: f32, points: &[(f32, f32)]) -> bool {
    let mut result = false;
    let mut previous = points[points.len() - 1];
    for &point in points {
        if (point.1 > y) != (previous.1 > y)
            && x < (previous.0 - point.0) * (y - point.1) / (previous.1 - point.1) + point.0
        {
            result = !result;
        }
        previous = point;
    }
    result
}
pub fn raster(status: Status, size: u32) -> IconData {
    assert!((16..=256).contains(&size));
    let [r, g, b, _] = status.color();
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let mut coverage = 0;
            for sy in 0..4 {
                for sx in 0..4 {
                    let px = (x as f32 + (sx as f32 + 0.5) / 4.) * 64. / size as f32;
                    let py = (y as f32 + (sy as f32 + 0.5) / 4.) * 64. / size as f32;
                    if inside(px, py, FACE)
                        && ![LEFT_EYE, RIGHT_EYE, NOSE]
                            .iter()
                            .any(|p| inside(px, py, p))
                    {
                        coverage += 1;
                    }
                }
            }
            rgba.extend([r, g, b, (coverage * 255 / 16) as u8]);
        }
    }
    IconData {
        rgba,
        width: size,
        height: size,
    }
}

/// Vector asset derived from the same geometry as the status pixmaps.
pub fn svg(status: Status) -> String {
    let [r, g, b, _] = status.color();
    let mut path = String::new();
    for polygon in [FACE, LEFT_EYE, RIGHT_EYE, NOSE] {
        for (n, (x, y)) in polygon.iter().enumerate() {
            path.push_str(&format!("{}{} {} ", if n == 0 { "M" } else { "L" }, x, y));
        }
        path.push_str("Z ");
    }
    format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 64 64\"><path fill=\"#{r:02x}{g:02x}{b:02x}\" fill-rule=\"evenodd\" d=\"{path}\"/></svg>")
}
