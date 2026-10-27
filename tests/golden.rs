//! Pins the exact output bytes of each release. If one of these fails, the
//! agenticon for an existing ID has changed, which breaks every published
//! identity.

use agenticon::mint::{Chain, ChainAnchor, derive_id};
use agenticon::{AgentId, Agenticon, View};
use sha2::{Digest, Sha256};

/// (id, sha256 of 128px 3D RGBA, sha256 of 128px flat RGBA, sha256 of WAV)
const GOLDEN: [(u64, &str, &str, &str); 4] = [
    (
        0,
        "40c146d1c0601aa4d88307b0733daf1d5d9a21e7432570b8592ec4bc9038f734",
        "2cbef88342933132b5d6da9e76a0479cd9e5349c757762fdd4ef941642ceaefe",
        "f5f4ac874ac39cc3757b3fdaddd8a9e103bd7bfa185e62722797bc52f56dbb88",
    ),
    (
        42,
        "5a1cc65b15a2f9a2e277ca7f58f83a030a8faaac6013ef52e384cab75efa6085",
        "a1b7db35925191704247206630a5a072073b85bb91d1c37087663e3dcdd22dc7",
        "b6aa6f8d75372d29afe4cb640d54ff6001aeff2dd9d3d1c5ec31a896e03b4714",
    ),
    (
        123_456_789_012,
        "cce1370a41aa5c6de31325697d0e2785bc31b0a3cad9466bc949213a45557563",
        "c382d6b5a2308d7f92e37284a56c237eb11b6cccad4fbd212af4f95bc89f69b4",
        "33c7d0450ef38ee70fef53c39666d1fb9cbb2423def3ab44d5d9b9ddb359b788",
    ),
    (
        999_999_999_999,
        "989ec265e77212eb98e11bfaa4b19712d572b3e45908d99dca4569584762c446",
        "f1041f40d53890a4c14a0a4e019c273b69894b3e3f68186e53cfb5022bcdcea6",
        "a03e2da42605a3803cb382d53855ce3697336b451ff63a21e4e006de566e7664",
    ),
];

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn outputs_match_previous_release() {
    for (value, solid, flat, wav) in GOLDEN {
        let id = AgentId::try_new(value).expect("golden ids are in range");
        let icon = Agenticon::new(id);
        let got = |view| {
            sha256_hex(&icon.render_rgba(view, 128).expect("128px renders"))
        };
        assert_eq!(got(View::Solid3d), solid, "3d view of {id} changed");
        assert_eq!(got(View::Flat), flat, "flat view of {id} changed");
        assert_eq!(
            sha256_hex(&icon.sound_wav()),
            wav,
            "sound of {id} changed"
        );
    }
}

#[test]
fn minted_ids_match_previous_release() {
    let anchor = ChainAnchor::from_hex(
        Chain::Bitcoin,
        973_406,
        "00000000000000000000cca56f80c78369ba3f9f8870749eec2e7787e217ac16",
    )
    .expect("valid block hash");
    assert_eq!(derive_id(&anchor, b"alice", 0).value(), 75_124_949_084);
}
