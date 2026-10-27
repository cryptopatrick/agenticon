//! Agent IDs and the bijective scrambling that turns them into genomes.

use std::fmt;
use std::str::FromStr;

use crate::Error;

/// Number of distinct agent IDs: one trillion.
pub const ID_SPACE: u64 = 1_000_000_000_000;

const HALF_BITS: u32 = 20;
const HALF_MASK: u64 = (1 << HALF_BITS) - 1;

/// Round keys for the permutation feeding the visual genome.
pub(crate) const VISUAL_KEYS: [u64; 6] = [
    0x243F_6A88_85A3_08D3,
    0x1319_8A2E_0370_7344,
    0xA409_3822_299F_31D0,
    0x082E_FA98_EC4E_6C89,
    0x4528_21E6_38D0_1377,
    0xBE54_66CF_34E9_0C6C,
];

/// Round keys for the permutation feeding the sound genome.
pub(crate) const SOUND_KEYS: [u64; 6] = [
    0xC0AC_29B7_C97C_50DD,
    0x3F84_D5B5_B547_0917,
    0x9216_D5D9_8979_FB1B,
    0xD131_0BA6_98DF_B5AC,
    0x2FFD_72DB_D01A_DFB7,
    0xB8E1_AFED_6A26_7E96,
];

/// A unique agent identifier in the range `[0, 10^12)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AgentId(u64);

impl AgentId {
    /// Creates an ID, failing if `value >= 10^12`.
    pub fn new(value: u64) -> Result<Self, Error> {
        if value < ID_SPACE {
            Ok(AgentId(value))
        } else {
            Err(Error::IdOutOfRange(value))
        }
    }

    /// The raw numeric value.
    pub fn value(self) -> u64 {
        self.0
    }

    pub(crate) fn scramble(self, keys: &[u64; 6]) -> u64 {
        permute(self.0, keys)
    }

    pub(crate) fn unscramble(genome: u64, keys: &[u64; 6]) -> Self {
        AgentId(unpermute(genome, keys))
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v = self.0;
        write!(
            f,
            "{:04}-{:04}-{:04}",
            v / 100_000_000,
            v / 10_000 % 10_000,
            v % 10_000
        )
    }
}

impl FromStr for AgentId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let digits: String =
            s.chars().filter(|c| *c != '-' && *c != '_').collect();
        let value = digits
            .parse::<u64>()
            .map_err(|_| Error::ParseId(s.to_string()))?;
        AgentId::new(value)
    }
}

pub(crate) fn splitmix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Balanced Feistel network on 40-bit values; a bijection on `[0, 2^40)`.
fn feistel(x: u64, keys: &[u64; 6]) -> u64 {
    let (mut l, mut r) = ((x >> HALF_BITS) & HALF_MASK, x & HALF_MASK);
    for k in keys {
        (l, r) = (r, l ^ (splitmix64(r ^ k) & HALF_MASK));
    }
    (l << HALF_BITS) | r
}

fn feistel_inv(x: u64, keys: &[u64; 6]) -> u64 {
    let (mut l, mut r) = ((x >> HALF_BITS) & HALF_MASK, x & HALF_MASK);
    for k in keys.iter().rev() {
        (l, r) = (r ^ (splitmix64(l ^ k) & HALF_MASK), l);
    }
    (l << HALF_BITS) | r
}

/// Cycle-walking restricts the 40-bit permutation to `[0, 10^12)`.
fn permute(x: u64, keys: &[u64; 6]) -> u64 {
    let mut y = feistel(x, keys);
    while y >= ID_SPACE {
        y = feistel(y, keys);
    }
    y
}

fn unpermute(x: u64, keys: &[u64; 6]) -> u64 {
    let mut y = feistel_inv(x, keys);
    while y >= ID_SPACE {
        y = feistel_inv(y, keys);
    }
    y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permutation_round_trips() {
        let mut s = 1u64;
        for _ in 0..100_000 {
            s = splitmix64(s);
            let id = s % ID_SPACE;
            let g = permute(id, &VISUAL_KEYS);
            assert!(g < ID_SPACE);
            assert_eq!(unpermute(g, &VISUAL_KEYS), id);
        }
    }

    #[test]
    fn neighbours_are_scattered() {
        let a = permute(42, &VISUAL_KEYS);
        let b = permute(43, &VISUAL_KEYS);
        assert!((a ^ b).count_ones() > 8);
    }

    #[test]
    fn display_and_parse() {
        let id = AgentId::new(123_456_789_012).unwrap();
        assert_eq!(id.to_string(), "1234-5678-9012");
        assert_eq!("1234-5678-9012".parse::<AgentId>().unwrap(), id);
        assert_eq!(
            "7".parse::<AgentId>().unwrap().to_string(),
            "0000-0000-0007"
        );
        assert!(AgentId::new(ID_SPACE).is_err());
    }
}
