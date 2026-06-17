//! Relocated unit tests for the seeded RNG harness (GTW-201 wave 22 — moved verbatim
//! from the former inline `#[cfg(test)] mod tests`).

use std::{fs, path::Path};

use rand::RngExt;

use crate::rng::{BattleSeed, SimRng};

/// Number of consecutive draws compared when pinning a stream.
const STREAM_LEN: usize = 64;

/// Pull `STREAM_LEN` consecutive `u64` draws from a fresh `SimRng` seeded
/// with `seed`.
fn draw_stream(seed: u64) -> Vec<u64> {
    let mut rng = SimRng::from_seed(BattleSeed::new(seed));
    (0..STREAM_LEN).map(|_| rng.next_u64()).collect()
}

#[test]
fn same_seed_same_stream() {
    // The pinned determinism property (docs/testing.md): two SimRng built
    // from the SAME seed produce IDENTICAL draw streams, value-for-value.
    let a = draw_stream(0xDEAD_BEEF);
    let b = draw_stream(0xDEAD_BEEF);
    assert_eq!(a, b);
}

#[test]
fn different_seed_different_stream() {
    // The counter-property: a DIFFERENT seed yields a different stream.
    // (Over 64 u64 draws a full-stream collision is astronomically
    // unlikely; an equal stream here would mean the seed isn't wired in.)
    let a = draw_stream(0xDEAD_BEEF);
    let c = draw_stream(0x1234_5678);
    assert_ne!(a, c);
}

#[test]
fn rng_handle_draws_from_the_same_seeded_stream() {
    // The &mut impl Rng accessor must draw the SAME stream as the direct
    // draw methods — it is the same underlying RNG, just borrowed as the
    // trait. Draw via the handle on one instance and via next_u64 on an
    // identically-seeded instance; the streams match.
    let mut via_handle = SimRng::from_seed(BattleSeed::new(42));
    let mut via_method = SimRng::from_seed(BattleSeed::new(42));
    let handle = via_handle.rng();
    let from_handle: Vec<u64> = (0..STREAM_LEN).map(|_| handle.random()).collect();
    let from_method: Vec<u64> = (0..STREAM_LEN).map(|_| via_method.next_u64()).collect();
    assert_eq!(from_handle, from_method);
}

#[test]
fn battle_seed_derefs_to_inner() {
    // The seed newtype Derefs to its raw u64 (house style) and round-trips.
    let seed = BattleSeed::new(7);
    assert_eq!(*seed, 7u64);
}

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
    // C4/C7(b): ZERO global/implicit RNG anywhere in gdtf_battle_sim. We
    // scan every source file for the forbidden draw points. The forbidden
    // tokens are BUILT FROM FRAGMENTS (and the bindings are named so no
    // literal token appears in code) so this test does NOT match its own
    // source — and an honest mention in a doc comment is filtered out.
    let token_a = ["thread", "rng"].join("_");
    let token_b = format!("Os{}", "Rng");
    let token_c = format!("rand::{}", "random");
    let forbidden = [token_a.as_str(), token_b.as_str(), token_c.as_str()];

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
            // Ignore doc/line comments: a comment that NAMES a forbidden
            // verb (this module documents "never a global/thread RNG") is
            // not a USE of it. Strip from the first `//` before scanning.
            let code = line.split("//").next().unwrap_or_default();
            for token in &forbidden {
                assert!(
                    !code.contains(token),
                    "forbidden global/thread RNG token `{token}` in {}:{}",
                    file.display(),
                    line_no + 1,
                );
            }
        }
    }
}
