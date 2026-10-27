//! Renders a few neighbouring and random IDs side by side:
//! `cargo run --example gallery -- [OUT_DIR]`.
use agenticon::{AgentId, Agenticon};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::args().nth(1).unwrap_or("target/gallery".into());
    let ids = [0, 1, 2, 3, 42, 1_000_000, 123_456_789_012, 999_999_999_999];
    for v in ids {
        let icon = Agenticon::new(AgentId::try_new(v)?);
        icon.save_all(&dir, 256)?;
        println!("{icon}\n");
    }
    println!("wrote {} agenticons to {dir}", ids.len());
    Ok(())
}
