//! Colours and palettes derived from the visual genome.

use std::fmt;

use crate::genome::{Background, VisualGenes};

/// An opaque sRGB colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgb {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
}

impl Rgb {
    /// Creates a colour from its channels.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Rgb { r, g, b }
    }

    /// Multiplies every channel by `k`, saturating at 0 and 255.
    pub(crate) fn scale(self, k: f64) -> Rgb {
        let f = |c: u8| (f64::from(c) * k).round().clamp(0.0, 255.0) as u8;
        Rgb::new(f(self.r), f(self.g), f(self.b))
    }
}

/// Formats as `#rrggbb`.
impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// Converts HSL (hue in degrees, saturation and lightness in `0..=1`) to RGB.
pub(crate) fn hsl(h: f64, s: f64, l: f64) -> Rgb {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let to8 = |v: f64| ((v + m) * 255.0).round() as u8;
    Rgb::new(to8(r), to8(g), to8(b))
}

/// Number of hue steps around the colour wheel (5 genome bits).
const HUE_STEPS: f64 = 32.0;

/// The four colours of one agenticon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Palette {
    /// Base colour: solid caps and the ring anchor.
    pub base: Rgb,
    /// First accent: side faces and lit ring segments.
    pub accent1: Rgb,
    /// Second accent: side faces and the ring track.
    pub accent2: Rgb,
    /// Background colour.
    pub background: Rgb,
}

impl Palette {
    /// Builds the palette for the given genes.
    #[must_use]
    pub fn from_genes(g: &VisualGenes) -> Palette {
        let h = f64::from(g.hue) * 360.0 / HUE_STEPS;
        let (o1, o2) = g.scheme.offsets();
        let background = match g.background {
            Background::Dark => Rgb::new(18, 18, 24),
            Background::Light => Rgb::new(240, 240, 236),
            Background::DarkTint => hsl(h, 0.35, 0.12),
            Background::LightTint => hsl(h, 0.45, 0.92),
        };
        Palette {
            base: hsl(h, 0.65, 0.55),
            accent1: hsl(h + o1, 0.6, 0.62),
            accent2: hsl(h + o2, 0.55, 0.44),
            background,
        }
    }
}
