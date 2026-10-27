//! Behaviour tests through the public API.

use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};

use agenticon::mint::{Chain, ChainAnchor, Registry, derive_id};
use agenticon::{AgentId, Agenticon, ID_SPACE, View, sound};

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
            AgentId::try_new((s >> 16) % ID_SPACE).expect("reduced into range")
        })
        .collect()
}

#[test]
fn rendering_is_deterministic() {
    let a = Agenticon::new(AgentId::try_new(31_337).expect("valid id"));
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
    let a = Agenticon::new(AgentId::try_new(500).expect("valid id"));
    let b = Agenticon::new(AgentId::try_new(501).expect("valid id"));
    let diff = (a.visual().encode() ^ b.visual().encode()).count_ones();
    assert!(diff >= 10, "only {diff} visual bits differ");
}

#[test]
fn wav_header_is_valid() {
    let wav =
        Agenticon::new(AgentId::try_new(9).expect("valid id")).sound_wav();
    assert_eq!(&wav[0..4], b"RIFF");
    assert_eq!(&wav[8..16], b"WAVEfmt ");
    let secs = (wav.len() - 44) as f64 / 2.0 / sound::SAMPLE_RATE as f64;
    assert!((1.5..3.0).contains(&secs), "{secs}s");
}

const BTC_HASH: &str =
    "00000000000000000001b1dd27bb8e6ed6f8e3a0b4d3ef2b6c1f0a4e9a3c8d21";

#[test]
fn minting_is_deterministic_and_salted() {
    let anchor = ChainAnchor::from_hex(Chain::Bitcoin, 900_000, BTC_HASH)
        .expect("valid hash");
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
    let anchor = ChainAnchor::from_hex(Chain::Bitcoin, 900_000, BTC_HASH)
        .expect("valid hash");
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

#[test]
fn agent_id_display_parse_and_conversions() {
    let id = AgentId::try_new(123_456_789_012).unwrap();
    assert_eq!(id.to_string(), "1234-5678-9012");
    assert_eq!("1234-5678-9012".parse::<AgentId>().unwrap(), id);
    assert_eq!("7".parse::<AgentId>().unwrap().to_string(), "0000-0000-0007");
    assert_eq!(AgentId::try_from(123_456_789_012u64).unwrap(), id);
    assert_eq!(u64::from(id), 123_456_789_012);
    assert_eq!(Agenticon::from(id), Agenticon::new(id));
}

#[test]
fn agent_id_errors_explain_themselves() {
    use std::error::Error;

    let err = AgentId::try_new(ID_SPACE).unwrap_err();
    assert_eq!(err.value(), ID_SPACE);

    let not_a_number = "abc".parse::<AgentId>().unwrap_err();
    assert!(!not_a_number.is_out_of_range());
    assert!(not_a_number.source().is_none());

    let too_big = "1000000000000".parse::<AgentId>().unwrap_err();
    assert!(too_big.is_out_of_range());
    assert_eq!(too_big.input(), "1000000000000");
    assert!(too_big.source().is_some());
}

#[test]
fn zero_size_render_is_an_error_not_a_panic() {
    let a = Agenticon::new(AgentId::try_new(1).unwrap());
    let err = a.render_rgba(View::Flat, 0).unwrap_err();
    assert!(err.is_invalid_size(), "{err}");
}

#[test]
fn chain_parses_names_and_tickers() {
    assert_eq!("bitcoin".parse::<Chain>().unwrap(), Chain::Bitcoin);
    assert_eq!("ETH".parse::<Chain>().unwrap(), Chain::Ethereum);
    assert!("dogecoin".parse::<Chain>().is_err());
}

#[test]
fn malformed_block_hashes_are_rejected() {
    let ok =
        ChainAnchor::from_hex(Chain::Bitcoin, 1, &format!("0x{BTC_HASH}"));
    assert!(ok.is_ok());
    // 64 bytes but not 64 ASCII characters: must not panic on slicing.
    let non_ascii = "é".repeat(32);
    assert!(ChainAnchor::from_hex(Chain::Bitcoin, 1, &non_ascii).is_err());
    assert!(
        ChainAnchor::from_hex(Chain::Bitcoin, 1, &"zz".repeat(32)).is_err()
    );
}

#[test]
fn registry_collects_and_reports_bad_lines() {
    let ids = [3, 1, 2].map(|v| AgentId::try_new(v).unwrap());
    let mut reg: Registry = ids.into_iter().collect();
    reg.extend([AgentId::try_new(1).unwrap()]);
    assert_eq!(reg.len(), 3);

    let path = std::env::temp_dir().join("agenticon-registry-bad.txt");
    std::fs::write(&path, "0000-0000-0001\n\nnot-an-id\n").unwrap();
    let err = Registry::load(&path).unwrap_err();
    assert!(
        matches!(
            err,
            agenticon::mint::LoadRegistryError::Parse { line: 3, .. }
        ),
        "{err:?}"
    );
    std::fs::remove_file(path).ok();
}
