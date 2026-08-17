use std::{fs, path::Path};

use crate::rng::{
    BattleSeed, InjuryRng, LootRng, ProcgenRng, ReactionRng, SeverityRng, ShotRng,
    derivation::fnv1a64,
};

const STREAM_LEN: usize = 64;

fn shot_stream(root: u64) -> Vec<u64> {
    let mut rng = ShotRng::from_root(BattleSeed::new(root));
    (0..STREAM_LEN).map(|_| rng.next_u64()).collect()
}

fn severity_stream(root: u64) -> Vec<u64> {
    let mut rng = SeverityRng::from_root(BattleSeed::new(root));
    (0..STREAM_LEN).map(|_| rng.next_u64()).collect()
}

#[test]
fn same_seed_same_stream() {
    let a = shot_stream(0xDEAD_BEEF);
    let b = shot_stream(0xDEAD_BEEF);
    assert_eq!(a, b);
}

#[test]
fn different_seed_different_stream() {
    let a = shot_stream(0xDEAD_BEEF);
    let c = shot_stream(0x1234_5678);
    assert_ne!(a, c);
}

#[test]
fn rng_handle_draws_from_the_same_seeded_stream() {
    let mut via_handle = ShotRng::from_root(BattleSeed::new(42));
    let mut via_method = ShotRng::from_root(BattleSeed::new(42));
    let handle = via_handle.rng();
    let from_handle: Vec<u64> = (0..STREAM_LEN)
        .map(|_| {
            use rand::RngExt as _;
            handle.random()
        })
        .collect();
    let from_method: Vec<u64> = (0..STREAM_LEN).map(|_| via_method.next_u64()).collect();
    assert_eq!(from_handle, from_method);
}

#[test]
fn battle_seed_derefs_to_inner() {
    let seed = BattleSeed::new(7);
    assert_eq!(*seed, 7u64);
}

#[test]
fn stream_independence_shot_unaffected_by_severity_draws() {
    let root = BattleSeed::new(0xC0_FFEE_1234);

    let baseline = shot_stream(*root);

    let mut sev = SeverityRng::from_root(root);
    for _ in 0..STREAM_LEN {
        sev.next_u64();
    }
    let after_severity_draws = shot_stream(*root);

    assert_eq!(
        baseline, after_severity_draws,
        "ShotRng's output must be unaffected by prior SeverityRng draws — the two \
         streams are derived from the same root but are INDEPENDENT (different labels)"
    );
}

#[test]
fn streams_produce_different_sequences_from_same_root() {
    let root = BattleSeed::new(0xABCD_EF01_2345_6789);
    let shot = shot_stream(*root);
    let sev = severity_stream(*root);
    assert_ne!(
        shot, sev,
        "ShotRng and SeverityRng must produce DIFFERENT sequences from the same root \
         (different labels → different FNV hash → different ChaCha12 key)"
    );
}

#[test]
fn all_six_streams_produce_distinct_sequences() {
    let root = BattleSeed::new(0x1234_5678_9ABC_DEF0);
    let shot: Vec<u64> = {
        let mut r = ShotRng::from_root(root);
        (0..STREAM_LEN).map(|_| r.next_u64()).collect()
    };
    let sev: Vec<u64> = {
        let mut r = SeverityRng::from_root(root);
        (0..STREAM_LEN).map(|_| r.next_u64()).collect()
    };
    let loot: Vec<u64> = {
        let mut r = LootRng::from_root(root);
        (0..STREAM_LEN).map(|_| r.next_u64()).collect()
    };
    let injury: Vec<u64> = {
        let mut r = InjuryRng::from_root(root);
        (0..STREAM_LEN).map(|_| r.next_u64()).collect()
    };
    let procgen: Vec<u64> = {
        let mut r = ProcgenRng::from_root(root);
        (0..STREAM_LEN).map(|_| r.next_u64()).collect()
    };
    let reaction: Vec<u64> = {
        let mut r = ReactionRng::from_root(root);
        (0..STREAM_LEN).map(|_| r.next_u64()).collect()
    };
    let streams = [
        ("ShotRng", &shot),
        ("SeverityRng", &sev),
        ("LootRng", &loot),
        ("InjuryRng", &injury),
        ("ProcgenRng", &procgen),
        ("ReactionRng", &reaction),
    ];
    for i in 0..streams.len() {
        for j in (i + 1)..streams.len() {
            assert_ne!(
                streams[i].1, streams[j].1,
                "streams {} and {} must produce different sequences from the same root",
                streams[i].0, streams[j].0,
            );
        }
    }
}

const PINNED_SHOT_FIRST_U64: u64 = 0xb6e6_9916_cc82_4771;

#[test]
fn pinned_shot_first_draw() {
    let actual = ShotRng::from_root(BattleSeed::new(0)).next_u64();

    assert_eq!(
        actual, PINNED_SHOT_FIRST_U64,
        "ShotRng pinned first draw changed — the FNV derivation, label, or ChaCha12 \
         keystream was altered. If this is intentional (label bump), recompute the \
         constant and update PINNED_SHOT_FIRST_U64."
    );
}

#[test]
fn pinned_severity_first_draw() {
    const PINNED: u64 = 0x7d67_812c_25e8_fb3e;
    let actual = SeverityRng::from_root(BattleSeed::new(0)).next_u64();
    assert_eq!(actual, PINNED, "SeverityRng pinned first draw changed");
}

#[test]
fn fnv1a64_is_stable() {
    let computed = fnv1a64(BattleSeed::new(0), b"gdtf.rng.shot.v1");
    let expected: u64 = 0xb114_3619_01d0_9046;
    assert_eq!(
        computed.get(),
        expected,
        "fnv1a64 output for (0, b\"gdtf.rng.shot.v1\") changed — the FNV constants \
         or label were altered"
    );
}

fn rs_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn sim_src_has_no_global_or_implicit_entropy_source() {
    let token_a = ["thread", "rng"].join("_");
    let token_b = format!("Os{}", "Rng");
    let token_c = format!("rand::{}", "random");
    let token_d = format!("System{}", "Time");
    let token_e = format!("UNIX_{}", "EPOCH");
    let token_f = ["std::", "env"].join("");
    let token_g = ["env::", "var"].join("");
    let token_h = format!("rand::{}", "rng");
    let token_i = format!("Thread{}", "Rng");
    let token_j = ["get", "random"].join("");
    let forbidden = [
        token_a.as_str(),
        token_b.as_str(),
        token_c.as_str(),
        token_d.as_str(),
        token_e.as_str(),
        token_f.as_str(),
        token_g.as_str(),
        token_h.as_str(),
        token_i.as_str(),
        token_j.as_str(),
    ];

    let src_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rs_files(&src_root, &mut files);
    assert_ne!(files, Vec::<std::path::PathBuf>::new());

    for file in &files {
        let Ok(contents) = fs::read_to_string(file) else {
            continue;
        };
        for (line_no, line) in contents.lines().enumerate() {
            let code = line.split("//").next().unwrap_or_default();
            for token in &forbidden {
                assert!(
                    !code.contains(token),
                    "forbidden global/thread/time/env RNG token `{token}` in {}:{}\n\
                     (sim must stay entropy-free — composition root gdtf_app owns env/time reads)",
                    file.display(),
                    line_no + 1,
                );
            }
        }
    }
}
