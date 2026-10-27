# agenticon

**agenticon is a library built around custom generation of identicon images for AI agents.**

[![Crates.io](https://img.shields.io/crates/v/agenticon.svg)](https://crates.io/crates/agenticon)
[![Documentation](https://docs.rs/agenticon/badge.svg)](https://docs.rs/agenticon)
[![License](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](https://github.com/cryptopatrick/agenticon)

## Overview
Every agent gets an ID in `[0, 10^12)` (shown as `XXXX-XXXX-XXXX`). A keyed
bijective permutation turns the ID into a 40-bit genome (2^40 > 10^12), and
every bit sets a trait. So no two IDs share an agenticon, and IDs `n` and
`n+1` look and sound unrelated.

## Key Features
- **3D view**: a shaded perspective view of a prism, antiprism, bipyramid or
  frustum (3–10 sides), with its own rotation, tilt, height, palette and
  20-segment orbit ring.
- **Flat view**: the same solid flattened along its axis (orthographic,
  unlit). It carries all 40 bits too.
- **Sound**: a ~2 s FM jingle (root, tempo, instrument, rhythm, 7 notes = 40 bits).
- **Minting**: derive IDs from a Bitcoin/Ethereum block hash + minter + nonce
  (SHA-256). A `Registry` re-rolls the nonce on collision, since no hash alone
  can guarantee uniqueness.

## Architecture

## Quick Start
```rust
use agenticon::{AgentId, Agenticon};

let icon = Agenticon::new("1234-5678-9012".parse::<AgentId>()?);
icon.save_all("out", 512)?; // out/1234-5678-9012-3d.png, -flat.png, .wav
```

## Documentation

## Examples
```sh
cargo run --example basic -- 1234-5678-9012
cargo run --example gallery
cargo run --release --example uniqueness -- 30000 48
H=$(curl -s https://mempool.space/api/blocks/tip/height)
B=$(curl -s https://mempool.space/api/blocks/tip/hash)
cargo run --example mint -- bitcoin $H $B alice bob
```

## Contributing
Contributions are welcome! 
Please see our [contributing guidelines](CONTRIBUTING.md) for details on:
- Code style and testing requirements
- Submitting bug reports and feature requests
- Development setup and workflow

## License
This project is licensed under the GNU Affero General Public License v3.0 only (AGPL-3.0-only). See [LICENSE](LICENSE) for details.
