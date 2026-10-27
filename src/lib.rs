//! # agenticon
//!
//! Identicons for AI agents: each of 10^12 agent IDs maps to its own
//!
//! - **3D view**: a shaded perspective rendering of a convex solid,
//! - **flat view**: the same solid flattened along its axis,
//! - **sound**: a short FM-synthesised jingle.
//!
//! An ID is scrambled by a keyed bijective permutation into a 40-bit genome
//! (2^40 > 10^12), and every genome bit sets a visible or audible trait. So
//! the mapping ID -> output is injective by construction, and IDs n and n+1
//! look and sound unrelated.
//!
//! ```
//! use agenticon::{AgentId, Agenticon};
//!
//! let a = Agenticon::new(AgentId::new(123_456_789_012).unwrap());
//! let png_3d = a.render_3d_png(256).unwrap();
//! let png_flat = a.render_flat_png(256).unwrap();
//! let wav = a.sound_wav();
//! assert!(!png_3d.is_empty() && !png_flat.is_empty() && !wav.is_empty());
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::fmt;
use std::path::Path;

mod geometry;

pub mod genome;
pub mod id;
pub mod mint;
pub mod render;
pub mod sound;

pub use genome::{SoundGenes, VisualGenes};
pub use id::{AgentId, ID_SPACE};
pub use render::{Palette, View};

/// Errors produced by this crate.
#[derive(Debug)]
pub enum Error {
    /// The value is not below 10^12.
    IdOutOfRange(u64),
    /// The string is not a valid ID.
    ParseId(String),
    /// The string is not a 32-byte hex hash.
    InvalidHex(String),
    /// Rendering failed.
    Render(String),
    /// An I/O error.
    Io(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::IdOutOfRange(v) => write!(f, "id {v} is not below 10^12"),
            Error::ParseId(s) => write!(f, "invalid agent id: {s:?}"),
            Error::InvalidHex(s) => write!(f, "invalid 32-byte hex: {s:?}"),
            Error::Render(s) => write!(f, "render error: {s}"),
            Error::Io(e) => write!(f, "i/o error: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

/// The full identity of one agent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Agenticon {
    id: AgentId,
    visual: VisualGenes,
    sound: SoundGenes,
}

impl Agenticon {
    /// Builds the agenticon for `id`.
    pub fn new(id: AgentId) -> Self {
        Agenticon {
            id,
            visual: VisualGenes::decode(id.scramble(&id::VISUAL_KEYS)),
            sound: SoundGenes::decode(id.scramble(&id::SOUND_KEYS)),
        }
    }

    /// The agenticon whose visual genome is `genome`, if one exists
    /// (`genome < 10^12`).
    pub fn from_visual_genome(genome: u64) -> Option<Self> {
        (genome < ID_SPACE).then(|| {
            Agenticon::new(AgentId::unscramble(genome, &id::VISUAL_KEYS))
        })
    }

    /// The agenticon whose sound genome is `genome`, if one exists
    /// (`genome < 10^12`).
    pub fn from_sound_genome(genome: u64) -> Option<Self> {
        (genome < ID_SPACE).then(|| {
            Agenticon::new(AgentId::unscramble(genome, &id::SOUND_KEYS))
        })
    }

    /// The agent ID.
    pub fn id(&self) -> AgentId {
        self.id
    }

    /// Decoded visual traits.
    pub fn visual(&self) -> &VisualGenes {
        &self.visual
    }

    /// Decoded sound traits.
    pub fn sound(&self) -> &SoundGenes {
        &self.sound
    }

    /// Colour palette.
    pub fn palette(&self) -> Palette {
        Palette::from_genes(&self.visual)
    }

    /// Raw RGBA pixels (premultiplied, fully opaque) of a view.
    pub fn render_rgba(
        &self,
        view: View,
        size: u32,
    ) -> Result<Vec<u8>, Error> {
        Ok(render::render(&self.visual, view, size)?.take())
    }

    /// PNG of a view.
    pub fn render_png(&self, view: View, size: u32) -> Result<Vec<u8>, Error> {
        render::render(&self.visual, view, size)?
            .encode_png()
            .map_err(|e| Error::Render(e.to_string()))
    }

    /// PNG of the shaded 3D view.
    pub fn render_3d_png(&self, size: u32) -> Result<Vec<u8>, Error> {
        self.render_png(View::Solid3d, size)
    }

    /// PNG of the flat view.
    pub fn render_flat_png(&self, size: u32) -> Result<Vec<u8>, Error> {
        self.render_png(View::Flat, size)
    }

    /// Mono samples in `[-1, 1]` at [`sound::SAMPLE_RATE`].
    pub fn sound_samples(&self) -> Vec<f32> {
        sound::synthesize(&self.sound)
    }

    /// The jingle as a 16-bit PCM WAV file.
    pub fn sound_wav(&self) -> Vec<u8> {
        sound::to_wav(&self.sound_samples())
    }

    /// Writes `<id>-3d.png`, `<id>-flat.png` and `<id>.wav` into `dir`.
    pub fn save_all(
        &self,
        dir: impl AsRef<Path>,
        size: u32,
    ) -> Result<(), Error> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir)?;
        let stem = self.id.to_string();
        std::fs::write(
            dir.join(format!("{stem}-3d.png")),
            self.render_3d_png(size)?,
        )?;
        std::fs::write(
            dir.join(format!("{stem}-flat.png")),
            self.render_flat_png(size)?,
        )?;
        std::fs::write(dir.join(format!("{stem}.wav")), self.sound_wav())?;
        Ok(())
    }
}

impl fmt::Display for Agenticon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v = &self.visual;
        let s = &self.sound;
        write!(
            f,
            "agenticon {}\n  shape: {:?} n={} yaw={} pitch={} stretch={}\n  \
             colour: hue={} {:?} bg={:?} edge={:?}\n  ring: {:020b}\n  \
             sound: root={} tempo={} timbre={} rhythm={} notes={:?}",
            self.id,
            v.family,
            v.sides,
            v.yaw,
            v.pitch,
            v.stretch,
            v.hue,
            v.scheme,
            v.background,
            v.edge,
            v.ring_pattern(),
            s.root,
            s.tempo,
            s.timbre,
            s.rhythm,
            s.notes,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::GENOME_BITS;
    use crate::id::splitmix64;

    /// Flipping any single genome bit must change the output.
    #[test]
    fn every_visual_bit_changes_both_views() {
        let mut s = 99u64;
        for _ in 0..4 {
            s = splitmix64(s);
            let base = Agenticon::new(AgentId::new(s % ID_SPACE).unwrap());
            let g = base.visual().encode();
            let p3 = base.render_rgba(View::Solid3d, 96).unwrap();
            let pf = base.render_rgba(View::Flat, 96).unwrap();
            for bit in 0..GENOME_BITS {
                let Some(other) = Agenticon::from_visual_genome(g ^ 1 << bit)
                else {
                    continue;
                };
                assert_ne!(
                    other.render_rgba(View::Solid3d, 96).unwrap(),
                    p3,
                    "3d view ignores bit {bit}"
                );
                assert_ne!(
                    other.render_rgba(View::Flat, 96).unwrap(),
                    pf,
                    "flat view ignores bit {bit}"
                );
            }
        }
    }

    #[test]
    fn every_sound_bit_changes_the_sound() {
        let mut s = 5u64;
        for _ in 0..4 {
            s = splitmix64(s);
            let base = Agenticon::new(AgentId::new(s % ID_SPACE).unwrap());
            let g = base.sound().encode();
            let wav = base.sound_samples();
            for bit in 0..GENOME_BITS {
                let Some(other) = Agenticon::from_sound_genome(g ^ 1 << bit)
                else {
                    continue;
                };
                assert_ne!(
                    other.sound_samples(),
                    wav,
                    "sound ignores bit {bit}"
                );
            }
        }
    }
}
