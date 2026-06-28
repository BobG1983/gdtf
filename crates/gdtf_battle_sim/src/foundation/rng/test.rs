//! Tests for the GTW-14 / GTW-466 per-subsystem RNG streams: stream
//! independence, replay determinism, pinned-output regression, FNV-1a-64
//! stability, and the entropy source scan.

use std::{fs, path::Path};

use crate::rng::{
    BattleSeed, InjuryRng, LootRng, ProcgenRng, ReactionRng, SeverityRng, ShotRng, streams::fnv1a64,
};

/// Number of consecutive draws compared when asserting stream equality or
/// independence.
const STREAM_LEN: usize = 64;

/// Pull `STREAM_LEN` consecutive `u64` draws from a fresh [`ShotRng`] seeded from
/// `root`.
fn shot_stream(root: u64) -> Vec<u64> {
    let mut rng = ShotRng::from_root(BattleSeed::new(root));
    (0..STREAM_LEN).map(|_| rng.next_u64()).collect()
}

/// Pull `STREAM_LEN` consecutive `u64` draws from a fresh [`SeverityRng`] seeded
/// from `root`.
fn severity_stream(root: u64) -> Vec<u64> {
    let mut rng = SeverityRng::from_root(BattleSeed::new(root));
    (0..STREAM_LEN).map(|_| rng.next_u64()).collect()
}

// ── Re-pointed determinism tests (formerly `same_seed_same_stream` etc.) ────

#[test]
fn same_seed_same_stream() {
    // The pinned determinism property: two ShotRngs built from the SAME root produce
    // IDENTICAL draw streams, value-for-value.
    let a = shot_stream(0xDEAD_BEEF);
    let b = shot_stream(0xDEAD_BEEF);
    assert_eq!(a, b);
}

#[test]
fn different_seed_different_stream() {
    // A different root seed yields a different stream (over 64 u64 draws a
    // full-stream collision is astronomically unlikely; equal here = seed not wired).
    let a = shot_stream(0xDEAD_BEEF);
    let c = shot_stream(0x1234_5678);
    assert_ne!(a, c);
}

#[test]
fn rng_handle_draws_from_the_same_seeded_stream() {
    // The `rng()` accessor must draw the SAME stream as `next_u64` — it is the same
    // underlying ChaCha12Rng, just borrowed as the trait. Draw via the handle on one
    // instance and via next_u64 on an identically-seeded instance; streams must match.
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
    // The seed newtype Derefs to its raw u64 (house style) and round-trips.
    let seed = BattleSeed::new(7);
    assert_eq!(*seed, 7u64);
}

// ── GTW-14 stream-independence tests ─────────────────────────────────────────

#[test]
fn stream_independence_shot_unaffected_by_severity_draws() {
    // C6 / GTW-14: a draw added to SeverityRng must NOT change ShotRng's output.
    let root = BattleSeed::new(0xC0_FFEE_1234);

    // Baseline: capture ShotRng's first STREAM_LEN draws with NO severity draws.
    let baseline = shot_stream(*root);

    // Perturbed: draw from SeverityRng first, then capture ShotRng's first STREAM_LEN.
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
    // ShotRng and SeverityRng from the same root must produce DIFFERENT sequences
    // (if they were identical, the streams would be correlated and useless as
    // independent draws). Over 64 values a collision is astronomically improbable;
    // equality here would mean the labels are identical (a bug).
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
    // All six stream types from the same root must be pairwise distinct.
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
    // Pairwise distinctness (15 pairs for 6 streams).
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

// ── GTW-14 pinned-output regression tests ────────────────────────────────────

/// The pinned first `u64` from `ShotRng::from_root(BattleSeed::new(0))`.
///
/// This literal was computed from the REAL `rand_chacha` build and committed as the
/// cross-build / cross-platform anchor for the full chain: FNV-1a-64 →
/// `seed_from_u64` (`SplitMix64`-style fill) → `ChaCha12` keystream. Any change to the
/// FNV constants, the label, the byte-order, or the cipher fails the
/// `pinned_shot_first_draw` test below loudly (the `assert_eq!` always runs). The
/// value is meaningful ONLY because the generator is the portable
/// `rand_chacha::ChaCha12Rng` (not the non-portable `StdRng`).
///
/// To recompute (after a deliberate re-tune that bumps the label):
/// `ShotRng::from_root(BattleSeed::new(0)).next_u64()`.
const PINNED_SHOT_FIRST_U64: u64 = 0xb6e6_9916_cc82_4771;

/// Anchor the full derivation chain to its exact pinned first draw.
///
/// The byte-stability of this value across builds and platforms is the property we
/// test — if it changes (same label, same FNV, same cipher), the
/// FNV → `seed_from_u64` → `ChaCha12` chain broke. `next_u64` cannot be `const`-eval'd
/// (the `ChaCha12` RNG is not `const`), so the value is computed at test time and
/// compared against the committed literal.
#[test]
fn pinned_shot_first_draw() {
    let actual = ShotRng::from_root(BattleSeed::new(0)).next_u64();

    // Regression: the full FNV → seed_from_u64 → ChaCha12 chain must reproduce the
    // exact pinned value across builds and platforms (portability of ChaCha12Rng).
    assert_eq!(
        actual, PINNED_SHOT_FIRST_U64,
        "ShotRng pinned first draw changed — the FNV derivation, label, or ChaCha12 \
         keystream was altered. If this is intentional (label bump), recompute the \
         constant and update PINNED_SHOT_FIRST_U64."
    );
}

#[test]
fn pinned_severity_first_draw() {
    // Same pattern as pinned_shot_first_draw — anchors SeverityRng independently.
    // Computed from the real `rand_chacha` build; the `assert_eq!` always runs so a
    // cipher/label/byte-order change fails this test loudly.
    const PINNED: u64 = 0x7d67_812c_25e8_fb3e;
    let actual = SeverityRng::from_root(BattleSeed::new(0)).next_u64();
    assert_eq!(actual, PINNED, "SeverityRng pinned first draw changed");
}

// ── FNV-1a-64 stability test ─────────────────────────────────────────────────

#[test]
fn fnv1a64_is_stable() {
    // Pin `fnv1a64(0u64, b"gdtf.rng.shot.v1")` to its exact literal.
    // This locks the derivation constants (offset/prime), the label, and the LE
    // byte-order. Any change to the algorithm or the ShotRng label fails this test.
    //
    // Hand-trace:
    //   root = 0u64 → to_le_bytes() = [0,0,0,0,0,0,0,0]
    //   label = b"gdtf.rng.shot.v1" (16 bytes)
    //   FNV offset basis = 0xcbf2_9ce4_8422_2325
    //   for each byte: hash = (hash ^ byte).wrapping_mul(0x0000_0100_0000_01b3)
    //
    // The expected value was computed offline from first principles.
    let computed = fnv1a64(0u64, b"gdtf.rng.shot.v1");
    // The pinned expected value for the ShotRng label, computed from first principles
    // (FNV-1a-64 over the 8 LE root bytes ++ the 16 label bytes). The `assert_eq!`
    // always runs so any change to the FNV constants, the LE byte-order, or the label
    // fails this test loudly.
    let expected: u64 = 0xb114_3619_01d0_9046;
    assert_eq!(
        computed, expected,
        "fnv1a64 output for (0, b\"gdtf.rng.shot.v1\") changed — the FNV constants \
         or label were altered"
    );
}

// ── Entropy scan (hardened for GTW-14) ───────────────────────────────────────

/// Collect every `.rs` source file under a directory, recursively.
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
    // C4/C7(b) + GTW-14: ZERO global/implicit RNG or time/env entropy anywhere in
    // gdtf_battle_sim. We scan every source file for the forbidden tokens.
    //
    // The tokens are BUILT FROM FRAGMENTS (and named so no literal appears in this
    // test's own code) so the test does NOT match its own source — and an honest
    // mention in a doc comment is filtered out (the comment-strip below).
    //
    // GTW-14 extends the list to cover SystemTime / UNIX_EPOCH / std::env / env::var
    // so the composition boundary (only gdtf_app reads time/env, never the sim) is
    // enforced by this scan.
    let token_a = ["thread", "rng"].join("_");
    let token_b = format!("Os{}", "Rng");
    let token_c = format!("rand::{}", "random");
    // GTW-14 additions — time/env sources must not appear in sim source.
    let token_d = format!("System{}", "Time");
    let token_e = format!("UNIX_{}", "EPOCH");
    let token_f = ["std::", "env"].join("");
    let token_g = ["env::", "var"].join("");
    // GTW-408 / M1 additions — rand 0.10's thread-local entropy accessors. In rand
    // 0.10 the thread RNG is `rand::rng()` (NOT the pre-0.10 `thread_rng()`), exposed
    // as `rngs::ThreadRng`, and the OS-entropy crate is `getrandom`. None may appear
    // in sim source. (Built from fragments — note lowercase `rand::rng` does NOT match
    // `rand::Rng`/`rand::RngExt`, the legitimate trait imports the streams use.)
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
    // Sanity: the scan actually found this crate's sources.
    assert!(!files.is_empty());

    for file in &files {
        let Ok(contents) = fs::read_to_string(file) else {
            continue;
        };
        for (line_no, line) in contents.lines().enumerate() {
            // Ignore doc/line comments: a comment that NAMES a forbidden verb (this
            // module documents "never a global/thread RNG") is not a USE of it. Strip
            // from the first `//` before scanning.
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
