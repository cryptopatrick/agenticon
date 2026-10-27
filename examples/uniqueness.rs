//! Empirical uniqueness check over random IDs:
//! `cargo run --release --example uniqueness -- [N] [SIZE]`.
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};

use agenticon::{AgentId, Agenticon, ID_SPACE, View};

fn fp<T: Hash + ?Sized>(v: &T) -> u64 {
    let mut h = DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let n: usize = args.next().map_or(Ok(20_000), |s| s.parse())?;
    let size: u32 = args.next().map_or(Ok(64), |s| s.parse())?;
    let (mut ids, mut v3, mut vf, mut snd) =
        (HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new());
    let mut s = 0x5EEDu64;
    while ids.len() < n {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let id = AgentId::new((s >> 16) % ID_SPACE)?;
        if !ids.insert(id) {
            continue;
        }
        let a = Agenticon::new(id);
        v3.insert(fp(&a.render_rgba(View::Solid3d, size)?));
        vf.insert(fp(&a.render_rgba(View::Flat, size)?));
        let bits: Vec<u32> =
            a.sound_samples().iter().map(|x| x.to_bits()).collect();
        snd.insert(fp(&bits));
    }
    println!("{n} distinct ids at {size}px:");
    println!("  3d view  : {} unique ({} collisions)", v3.len(), n - v3.len());
    println!("  flat view: {} unique ({} collisions)", vf.len(), n - vf.len());
    println!(
        "  sound    : {} unique ({} collisions)",
        snd.len(),
        n - snd.len()
    );
    Ok(())
}
