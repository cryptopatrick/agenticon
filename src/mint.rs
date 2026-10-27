//! Minting agent IDs from public blockchain randomness.
//!
//! A block hash is public randomness that nobody could know before the block
//! was mined. Hashing it with a minter identifier gives an ID that anyone can
//! re-check. A hash alone does not guarantee uniqueness, though: with 10^12
//! IDs a collision becomes likely (~50%) after about 1.18 million mints.
//! Something has to keep track of which IDs are taken. Here that is the
//! [`Registry`]; in production it could be an on-chain contract.
//!
//! # Examples
//!
//! ```
//! use agenticon::mint::{Chain, ChainAnchor, Registry};
//!
//! let anchor = ChainAnchor::from_hex(
//!     Chain::Bitcoin,
//!     973_406,
//!     "00000000000000000000cca56f80c78369ba3f9f8870749eec2e7787e217ac16",
//! )?;
//! let mut registry = Registry::new();
//! let minted = registry.mint(&anchor, b"alice");
//! assert_eq!(minted.id.to_string(), "0751-2494-9084");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::str::FromStr;

use sha2::{Digest, Sha256};

use crate::{AgentId, ID_SPACE, ParseAgentIdError};

/// Domain-separation tag hashed into every derived ID.
const DOMAIN_TAG: &[u8] = b"agenticon/v1";

/// Blockchain the anchor comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Chain {
    /// Bitcoin mainnet.
    Bitcoin,
    /// Ethereum mainnet.
    Ethereum,
}

/// Formats as the lowercase chain name, which is also what gets hashed.
impl fmt::Display for Chain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Chain::Bitcoin => "bitcoin",
            Chain::Ethereum => "ethereum",
        })
    }
}

impl FromStr for Chain {
    type Err = ParseChainError;

    /// Accepts `bitcoin`/`btc` and `ethereum`/`eth`, in any case.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "bitcoin" | "btc" => Ok(Chain::Bitcoin),
            "ethereum" | "eth" => Ok(Chain::Ethereum),
            _ => Err(ParseChainError { input: s.to_string() }),
        }
    }
}

/// A string that names no supported [`Chain`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseChainError {
    input: String,
}

impl fmt::Display for ParseChainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown chain {:?}", self.input)
    }
}

impl Error for ParseChainError {}

/// A block on a public chain, used as a source of randomness.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChainAnchor {
    /// The chain.
    pub chain: Chain,
    /// Block height.
    pub height: u64,
    /// Block hash, in the byte order explorers display (big-endian hex).
    pub block_hash: [u8; 32],
}

impl ChainAnchor {
    /// Builds an anchor from a 64-hex-digit block hash; a `0x` prefix and
    /// surrounding whitespace are allowed.
    ///
    /// # Errors
    ///
    /// Returns [`ParseBlockHashError`] if `hash_hex` is not 32 hex bytes.
    pub fn from_hex(
        chain: Chain,
        height: u64,
        hash_hex: &str,
    ) -> Result<Self, ParseBlockHashError> {
        let err = || ParseBlockHashError { input: hash_hex.to_string() };
        let hex = hash_hex.trim().trim_start_matches("0x");
        if hex.len() != 64 || !hex.is_ascii() {
            return Err(err());
        }
        let mut block_hash = [0u8; 32];
        for (i, byte) in block_hash.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16)
                .map_err(|_| err())?;
        }
        Ok(ChainAnchor { chain, height, block_hash })
    }
}

/// A string that is not a 32-byte hex block hash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseBlockHashError {
    input: String,
}

impl fmt::Display for ParseBlockHashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid block hash {:?}: expected 64 hex digits",
            self.input
        )
    }
}

impl Error for ParseBlockHashError {}

/// Derives an ID from `SHA-256("agenticon/v1" | chain | height | hash |
/// minter length | minter | nonce)`.
///
/// The first 8 digest bytes are reduced modulo 10^12; the resulting bias of
/// about 5e-8 is negligible.
#[must_use]
pub fn derive_id(anchor: &ChainAnchor, minter: &[u8], nonce: u32) -> AgentId {
    let mut h = Sha256::new();
    h.update(DOMAIN_TAG);
    h.update(anchor.chain.to_string().as_bytes());
    h.update(anchor.height.to_be_bytes());
    h.update(anchor.block_hash);
    h.update((minter.len() as u64).to_be_bytes());
    h.update(minter);
    h.update(nonce.to_be_bytes());
    let digest = h.finalize();
    let mut first = [0u8; 8];
    first.copy_from_slice(&digest[..8]);
    AgentId::try_new(u64::from_be_bytes(first) % ID_SPACE)
        .expect("a value reduced modulo ID_SPACE is a valid id")
}

/// The result of a successful mint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Minted {
    /// The newly assigned ID.
    pub id: AgentId,
    /// The nonce that produced it (non-zero when earlier nonces collided).
    pub nonce: u32,
}

/// The set of minted IDs, used to keep every new mint unique.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Registry {
    taken: HashSet<AgentId>,
}

impl Registry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of minted IDs.
    #[must_use]
    pub fn len(&self) -> usize {
        self.taken.len()
    }

    /// Whether nothing has been minted.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.taken.is_empty()
    }

    /// Whether `id` is already taken.
    #[must_use]
    pub fn contains(&self, id: AgentId) -> bool {
        self.taken.contains(&id)
    }

    /// Marks an externally assigned ID as taken. Returns `false` if it
    /// already was.
    pub fn insert(&mut self, id: AgentId) -> bool {
        self.taken.insert(id)
    }

    /// Mints a fresh ID, trying nonces 0, 1, 2, … until one is free.
    ///
    /// # Panics
    ///
    /// Panics if all 2^32 nonces collide, which cannot happen before the
    /// registry holds billions of IDs.
    pub fn mint(&mut self, anchor: &ChainAnchor, minter: &[u8]) -> Minted {
        let mut nonce = 0u32;
        loop {
            let id = derive_id(anchor, minter, nonce);
            if self.taken.insert(id) {
                return Minted { id, nonce };
            }
            nonce = nonce
                .checked_add(1)
                .expect("every nonce for this anchor and minter is taken");
        }
    }

    /// Loads a registry from a text file with one ID per line; blank lines
    /// are skipped.
    ///
    /// # Errors
    ///
    /// Returns [`LoadRegistryError`] if the file cannot be read or a line is
    /// not a valid ID.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, LoadRegistryError> {
        let file = File::open(path)?;
        let mut reg = Registry::new();
        for (i, line) in BufReader::new(file).lines().enumerate() {
            let line = line?;
            let line = line.trim();
            if !line.is_empty() {
                let id = line.parse().map_err(|source| {
                    LoadRegistryError::Parse { line: i + 1, source }
                })?;
                reg.taken.insert(id);
            }
        }
        Ok(reg)
    }

    /// Saves the registry as a text file with one ID per line, sorted.
    ///
    /// # Errors
    ///
    /// Returns any I/O error from creating or writing the file.
    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let mut ids: Vec<AgentId> = self.taken.iter().copied().collect();
        ids.sort_unstable();
        let mut f = BufWriter::new(File::create(path)?);
        for id in ids {
            writeln!(f, "{id}")?;
        }
        f.flush()
    }
}

impl FromIterator<AgentId> for Registry {
    fn from_iter<I: IntoIterator<Item = AgentId>>(iter: I) -> Self {
        Registry { taken: iter.into_iter().collect() }
    }
}

impl Extend<AgentId> for Registry {
    fn extend<I: IntoIterator<Item = AgentId>>(&mut self, iter: I) {
        self.taken.extend(iter);
    }
}

/// Loading a [`Registry`] from a file failed.
#[derive(Debug)]
#[non_exhaustive]
pub enum LoadRegistryError {
    /// The file could not be read.
    Io(io::Error),
    /// A line is not a valid agent ID.
    Parse {
        /// 1-based line number.
        line: usize,
        /// Why the line was rejected.
        source: ParseAgentIdError,
    },
}

impl fmt::Display for LoadRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadRegistryError::Io(_) => f.write_str("cannot read registry"),
            LoadRegistryError::Parse { line, .. } => {
                write!(f, "invalid agent id on registry line {line}")
            }
        }
    }
}

impl Error for LoadRegistryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            LoadRegistryError::Io(e) => Some(e),
            LoadRegistryError::Parse { source, .. } => Some(source),
        }
    }
}

impl From<io::Error> for LoadRegistryError {
    fn from(e: io::Error) -> Self {
        LoadRegistryError::Io(e)
    }
}
