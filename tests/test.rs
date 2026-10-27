use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};

use agenticon::mint::{Chain, ChainAnchor, Registry, derive_id};
use agenticon::*;

fn fingerprint<T: Hash + ?Sized>(v: &T) -> u64 {
    let mut h = DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

fn sample_ids(n: usize, seed: u64) -> Vec<AgentId> {
    let mut s = seed;
    (0..n)
        .map(|_| {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            AgentId::new((s >> 16) % ID_SPACE).unwrap()
        })
        .collect()
}

#[test]
fn rendering_is_deterministic() {
    let a = Agenticon::new(AgentId::new(31_337).unwrap());
    assert_eq!(a.render_3d_png(64).unwrap(), a.render_3d_png(64).unwrap());
    assert_eq!(a.sound_wav(), a.sound_wav());
}

#[test]
fn random_sample_has_no_output_collisions() {
    let ids = sample_ids(1500, 1);
    let (mut v3, mut vf, mut snd) =
        (HashSet::new(), HashSet::new(), HashSet::new());
    for id in &ids {
        let a = Agenticon::new(*id);
        assert!(
            v3.insert(fingerprint(&a.render_rgba(View::Solid3d, 64).unwrap()))
        );
        assert!(
            vf.insert(fingerprint(&a.render_rgba(View::Flat, 64).unwrap()))
        );
        let s: Vec<u32> =
            a.sound_samples().iter().map(|x| x.to_bits()).collect();
        assert!(snd.insert(fingerprint(&s)));
    }
}

#[test]
fn neighbouring_ids_share_few_traits() {
    let a = Agenticon::new(AgentId::new(500).unwrap());
    let b = Agenticon::new(AgentId::new(501).unwrap());
    let diff = (a.visual().encode() ^ b.visual().encode()).count_ones();
    assert!(diff >= 10, "only {diff} visual bits differ");
}

#[test]
fn wav_header_is_valid() {
    let wav = Agenticon::new(AgentId::new(9).unwrap()).sound_wav();
    assert_eq!(&wav[0..4], b"RIFF");
    assert_eq!(&wav[8..16], b"WAVEfmt ");
    let secs = (wav.len() - 44) as f64 / 2.0 / sound::SAMPLE_RATE as f64;
    assert!((1.5..3.0).contains(&secs), "{secs}s");
}

const BTC_HASH: &str =
    "00000000000000000001b1dd27bb8e6ed6f8e3a0b4d3ef2b6c1f0a4e9a3c8d21";

#[test]
fn minting_is_deterministic_and_salted() {
    let anchor =
        ChainAnchor::from_hex(Chain::Bitcoin, 900_000, BTC_HASH).unwrap();
    assert_eq!(
        derive_id(&anchor, b"alice", 0),
        derive_id(&anchor, b"alice", 0)
    );
    assert_ne!(derive_id(&anchor, b"alice", 0), derive_id(&anchor, b"bob", 0));
    assert_ne!(
        derive_id(&anchor, b"alice", 0),
        derive_id(&anchor, b"alice", 1)
    );
    let eth = ChainAnchor { chain: Chain::Ethereum, ..anchor };
    assert_ne!(derive_id(&anchor, b"alice", 0), derive_id(&eth, b"alice", 0));
    assert!(ChainAnchor::from_hex(Chain::Bitcoin, 1, "abc").is_err());
}

#[test]
fn registry_rerolls_on_collision() {
    let anchor =
        ChainAnchor::from_hex(Chain::Bitcoin, 900_000, BTC_HASH).unwrap();
    let mut reg = Registry::new();
    reg.insert(derive_id(&anchor, b"alice", 0));
    let m = reg.mint(&anchor, b"alice");
    assert_eq!(m.nonce, 1);
    assert_eq!(m.id, derive_id(&anchor, b"alice", 1));
    assert_eq!(reg.len(), 2);

    let path = std::env::temp_dir().join("agenticon-registry-test.txt");
    reg.save(&path).unwrap();
    let loaded = Registry::load(&path).unwrap();
    assert!(loaded.contains(m.id) && loaded.len() == 2);
    std::fs::remove_file(path).ok();
}
