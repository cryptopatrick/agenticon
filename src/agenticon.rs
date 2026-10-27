//! The [`Agenticon`] type: one agent's complete visual and audible identity.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use crate::agent_id::{SOUND_KEYS, VISUAL_KEYS};
use crate::color::Palette;
use crate::genome::{SoundGenes, VisualGenes};
use crate::render::{self, RenderError, View};
use crate::sound;
use crate::{AgentId, ID_SPACE};

/// The full identity of one agent: a 3D view, a flat view and a jingle.
///
/// # Examples
///
/// ```
/// use agenticon::{AgentId, Agenticon};
///
/// let icon = Agenticon::new(AgentId::try_new(123_456_789_012)?);
/// let png_3d = icon.render_3d_png(256)?;
/// let png_flat = icon.render_flat_png(256)?;
/// let wav = icon.sound_wav();
/// assert!(png_3d.starts_with(b"\x89PNG") && png_flat.starts_with(b"\x89PNG"));
/// assert!(wav.starts_with(b"RIFF"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Agenticon {
    id: AgentId,
    visual: VisualGenes,
    sound: SoundGenes,
}

impl Agenticon {
    /// Builds the agenticon for `id`.
    #[must_use]
    pub fn new(id: AgentId) -> Self {
        Agenticon {
            id,
            visual: VisualGenes::decode(id.scramble(&VISUAL_KEYS)),
            sound: SoundGenes::decode(id.scramble(&SOUND_KEYS)),
        }
    }

    /// The agenticon whose visual genome is `genome`, or `None` if
    /// `genome >= 10^12` (no ID maps there).
    #[must_use]
    pub fn from_visual_genome(genome: u64) -> Option<Self> {
        (genome < ID_SPACE)
            .then(|| Agenticon::new(AgentId::unscramble(genome, &VISUAL_KEYS)))
    }

    /// The agenticon whose sound genome is `genome`, or `None` if
    /// `genome >= 10^12` (no ID maps there).
    #[must_use]
    pub fn from_sound_genome(genome: u64) -> Option<Self> {
        (genome < ID_SPACE)
            .then(|| Agenticon::new(AgentId::unscramble(genome, &SOUND_KEYS)))
    }

    /// The agent ID.
    #[must_use]
    pub fn id(&self) -> AgentId {
        self.id
    }

    /// Decoded visual traits.
    #[must_use]
    pub fn visual(&self) -> &VisualGenes {
        &self.visual
    }

    /// Decoded sound traits.
    #[must_use]
    pub fn sound(&self) -> &SoundGenes {
        &self.sound
    }

    /// The colour palette shared by both views.
    #[must_use]
    pub fn palette(&self) -> Palette {
        Palette::from_genes(&self.visual)
    }

    /// Raw RGBA pixels of a view, `size * size * 4` bytes, row-major and fully
    /// opaque.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] if `size` is zero or too large to allocate.
    pub fn render_rgba(
        &self,
        view: View,
        size: u32,
    ) -> Result<Vec<u8>, RenderError> {
        Ok(render::render(&self.visual, view, size)?.take())
    }

    /// A view encoded as PNG.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] if `size` is zero or too large to allocate, or
    /// if PNG encoding fails.
    pub fn render_png(
        &self,
        view: View,
        size: u32,
    ) -> Result<Vec<u8>, RenderError> {
        render::encode_png(&render::render(&self.visual, view, size)?)
    }

    /// The shaded 3D view as PNG; shorthand for
    /// [`render_png(View::Solid3d, size)`](Self::render_png).
    ///
    /// # Errors
    ///
    /// As for [`render_png`](Self::render_png).
    pub fn render_3d_png(&self, size: u32) -> Result<Vec<u8>, RenderError> {
        self.render_png(View::Solid3d, size)
    }

    /// The flat view as PNG; shorthand for
    /// [`render_png(View::Flat, size)`](Self::render_png).
    ///
    /// # Errors
    ///
    /// As for [`render_png`](Self::render_png).
    pub fn render_flat_png(&self, size: u32) -> Result<Vec<u8>, RenderError> {
        self.render_png(View::Flat, size)
    }

    /// The jingle as mono samples in `[-1, 1]` at [`sound::SAMPLE_RATE`].
    #[must_use]
    pub fn sound_samples(&self) -> Vec<f32> {
        sound::synthesize(&self.sound)
    }

    /// The jingle as a 16-bit mono PCM WAV file.
    #[must_use]
    pub fn sound_wav(&self) -> Vec<u8> {
        sound::to_wav(&self.sound_samples())
    }

    /// Writes `<id>-3d.png`, `<id>-flat.png` and `<id>.wav` into `dir`,
    /// creating it if needed. Existing files are overwritten.
    ///
    /// # Errors
    ///
    /// Returns [`SaveError`] if rendering fails or a file cannot be written.
    pub fn save_all(
        &self,
        dir: impl AsRef<Path>,
        size: u32,
    ) -> Result<(), SaveError> {
        let dir = dir.as_ref();
        fs::create_dir_all(dir)?;
        let stem = self.id.to_string();
        fs::write(
            dir.join(format!("{stem}-3d.png")),
            self.render_3d_png(size)?,
        )?;
        fs::write(
            dir.join(format!("{stem}-flat.png")),
            self.render_flat_png(size)?,
        )?;
        fs::write(dir.join(format!("{stem}.wav")), self.sound_wav())?;
        Ok(())
    }
}

impl From<AgentId> for Agenticon {
    fn from(id: AgentId) -> Self {
        Agenticon::new(id)
    }
}

/// A multi-line, human-readable summary of every trait.
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
            v.family(),
            v.sides(),
            v.yaw(),
            v.pitch(),
            v.stretch(),
            v.hue(),
            v.scheme(),
            v.background(),
            v.edge(),
            v.ring_pattern(),
            s.root(),
            s.tempo(),
            s.timbre(),
            s.rhythm(),
            s.notes(),
        )
    }
}

/// Saving an agenticon's files failed.
#[derive(Debug)]
#[non_exhaustive]
pub enum SaveError {
    /// Rendering an image failed.
    Render(RenderError),
    /// Creating the directory or writing a file failed.
    Io(io::Error),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::Render(_) => f.write_str("cannot render agenticon"),
            SaveError::Io(_) => f.write_str("cannot write agenticon files"),
        }
    }
}

impl Error for SaveError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SaveError::Render(e) => Some(e),
            SaveError::Io(e) => Some(e),
        }
    }
}

impl From<RenderError> for SaveError {
    fn from(e: RenderError) -> Self {
        SaveError::Render(e)
    }
}

impl From<io::Error> for SaveError {
    fn from(e: io::Error) -> Self {
        SaveError::Io(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_id::splitmix64;
    use crate::genome::GENOME_BITS;

    /// Flipping any single genome bit must change the output.
    #[test]
    fn every_visual_bit_changes_both_views() {
        let mut s = 99u64;
        for _ in 0..4 {
            s = splitmix64(s);
            let base = Agenticon::new(
                AgentId::try_new(s % ID_SPACE).expect("reduced into range"),
            );
            let g = base.visual().encode();
            let solid = base.render_rgba(View::Solid3d, 96).expect("renders");
            let flat = base.render_rgba(View::Flat, 96).expect("renders");
            for bit in 0..GENOME_BITS {
                let Some(other) = Agenticon::from_visual_genome(g ^ 1 << bit)
                else {
                    continue;
                };
                assert_ne!(
                    other.render_rgba(View::Solid3d, 96).expect("renders"),
                    solid,
                    "3d view ignores bit {bit} of genome {g:#x}"
                );
                assert_ne!(
                    other.render_rgba(View::Flat, 96).expect("renders"),
                    flat,
                    "flat view ignores bit {bit} of genome {g:#x}"
                );
            }
        }
    }

    #[test]
    fn every_sound_bit_changes_the_sound() {
        let mut s = 5u64;
        for _ in 0..4 {
            s = splitmix64(s);
            let base = Agenticon::new(
                AgentId::try_new(s % ID_SPACE).expect("reduced into range"),
            );
            let g = base.sound().encode();
            let samples = base.sound_samples();
            for bit in 0..GENOME_BITS {
                let Some(other) = Agenticon::from_sound_genome(g ^ 1 << bit)
                else {
                    continue;
                };
                assert_ne!(
                    other.sound_samples(),
                    samples,
                    "sound ignores bit {bit} of genome {g:#x}"
                );
            }
        }
    }
}
