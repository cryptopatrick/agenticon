//! Renders one agenticon: `cargo run --example basic -- [ID] [OUT_DIR]`.
use agenticon::{AgentId, Agenticon};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let id: AgentId =
        args.next().unwrap_or_else(|| "1234-5678-9012".into()).parse()?;
    let dir = args.next().unwrap_or_else(|| "target/agenticons".into());

    let icon = Agenticon::new(id);
    println!("{icon}");
    icon.save_all(&dir, 512)?;
    println!("wrote {dir}/{id}-3d.png, {id}-flat.png, {id}.wav");
    Ok(())
}
