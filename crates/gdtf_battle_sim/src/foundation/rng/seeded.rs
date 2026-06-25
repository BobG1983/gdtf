//! The seeded-RNG implementation — [`BattleSeed`] + the [`SimRng`] resource and its thin
//! draw surface. See the module docs (`super`) for the determinism contract.

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
