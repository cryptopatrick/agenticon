//! Identicons for AI agents: a 3D view, a flat view and a jingle for each of
//! 10^12 agent IDs.
//!
//! - **3D view**: a shaded perspective rendering of a convex solid.
//! - **Flat view**: the same solid flattened along its axis.
//! - **Sound**: a short FM-synthesised jingle.
//!
//! A keyed bijective permutation scrambles each ID into a 40-bit genome
//! (2^40 > 10^12), and every genome bit sets a visible or audible trait. So
//! no two IDs share an agenticon, and IDs n and n+1 look and sound
//! unrelated. The [`mint`] module derives IDs from Bitcoin or Ethereum block
//! hashes.
//!
//! # Examples
//!
//! ```no_run
//! use agenticon::{AgentId, Agenticon};
//!
//! let icon = Agenticon::new("1234-5678-9012".parse::<AgentId>()?);
//! println!("{icon}");
//! icon.save_all("out", 512)?; // out/1234-5678-9012-3d.png, -flat.png, .wav
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod agent_id;
mod agenticon;
mod geometry;
mod render;

pub mod color;
pub mod genome;
pub mod mint;
pub mod sound;

pub use agent_id::{AgentId, ID_SPACE, IdOutOfRangeError, ParseAgentIdError};
pub use agenticon::{Agenticon, SaveError};
pub use render::{RenderError, View};

// Public types must stay usable across threads.
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Agenticon>();
    assert_send_sync::<AgentId>();
    assert_send_sync::<RenderError>();
    assert_send_sync::<SaveError>();
    assert_send_sync::<ParseAgentIdError>();
    assert_send_sync::<mint::Registry>();
    assert_send_sync::<mint::LoadRegistryError>();
};
