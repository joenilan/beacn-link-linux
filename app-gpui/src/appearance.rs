//! Owner-approved Console palette. Session status colors belong to app-core.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Accent {
    #[default]
    Mint,
    Blue,
    Violet,
    Amber,
    Rose,
}
impl Accent {
    pub const ALL: [Self; 5] = [
        Self::Mint,
        Self::Blue,
        Self::Violet,
        Self::Amber,
        Self::Rose,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Mint => "Mint",
            Self::Blue => "Blue",
            Self::Violet => "Violet",
            Self::Amber => "Amber",
            Self::Rose => "Rose",
        }
    }
    fn channels(self, light: bool) -> [u32; 4] {
        match (self, light) {
            (Self::Mint, false) => [0xa6d5bd, 0xa6bfe7, 0xc1a9d8, 0xd9b68b],
            (Self::Mint, true) => [0x39765f, 0x426ca1, 0x795794, 0x946831],
            (Self::Blue, false) => [0x9dc3ef, 0x8bc1da, 0xabb9e8, 0x95d2cc],
            (Self::Blue, true) => [0x345f9a, 0x32728b, 0x5c63a4, 0x35776d],
            (Self::Violet, false) => [0xbea9e3, 0xb1b8e8, 0xd1afd6, 0xa9bfd5],
            (Self::Violet, true) => [0x7551a5, 0x62669f, 0x985b9d, 0x4e7495],
            (Self::Amber, false) => [0xddb98f, 0xd8ca9c, 0xcca89b, 0xc2be9c],
            (Self::Amber, true) => [0x8e612a, 0x80702d, 0x95614d, 0x727033],
            (Self::Rose, false) => [0xdca8b6, 0xc2a9ce, 0xd3b9bd, 0xd1b3a0],
            (Self::Rose, true) => [0x9b4b67, 0x815a94, 0x956573, 0x8c6549],
        }
    }
}
#[derive(Clone, Copy, Default)]
pub struct Appearance {
    pub light: bool,
    pub accent: Accent,
}
pub struct Colors {
    pub background: u32,
    pub surface: u32,
    pub well: u32,
    pub line: u32,
    pub text: u32,
    pub muted: u32,
    pub track: u32,
    pub cap: u32,
    pub channels: [u32; 4],
}
impl Appearance {
    pub fn colors(self) -> Colors {
        let channels = self.accent.channels(self.light);
        if self.light {
            Colors {
                background: 0xf7f7f3,
                surface: 0xe9ede9,
                well: 0xf7f9f6,
                line: 0xbfc8c8,
                text: 0x293138,
                muted: 0x65717a,
                track: 0xc3ceca,
                cap: 0xa9b6b2,
                channels,
            }
        } else {
            Colors {
                background: 0x232529,
                surface: 0x2d3035,
                well: 0x1c1f23,
                line: 0x404247,
                text: 0xe5e7e8,
                muted: 0xa0a6ad,
                track: 0x171a1e,
                cap: 0x747c82,
                channels,
            }
        }
    }
}
