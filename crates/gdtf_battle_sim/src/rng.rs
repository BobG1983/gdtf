//! Injected seeded-RNG harness: the model's single, deterministic draw point.
//!
//! `docs/combat/resolution.md` is unambiguous — the resolution math is
//! **deterministic** and "**every draw comes from the model-owned seeded RNG,
//! injected once at setup**". `docs/testing.md` pins the mechanism: anything
//! random takes an RNG by parameter (`&mut impl rand::Rng`, or
//! `StdRng::seed_from_u64(seed)`), **never a global/thread RNG**, and
//! *same-seed-same-stream is itself a pinned property*.
//!
//! [`SimRng`] is the one legal home for that RNG inside the sim. It is a Bevy
//! [`Resource`] wrapping a [`StdRng`](rand::rngs::StdRng) in a **private** field
//! — the concrete RNG type never escapes the boundary (no public field, no
//! accessor that names or returns it). Downstream combat-math systems take
//! `ResMut<SimRng>` and draw through its thin surface: the convenience draw
//! methods ([`SimRng::next_u64`] / [`SimRng::random_range`] /
//! [`SimRng::sample`]) for direct draws, or [`SimRng::rng`] — an
//! `&mut impl rand::Rng` handle — to feed the `fn(.., rng: &mut impl Rng)`
//! combat-math signatures that `docs/testing.md` prescribes. Either way the
//! draw bottoms out in this one seeded stream.
//!
//! The seed is supplied by the composition root via [`SimRng::from_seed`]; the
//! [`BattleSeed`] newtype carries it (no bare `u64` crosses the boundary —
//! `docs/combat/resolution.md`'s `battle_seed` / `GDTF_BATTLE_SEED` is the
//! eventual source). Wiring the *actual* insertion into the running app, and
//! GTW-14's roster-side seed source, are out of scope for this slice (E1.8).

use bevy::prelude::Resource;
use rand::{
    Rng, RngExt, SeedableRng,
    distr::{Distribution, uniform::SampleRange},
    rngs::StdRng,
};

/// The u64 seed that fixes a battle's entire RNG stream — a deterministic
/// replay handle.
///
/// `docs/combat/resolution.md`: a battle's RNG is "randomized per battle by
/// default; a `battle_seed` / `GDTF_BATTLE_SEED` env pins a replay". This is
/// that value, wrapped so a bare `u64` never crosses an API boundary
/// (`.claude/rules/no-bare-types.md`). The same seed *always* reproduces the
/// same draw stream (the pinned property — see the tests). GTW-14's roster-side
/// `BattleSeed` source is the eventual producer; this slice only consumes a
/// seed value, so the composition root may build one directly.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BattleSeed(u64);

impl BattleSeed {
    /// Build a battle seed from its raw `u64`.
    ///
    /// The one constructor for the seed identity, keeping the inner `u64`
    /// private so a seed can never be confused with another `u64` domain value.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }
}

/// The model-owned seeded RNG — the **only** legal source of randomness in the
/// sim.
///
/// A Bevy [`Resource`] so combat systems reach it as `ResMut<SimRng>` and every
/// draw funnels through this single, deterministic stream (injected once at
/// setup — `docs/combat/resolution.md`). The inner [`StdRng`](rand::rngs::StdRng)
/// is **private**: the concrete RNG type is an implementation detail that never
/// escapes — callers draw through the thin surface below, never by touching the
/// field. Constructed from a [`BattleSeed`] via
/// [`seed_from_u64`](rand::SeedableRng::seed_from_u64), so the same seed yields
/// an identical stream (the pinned determinism property).
#[derive(Resource)]
pub struct SimRng(StdRng);

impl SimRng {
    /// Construct the sim RNG from a [`BattleSeed`].
    ///
    /// Seeds a [`StdRng`](rand::rngs::StdRng) via
    /// [`seed_from_u64`](rand::SeedableRng::seed_from_u64) — a deterministic,
    /// entropy-free expansion of the `u64` seed into the RNG's full state, so
    /// two `SimRng`s built from the same [`BattleSeed`] produce identical draw
    /// streams. There is no entropy-seeded constructor by design: the sim must
    /// stay replayable, so the seed always comes from the composition root.
    #[must_use]
    pub fn from_seed(seed: BattleSeed) -> Self {
        Self(StdRng::seed_from_u64(*seed))
    }

    /// Borrow the inner RNG as an `&mut impl rand::Rng`.
    ///
    /// The bridge to `docs/testing.md`'s prescribed combat-math shape — those
    /// functions take `rng: &mut impl rand::Rng` and draw through it, so they
    /// receive *this* handle and their draws come from the one seeded stream.
    /// The return type is the [`Rng`](rand::Rng) **trait**, never the concrete
    /// `StdRng`, so the implementation type still does not escape the boundary.
    pub fn rng(&mut self) -> &mut impl Rng {
        &mut self.0
    }

    /// Draw the next raw `u64` from the stream.
    ///
    /// A thin convenience over [`random`](rand::RngExt::random) for callers that
    /// just need one uniform `u64` (e.g. forking a sub-seed, or the determinism
    /// tests below). Not named `gen` — that is a reserved keyword in Rust
    /// edition 2024 — and `rand` 0.10 renamed the draw verbs to `random*`
    /// anyway. Returns the bare `u64` only because it is the RNG's own output
    /// width, not a domain value (the caller wraps it into whatever it means).
    pub fn next_u64(&mut self) -> u64 {
        self.0.random()
    }

    /// Draw a value uniformly from `range`.
    ///
    /// Forwards to [`random_range`](rand::RngExt::random_range) (the `rand` 0.10
    /// rename of the old `gen_range`) so callers that need a bounded sample —
    /// e.g. `roll(0..R_eff)` in the severity formula — draw it from the one
    /// seeded stream without naming the concrete RNG.
    pub fn random_range<T, R>(&mut self, range: R) -> T
    where
        T: rand::distr::uniform::SampleUniform,
        R: SampleRange<T>,
    {
        self.0.random_range(range)
    }

    /// Draw a value from an arbitrary [`Distribution`](rand::distr::Distribution).
    ///
    /// Forwards to [`sample`](rand::RngExt::sample) so a weighted draw (the
    /// body-part roll, the cone radius power-law) can be expressed as a
    /// distribution and still bottom out in the single seeded stream.
    pub fn sample<T, D: Distribution<T>>(&mut self, distr: D) -> T {
        self.0.sample(distr)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;

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
}
