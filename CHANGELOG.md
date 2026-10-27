## Unreleased

## v0.2.0 - 2026-10-27
API cleanup following the Rust API Guidelines. Every agenticon is unchanged:
the same ID renders the same PNG and WAV bytes and mints the same IDs as in
0.1.0, and `tests/golden.rs` pins this.

 - *Breaking*: `AgentId::new` is now `AgentId::try_new` and returns
   `IdOutOfRangeError`; `TryFrom<u64>` and `From<AgentId> for u64` added.
 - *Breaking*: the crate-wide `Error` enum is replaced by specific errors:
   `ParseAgentIdError`, `RenderError`, `SaveError`, `mint::ParseBlockHashError`,
   `mint::ParseChainError` and `mint::LoadRegistryError`, all with `source()`.
 - *Breaking*: `VisualGenes` and `SoundGenes` fields are private; use the
   accessor methods. Public enums and data structs are `#[non_exhaustive]`.
 - *Breaking*: each item has one path. `AgentId`, `ID_SPACE`, `View` and the
   error types live at the crate root. `Rgb` and `Palette` move to `color`,
   and `Rgb` has named fields `r`, `g`, `b`. The `id` and `render` modules are
   private.
 - `Registry` implements `FromIterator` and `Extend`; `Registry::save`
   returns `io::Result`.
 - `Chain` implements `FromStr` (`bitcoin`/`btc`, `ethereum`/`eth`).
 - `Agenticon` implements `From<AgentId>` and `Hash`; `Rgb` implements
   `Display` as `#rrggbb`.
 - Fix: `ChainAnchor::from_hex` panicked on 64-byte non-ASCII input; it now
   returns an error.
 - Lints configured in `Cargo.toml`; MSRV set to 1.87; GitHub Actions CI added.

## v0.1.0 - 2026-10-27
 - Initial release: 10^12 IDs, 3D and flat views, FM jingle, blockchain
   anchored minting.

Flags: *Note*, *Breaking*:
