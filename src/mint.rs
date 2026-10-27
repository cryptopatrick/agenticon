//! Minting agent IDs from public blockchain randomness.
//!
//! A block hash is public randomness that nobody could know before the block
//! was mined. Hashing it with a minter identifier gives an ID that anyone can
//! re-check. A hash alone does not guarantee uniqueness, though: with 10^12
//! IDs a collision becomes likely (~50%) after about 1.18 million mints.
//! Something has to keep track of which IDs are taken. Here that is the
//! [`Registry`]; in production it could be an on-chain contract.

use std::collections::HashSet;
use std::fmt;
use std::io::{BufRead, Write};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::{AgentId, Error, ID_SPACE};

/// Blockchain the anchor comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chain {
    /// Bitcoin mainnet.
    Bitcoin,
    /// Ethereum mainnet.
    Ethereum,
}

impl fmt::Display for Chain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Chain::Bitcoin => "bitcoin",
            Chain::Ethereum => "ethereum",
        })
    }
}

/// A block on a public chain, used as a source of randomness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChainAnchor {
    /// The chain.
    pub chain: Chain,
    /// Block height.
    pub height: u64,
    /// Block hash, as displayed by explorers (big-endian hex).
    pub block_hash: [u8; 32],
}

impl ChainAnchor {
    /// Parses a 64-hex-digit block hash (an optional `0x` prefix is allowed).
    pub fn from_hex(
        chain: Chain,
        height: u64,
        hash_hex: &str,
    ) -> Result<Self, Error> {
        let hex = hash_hex.trim().trim_start_matches("0x");
        if hex.len() != 64 {
            return Err(Error::InvalidHex(hash_hex.to_string()));
        }
        let mut block_hash = [0u8; 32];
        for (i, byte) in block_hash.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16)
                .map_err(|_| Error::InvalidHex(hash_hex.to_string()))?;
        }
        Ok(ChainAnchor { chain, height, block_hash })
    }
}

/// Derives an ID from `SHA-256("agenticon/v1" | chain | height | hash |
/// minter | nonce)`. Reducing modulo 10^12 adds a bias of about 5e-8, which
/// is negligible.
pub fn derive_id(anchor: &ChainAnchor, minter: &[u8], nonce: u32) -> AgentId {
    let mut h = Sha256::new();
    h.update(b"agenticon/v1");
    h.update(anchor.chain.to_string().as_bytes());
    h.update(anchor.height.to_be_bytes());
    h.update(anchor.block_hash);
    h.update((minter.len() as u64).to_be_bytes());
    h.update(minter);
    h.update(nonce.to_be_bytes());
    let digest = h.finalize();
    let mut first = [0u8; 8];
    first.copy_from_slice(&digest[..8]);
    AgentId::new(u64::from_be_bytes(first) % ID_SPACE)
        .expect("value reduced modulo ID_SPACE")
}

/// The result of a successful mint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Minted {
    /// The newly assigned ID.
    pub id: AgentId,
    /// The nonce that produced it (non-zero when earlier nonces collided).
    pub nonce: u32,
}

/// Keeps track of minted IDs so every new mint is unique.
#[derive(Clone, Debug, Default)]
pub struct Registry {
    taken: HashSet<AgentId>,
}

impl Registry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of minted IDs.
    pub fn len(&self) -> usize {
        self.taken.len()
    }

    /// Whether nothing has been minted.
    pub fn is_empty(&self) -> bool {
        self.taken.is_empty()
    }

    /// Whether `id` is already taken.
    pub fn contains(&self, id: AgentId) -> bool {
        self.taken.contains(&id)
    }

    /// Marks an externally assigned ID as taken. Returns false if it already
    /// was.
    pub fn insert(&mut self, id: AgentId) -> bool {
        self.taken.insert(id)
    }

    /// Mints a fresh ID, trying successive nonces until one is free.
    pub fn mint(&mut self, anchor: &ChainAnchor, minter: &[u8]) -> Minted {
        let mut nonce = 0u32;
        loop {
            let id = derive_id(anchor, minter, nonce);
            if self.taken.insert(id) {
                return Minted { id, nonce };
            }
            nonce += 1;
        }
    }

    /// Loads a registry from a text file with one ID per line.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let file = std::fs::File::open(path)?;
        let mut reg = Registry::new();
        for line in std::io::BufReader::new(file).lines() {
            let line = line?;
            if !line.trim().is_empty() {
                reg.taken.insert(line.trim().parse()?);
            }
        }
        Ok(reg)
    }

    /// Saves the registry as a text file with one ID per line, sorted.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let mut ids: Vec<_> = self.taken.iter().copied().collect();
        ids.sort();
        let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
        for id in ids {
            writeln!(f, "{id}")?;
        }
        Ok(())
    }
}
