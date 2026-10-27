//! FM-synthesised identity jingles.

use std::f64::consts::TAU;

use crate::genome::{RHYTHMS, SoundGenes};

/// Output sample rate in Hz.
pub const SAMPLE_RATE: u32 = 44_100;

/// MIDI note of root step 0 (D3).
const BASE_MIDI: i32 = 50;

/// Three-octave major-pentatonic ladder, in semitones above the root.
const LADDER: [i32; 16] =
    [0, 2, 4, 7, 9, 12, 14, 16, 19, 21, 24, 26, 28, 31, 33, 36];

/// Length of one rhythm unit (1/16 of the phrase) per tempo index.
const UNIT_MS: [f64; 4] = [95.0, 115.0, 135.0, 155.0];

/// FM patches: (modulator ratio, modulation index, decay rate per second).
const PATCHES: [(f64, f64, f64); 8] = [
    (1.0, 0.0, 3.0),
    (1.0, 1.5, 4.0),
    (2.0, 2.0, 5.0),
    (3.0, 1.2, 6.0),
    (0.5, 1.0, 3.0),
    (3.5, 2.5, 8.0),
    (1.41, 3.0, 9.0),
    (4.0, 0.8, 2.0),
];

/// One note of the jingle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Note {
    /// MIDI note number.
    pub midi: i32,
    /// Duration in seconds.
    pub seconds: f64,
}

/// The note sequence for the given genes. The first note is always the root.
pub fn melody(g: &SoundGenes) -> Vec<Note> {
    let unit = UNIT_MS[g.tempo as usize] / 1000.0;
    let rhythm = RHYTHMS[g.rhythm as usize];
    std::iter::once(0)
        .chain(g.notes.iter().map(|&n| LADDER[n as usize]))
        .zip(rhythm)
        .map(|(step, units)| Note {
            midi: BASE_MIDI + g.root as i32 + step,
            seconds: units as f64 * unit,
        })
        .collect()
}

/// Synthesises the jingle as mono samples in `[-1, 1]`.
pub fn synthesize(g: &SoundGenes) -> Vec<f32> {
    let (ratio, index, decay) = PATCHES[g.timbre as usize];
    let sr = SAMPLE_RATE as f64;
    let mut out = Vec::new();
    for note in melody(g) {
        let f = 440.0 * 2f64.powf((note.midi - 69) as f64 / 12.0);
        let n = (note.seconds * sr) as usize;
        let release = (0.015 * sr) as usize;
        for i in 0..n {
            let t = i as f64 / sr;
            let attack = (t / 0.005).min(1.0);
            let tail = ((n - i) as f64 / release as f64).min(1.0);
            let env = attack * tail * (-decay * t).exp();
            let m = index * env * (TAU * f * ratio * t).sin();
            out.push((0.6 * env * (TAU * f * t + m).sin()) as f32);
        }
    }
    out.extend(std::iter::repeat_n(0.0, (0.2 * sr) as usize));
    out
}

/// Encodes mono samples as a 16-bit PCM WAV file.
pub fn to_wav(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut w = Vec::with_capacity(44 + data_len as usize);
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + data_len).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes()); // PCM
    w.extend_from_slice(&1u16.to_le_bytes()); // mono
    w.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    w.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    w.extend_from_slice(&2u16.to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        w.extend_from_slice(&v.to_le_bytes());
    }
    w
}
