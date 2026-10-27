//! Mints agent IDs from a real block:
//!
//! ```sh
//! H=$(curl -s https://mempool.space/api/blocks/tip/height)
//! B=$(curl -s https://mempool.space/api/blocks/tip/hash)
//! cargo run --example mint -- bitcoin $H $B alice bob carol
//! ```
use agenticon::Agenticon;
use agenticon::mint::{Chain, ChainAnchor, Registry};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 4 {
        eprintln!(
            "usage: mint <bitcoin|ethereum> <height> <block-hash> <minter>..."
        );
        std::process::exit(2);
    }
    let chain = match args[0].as_str() {
        "ethereum" | "eth" => Chain::Ethereum,
        _ => Chain::Bitcoin,
    };
    let anchor = ChainAnchor::from_hex(chain, args[1].parse()?, &args[2])?;
    let mut registry = Registry::new();
    for minter in &args[3..] {
        let m = registry.mint(&anchor, minter.as_bytes());
        let icon = Agenticon::new(m.id);
        icon.save_all("target/minted", 512)?;
        println!("{minter} -> {} (nonce {})\n{icon}\n", m.id, m.nonce);
    }
    println!(
        "anchored to {chain} block {}; files in target/minted",
        anchor.height
    );
    Ok(())
}
