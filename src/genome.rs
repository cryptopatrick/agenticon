//! Decoding a 40-bit genome into visual and sound traits.
//!
//! Each decoder consumes exactly 40 bits and has a matching encoder, so the
//! mapping genome -> traits is lossless (and therefore injective).

/// Total genome width in bits (2^40 > 10^12).
pub const GENOME_BITS: u32 = 40;

struct BitReader(u64);

impl BitReader {
    fn take(&mut self, n: u32) -> u64 {
        let v = self.0 & ((1 << n) - 1);
        self.0 >>= n;
        v
    }
}

struct BitWriter {
    value: u64,
    pos: u32,
}

impl BitWriter {
    fn put(&mut self, v: u64, n: u32) {
        self.value |= (v & ((1 << n) - 1)) << self.pos;
        self.pos += n;
    }
}

/// Family of convex solid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Family {
    /// Two n-gon caps joined by quads.
    Prism,
    /// Two twisted n-gon caps joined by triangles.
    Antiprism,
    /// Two pyramids glued at an n-gon equator.
    Bipyramid,
    /// Truncated pyramid with a smaller top cap.
    Frustum,
}

/// How the two accent colours relate to the base hue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Scheme {
    /// +30 / -30 degrees.
    Analogous,
    /// +120 / +240 degrees.
    Triadic,
    /// +150 / +210 degrees.
    SplitComplementary,
    /// +90 / +180 degrees.
    Square,
}

impl Scheme {
    /// Hue offsets (degrees) of the two accent colours.
    #[must_use]
    pub fn offsets(self) -> (f64, f64) {
        match self {
            Scheme::Analogous => (30.0, -30.0),
            Scheme::Triadic => (120.0, 240.0),
            Scheme::SplitComplementary => (150.0, 210.0),
            Scheme::Square => (90.0, 180.0),
        }
    }
}

/// Background treatment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Background {
    /// Near black.
    Dark,
    /// Near white.
    Light,
    /// Dark, tinted with the base hue.
    DarkTint,
    /// Light, tinted with the base hue.
    LightTint,
}

/// Outline treatment of faces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EdgeStyle {
    /// No outlines.
    None,
    /// Thin dark outlines.
    ThinDark,
    /// Thin light outlines.
    ThinLight,
    /// Thick dark outlines.
    ThickDark,
}

/// Number of segments in the orbit ring.
pub const RING_SEGMENTS: u32 = 20;

/// Visual traits decoded from the visual genome.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VisualGenes {
    /// Solid family (2 bits).
    pub(crate) family: Family,
    /// Polygon side count, 3..=10 (3 bits).
    pub(crate) sides: u8,
    /// Rotation about the solid's axis, 0..16 steps of one symmetry period
    /// (4 bits).
    pub(crate) yaw: u8,
    /// Camera tilt for the 3D view, 0..8 (3 bits).
    pub(crate) pitch: u8,
    /// Height of the solid, 0..4 (2 bits).
    pub(crate) stretch: u8,
    /// Base hue, 0..32 steps of 11.25 degrees (5 bits).
    pub(crate) hue: u8,
    /// Colour scheme (2 bits).
    pub(crate) scheme: Scheme,
    /// Background (2 bits).
    pub(crate) background: Background,
    /// Edge style (2 bits).
    pub(crate) edge: EdgeStyle,
    /// Free ring bits (15 bits). The ring also repeats `pitch` and `stretch`
    /// so the flat view, which cannot show them, still carries all 40 bits.
    pub(crate) ring_bits: u16,
}

impl VisualGenes {
    /// Decodes 40 bits of genome.
    #[must_use]
    pub fn decode(genome: u64) -> Self {
        let mut r = BitReader(genome);
        let family = match r.take(2) {
            0 => Family::Prism,
            1 => Family::Antiprism,
            2 => Family::Bipyramid,
            _ => Family::Frustum,
        };
        let sides = 3 + r.take(3) as u8;
        let yaw = r.take(4) as u8;
        let pitch = r.take(3) as u8;
        let stretch = r.take(2) as u8;
        let hue = r.take(5) as u8;
        let scheme = match r.take(2) {
            0 => Scheme::Analogous,
            1 => Scheme::Triadic,
            2 => Scheme::SplitComplementary,
            _ => Scheme::Square,
        };
        let background = match r.take(2) {
            0 => Background::Dark,
            1 => Background::Light,
            2 => Background::DarkTint,
            _ => Background::LightTint,
        };
        let edge = match r.take(2) {
            0 => EdgeStyle::None,
            1 => EdgeStyle::ThinDark,
            2 => EdgeStyle::ThinLight,
            _ => EdgeStyle::ThickDark,
        };
        let ring_bits = r.take(15) as u16;
        VisualGenes {
            family,
            sides,
            yaw,
            pitch,
            stretch,
            hue,
            scheme,
            background,
            edge,
            ring_bits,
        }
    }

    /// Re-encodes the traits into the genome they came from.
    #[must_use]
    pub fn encode(&self) -> u64 {
        let mut w = BitWriter { value: 0, pos: 0 };
        w.put(self.family as u64, 2);
        w.put(u64::from(self.sides) - 3, 3);
        w.put(u64::from(self.yaw), 4);
        w.put(u64::from(self.pitch), 3);
        w.put(u64::from(self.stretch), 2);
        w.put(u64::from(self.hue), 5);
        w.put(self.scheme as u64, 2);
        w.put(self.background as u64, 2);
        w.put(self.edge as u64, 2);
        w.put(u64::from(self.ring_bits), 15);
        debug_assert_eq!(w.pos, GENOME_BITS);
        w.value
    }

    /// Solid family.
    #[must_use]
    pub fn family(&self) -> Family {
        self.family
    }

    /// Polygon side count, 3..=10.
    #[must_use]
    pub fn sides(&self) -> u8 {
        self.sides
    }

    /// Rotation step about the solid's axis, 0..16.
    #[must_use]
    pub fn yaw(&self) -> u8 {
        self.yaw
    }

    /// Camera tilt step for the 3D view, 0..8.
    #[must_use]
    pub fn pitch(&self) -> u8 {
        self.pitch
    }

    /// Height step of the solid, 0..4.
    #[must_use]
    pub fn stretch(&self) -> u8 {
        self.stretch
    }

    /// Base hue step, 0..32 (11.25 degrees each).
    #[must_use]
    pub fn hue(&self) -> u8 {
        self.hue
    }

    /// Colour scheme.
    #[must_use]
    pub fn scheme(&self) -> Scheme {
        self.scheme
    }

    /// Background treatment.
    #[must_use]
    pub fn background(&self) -> Background {
        self.background
    }

    /// Outline treatment.
    #[must_use]
    pub fn edge(&self) -> EdgeStyle {
        self.edge
    }

    /// The 15 free ring bits.
    #[must_use]
    pub fn ring_bits(&self) -> u16 {
        self.ring_bits
    }

    /// The full 20-segment ring pattern: 15 free bits, then pitch, then
    /// stretch.
    #[must_use]
    pub fn ring_pattern(&self) -> u32 {
        u32::from(self.ring_bits)
            | u32::from(self.pitch) << 15
            | u32::from(self.stretch) << 18
    }
}

/// Note-duration patterns in 1/16ths of a phrase. All sum to 16, so no
/// pattern is a tempo-scaled copy of another.
pub(crate) const RHYTHMS: [[u8; 8]; 8] = [
    [2, 2, 2, 2, 2, 2, 2, 2],
    [3, 1, 3, 1, 3, 1, 2, 2],
    [1, 1, 2, 1, 1, 2, 4, 4],
    [4, 2, 2, 4, 1, 1, 1, 1],
    [2, 1, 1, 2, 2, 1, 1, 6],
    [1, 3, 1, 3, 1, 3, 2, 2],
    [3, 3, 2, 2, 2, 1, 1, 2],
    [1, 1, 1, 1, 4, 4, 2, 2],
];

/// Sound traits decoded from the sound genome.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SoundGenes {
    /// Root note in semitones above the base pitch, 0..16 (4 bits). The first
    /// note always plays the root, so the root is audible.
    pub(crate) root: u8,
    /// Tempo index, 0..4 (2 bits).
    pub(crate) tempo: u8,
    /// Instrument (FM patch) index, 0..8 (3 bits).
    pub(crate) timbre: u8,
    /// Index into [`RHYTHMS`] (3 bits).
    pub(crate) rhythm: u8,
    /// Seven melody notes after the root, each a step 0..16 on a
    /// three-octave pentatonic ladder (7 x 4 bits).
    pub(crate) notes: [u8; 7],
}

impl SoundGenes {
    /// Decodes 40 bits of genome.
    #[must_use]
    pub fn decode(genome: u64) -> Self {
        let mut r = BitReader(genome);
        let root = r.take(4) as u8;
        let tempo = r.take(2) as u8;
        let timbre = r.take(3) as u8;
        let rhythm = r.take(3) as u8;
        let mut notes = [0u8; 7];
        for n in &mut notes {
            *n = r.take(4) as u8;
        }
        SoundGenes { root, tempo, timbre, rhythm, notes }
    }

    /// Re-encodes the traits into the genome they came from.
    #[must_use]
    pub fn encode(&self) -> u64 {
        let mut w = BitWriter { value: 0, pos: 0 };
        w.put(u64::from(self.root), 4);
        w.put(u64::from(self.tempo), 2);
        w.put(u64::from(self.timbre), 3);
        w.put(u64::from(self.rhythm), 3);
        for n in self.notes {
            w.put(u64::from(n), 4);
        }
        debug_assert_eq!(w.pos, GENOME_BITS);
        w.value
    }

    /// Root note in semitones above D3, 0..16.
    #[must_use]
    pub fn root(&self) -> u8 {
        self.root
    }

    /// Tempo step, 0..4.
    #[must_use]
    pub fn tempo(&self) -> u8 {
        self.tempo
    }

    /// Instrument (FM patch) index, 0..8.
    #[must_use]
    pub fn timbre(&self) -> u8 {
        self.timbre
    }

    /// Rhythm pattern index, 0..8.
    #[must_use]
    pub fn rhythm(&self) -> u8 {
        self.rhythm
    }

    /// Melody steps after the root, each 0..16 on a pentatonic ladder.
    #[must_use]
    pub fn notes(&self) -> [u8; 7] {
        self.notes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_id::splitmix64;

    #[test]
    fn decoding_is_lossless() {
        let mut s = 7u64;
        for _ in 0..50_000 {
            s = splitmix64(s);
            let g = s & ((1 << GENOME_BITS) - 1);
            assert_eq!(VisualGenes::decode(g).encode(), g);
            assert_eq!(SoundGenes::decode(g).encode(), g);
        }
    }

    #[test]
    fn rhythms_are_distinct_and_balanced() {
        for (i, a) in RHYTHMS.iter().enumerate() {
            assert_eq!(a.iter().map(|&x| u32::from(x)).sum::<u32>(), 16);
            for b in &RHYTHMS[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }
}
