//! Per-subsystem RNG stream resources (GTW-14).
//!
//! Each stream is a distinct [`bevy::prelude::Resource`] newtype wrapping a private
//! [`rand_chacha::ChaCha12Rng`] — the portable, rand-family `ChaCha` cipher that rand's
//! own docs steer replay-safety to (unlike [`rand::rngs::StdRng`], which is documented
//! "non-portable" and "output may be platform-dependent"). Five streams
//! cover every current and near-future RNG draw site in the sim:
//!
//! | Resource | Draw sites |
//! |---|---|
//! | [`ShotRng`] | cone sample (§1 trajectory) + §4 body-part roll |
//! | [`SeverityRng`] | §6 roll term (the ONE draw per hit) |
//! | [`LootRng`] | reserved — loot generation (no draw sites yet) |
//! | [`InjuryRng`] | in-battle injury roll — one draw per non-graze, non-fatal wound (GTW-438) |
//! | [`ProcgenRng`] | reserved — procedural level generation (no draw sites yet) |
//!
//! ## Derivation — the crux
//!
//! Given a [`BattleSeed`] `root`, each stream's seed is:
//!
//! ```text
//! per_stream_u64 = fnv1a64( root.to_le_bytes() ++ LABEL_bytes )
//! stream_rng     = ChaCha12Rng::seed_from_u64(per_stream_u64)
//! ```
//!
//! FNV-1a-64 is hand-rolled as a `const fn` in the sibling `derivation` leaf (NOT
//! `std::hash::DefaultHasher`, whose
//! output is not stability-guaranteed across Rust releases). The `root` is always
//! exactly 8 leading little-endian bytes, and each label is a fixed versioned ASCII
//! constant — so the concatenation is unambiguous with no length prefix. The resulting
//! `u64` goes to [`SeedableRng::seed_from_u64`], `rand_core`'s default fixed
//! `SplitMix64`-style integer expansion that is deterministic and platform-stable (the
//! `u64` arithmetic is identical everywhere).
//! Finally, [`ChaCha12Rng`] is the portable, vector-pinned cipher — so the *entire*
//! root → stream-state → draw chain is byte-stable for cross-build/cross-platform
//! replay.
//!
//! **Independence guarantee:** each stream's seed = `fnv1a64(root_le ++ its_own_label)`,
//! a pure function of `(root_seed, that label)` and nothing else. No shared cursor,
//! no sequence-derivation. Adding a stream (or reordering them) cannot perturb any
//! other stream's seed — the addresses of the other labels are unchanged.
//!
//! **Stream label versioning:** labels carry a `.v1` suffix (e.g. `b"gdtf.rng.shot.v1"`)
//! so a deliberate re-tune of one stream's derivation bumps its label without
//! disturbing others. A version bump invalidates replays that depend on that stream —
//! intentionally, because the stream changed.
//!
//! ## Binding constraint
//!
//! No system may take `Res<ShotRng>` / `Res<SeverityRng>` (or any stream resource) for
//! read-only access. Drawing advances the `ChaCha12` cursor, so RNG access is
//! [`ResMut`](bevy::prelude::ResMut) by definition. A read-only borrow (`Res<T>`) would
//! needlessly serialize the borrowing system against every other system that also takes
//! `Res<T>`, and it cannot draw. A future system needing a reproducible RNG-derived value
//! should draw it into a non-`Resource` value; it must never reach for `Res<T>` here.
//!
//! ## Single source of truth
//!
//! The [`impl_sim_stream!`] macro is the **sole definition of every stream's draw
//! surface**. Adding a draw method (e.g. `fill_bytes`) means editing the macro — NOT
//! each stamped type individually. Each public stream type remains a separate concrete
//! `Resource` so `ResMut<ShotRng>` and `ResMut<SeverityRng>` are disjoint params and
//! the Bevy scheduler can run systems on different streams in parallel.

use rand::{SeedableRng, distr::uniform::SampleRange};
use rand_chacha::ChaCha12Rng;

use super::{
    derivation::{StreamLabel, fnv1a64},
    seeded::BattleSeed,
};

// ── The single draw-surface macro ────────────────────────────────────────────

/// Stamp a per-subsystem RNG stream [`Resource`](bevy::prelude::Resource) newtype.
///
/// This macro is the **single source of truth** for every stream's draw surface.
/// Adding a draw method (e.g. `fill_bytes`) means editing this macro — NOT each
/// stamped type individually. The macro generates, for one `$name` type with label
/// `$label` (a `b"…"` byte-string literal):
///
/// - A `#[derive(Resource)]` tuple-newtype wrapping a private `ChaCha12Rng`.
/// - A `const LABEL: StreamLabel` that names this stream in the derivation.
/// - `from_root(root: BattleSeed) -> Self` — the only constructor; derives the
///   stream seed via `fnv1a64(root, LABEL)` → `ChaCha12Rng::seed_from_u64`.
/// - `rng(&mut self) -> &mut impl rand::Rng` — borrow the inner cipher as the
///   `rand::Rng` trait (for the `sample_cone_vector` / `roll_body_part` leaves that
///   take `&mut impl Rng`). The concrete type never escapes.
/// - `random_range<T, R>(&mut self, range: R) -> T` — draw a value uniformly from
///   `range`. The actual sim draw sites: the §4 body-part roll (`roll_body_part` →
///   cumulative-weight `random_range`, NOT a `WeightedIndex`) and the §1 cone
///   sample (radius + azimuth, drawn via the `rng()` handle).
/// - `random_range_or_midpoint(&mut self, range: Range<f32>) -> f32` — the SAFE
///   draw for a TUNABLE-DRIVEN range (GTW-640 / GTW-644): never panics, ALWAYS
///   consumes exactly one draw, collapses a degenerate (`start >= end`) range to
///   the bounds' midpoint. The §6 `roll(lo..hi)` term and the §7 opposed-Fight
///   rolls draw through this.
/// - `next_u64(&mut self) -> u64` — draw the next raw `u64`. Test/regression-anchor
///   only (the pinned-output tests + leaf draw-determinism tests); no production
///   draw site uses it, but the integration test crates do, so it stays public.
///
/// # Binding constraint
///
/// No system may take `Res<$name>` for read-only access (drawing advances the cursor,
/// so RNG access is `ResMut` by definition). See the module doc.
macro_rules! impl_sim_stream {
    ($(#[$attr:meta])* $name:ident, $label:expr) => {
        $(#[$attr])*
        #[derive(bevy::prelude::Resource)]
        pub struct $name(ChaCha12Rng);

        impl $name {
            /// The stable versioned label identifying this stream in the FNV-1a-64
            /// root-seed derivation (GTW-14, §A). Distinct from every other stream's
            /// label, so adding or reordering streams cannot perturb this seed.
            pub const LABEL: StreamLabel = StreamLabel::new($label);

            /// Derive this stream from a [`BattleSeed`] root.
            ///
            /// Applies `fnv1a64(root, Self::LABEL)` → `ChaCha12Rng::seed_from_u64` —
            /// the portable, IETF-pinned derivation. Two calls with the same `root`
            /// yield identical initial states; calls with different roots yield different
            /// states (astronomically unlikely collision over a 64-bit output). The only
            /// legal constructor: there is no entropy-seeded path by design (the sim must
            /// stay replay-pure).
            #[must_use]
            pub fn from_root(root: BattleSeed) -> Self {
                let seed = fnv1a64(root, Self::LABEL.as_bytes());
                Self(ChaCha12Rng::seed_from_u64(seed))
            }

            /// Borrow the inner [`ChaCha12Rng`] as `&mut impl rand::Rng`.
            ///
            /// The bridge to combat-math leaves that take `rng: &mut impl rand::Rng`
            /// (e.g. `sample_cone_vector` / `roll_body_part`). The concrete cipher type
            /// never escapes — callers draw through the trait, not the inner type.
            pub fn rng(&mut self) -> &mut impl rand::Rng {
                &mut self.0
            }

            /// Draw the next raw `u64` from the stream.
            ///
            /// Test/regression-anchor draw: the pinned-output tests that anchor the
            /// entire derivation chain, plus the leaf draw-determinism tests, draw one
            /// uniform `u64` through this. No production draw site uses it — but the
            /// integration-test crates (a separate compilation unit that links the lib
            /// built WITHOUT `--cfg test`) do, so it must stay public rather than
            /// `#[cfg(test)]`.
            pub fn next_u64(&mut self) -> u64 {
                use rand::RngExt as _;
                self.0.random()
            }

            /// Draw a value uniformly from `range`.
            ///
            /// The `rand` 0.10 rename of `gen_range` — for draw sites whose range is
            /// non-empty BY CONSTRUCTION (it panics on an empty range). A range whose
            /// bounds derive from tunable/authored data must draw through
            /// [`random_range_or_midpoint`](Self::random_range_or_midpoint) instead.
            pub fn random_range<T, R>(&mut self, range: R) -> T
            where
                T: rand::distr::uniform::SampleUniform,
                R: SampleRange<T>,
            {
                use rand::RngExt as _;
                self.0.random_range(range)
            }

            /// Draw uniformly from a TUNABLE-DRIVEN `f32` range — NEVER panicking and
            /// ALWAYS consuming exactly one draw, even when the range is empty or
            /// inverted (GTW-640 / GTW-644).
            ///
            /// The safe-draw verb for any range whose bounds derive from tunable /
            /// authored / hot-reloadable data — which makes the bounds attacker-controlled
            /// at runtime: a documented-legal tuning edit (e.g. a fight variance of `0.0`)
            /// can degenerate such a range mid-session. A LIVE range (`start < end`) draws
            /// byte-identically to [`random_range`](Self::random_range); a DEGENERATE
            /// range (`start >= end`) still consumes exactly one uniform draw and returns
            /// the bounds' MIDPOINT (so the §7 band `[1 − v, 1 + v]` with `v <= 0.0`
            /// collapses to factor `1.0`).
            ///
            /// **Draw-count stability is the documented invariant, not an optimization**:
            /// one call = one draw, unconditionally, so the stream cursor advances
            /// identically whether or not the degenerate case was hit — seeded replays
            /// stay aligned across every downstream draw. (GTW-644's defect was a guard
            /// that SKIPPED the degenerate draw and silently sheared the stream.)
            pub fn random_range_or_midpoint(&mut self, range: core::ops::Range<f32>) -> f32 {
                super::safe_draw::uniform_or_midpoint(&mut self.0, range)
            }
        }
    };
}

impl_sim_stream!(
    /// The **shot-resolution** RNG stream.
    ///
    /// Draw sites: the §1 in-cone trajectory sample (`sample_cone_vector` — radius +
    /// azimuth) and the §4 body-part roll (`roll_body_part`). Routed to
    /// `resolve_coarse` and `outcome_from_march` via `ResMut<ShotRng>` in
    /// `dispatch_fire` and threaded through `fire` → `resolve_round` → `resolve_coarse`
    /// (GTW-14, §D).
    ///
    /// No system may take `Res<ShotRng>` — see the module binding constraint.
    ShotRng,
    b"gdtf.rng.shot.v1"
);

impl_sim_stream!(
    /// The **wound-severity** RNG stream.
    ///
    /// Draw sites: the §6 roll term — one draw per ganger hit. Routed to
    /// `resolve_and_apply` → `fold_ganger` → `roll_severity` → `roll_term` via
    /// `ResMut<SeverityRng>` in `dispatch_fire` and threaded through `fire` →
    /// `resolve_round` → `resolve_and_apply` (GTW-14, §D).
    ///
    /// No system may take `Res<SeverityRng>` — see the module binding constraint.
    SeverityRng,
    b"gdtf.rng.severity.v1"
);

impl_sim_stream!(
    /// The **loot-generation** RNG stream (reserved — no draw sites yet).
    ///
    /// Inserted at battle setup alongside the other streams so its label and initial
    /// state are fixed at GTW-14 boundary. A future first draw on this stream will
    /// replay correctly with no migration (the seed is already pinned).
    ///
    /// No system may take `Res<LootRng>` — see the module binding constraint.
    LootRng,
    b"gdtf.rng.loot.v1"
);

impl_sim_stream!(
    /// The **in-battle injury-roll** RNG stream.
    ///
    /// Draw site: the `fold_ganger` damage fold calls `roll_injury` to pick a named
    /// condition from the weighted `(category, severity)` table once per non-graze,
    /// non-fatal wound (GTW-437 / GTW-438). The stream is labelled `injury.v1` and
    /// seeded at battle setup alongside the other streams so its cursor is pinned from
    /// frame 0 — adding or reordering other streams cannot perturb its seed (the
    /// independence guarantee in the module doc).
    ///
    /// No system may take `Res<InjuryRng>` — see the module binding constraint.
    InjuryRng,
    b"gdtf.rng.injury.v1"
);

impl_sim_stream!(
    /// The **procedural level generation** RNG stream (reserved — no draw sites yet).
    ///
    /// Inserted at battle setup alongside the other streams so its label and initial
    /// state are fixed at GTW-14 boundary.
    ///
    /// No system may take `Res<ProcgenRng>` — see the module binding constraint.
    ProcgenRng,
    b"gdtf.rng.procgen.v1"
);

impl_sim_stream!(
    /// The **reaction-fire** RNG stream (GTW-466 data substrate — reserved).
    ///
    /// Inserted at battle setup alongside the other streams so its label and initial
    /// state are pinned from the GTW-466 boundary. No draw sites exist yet — the
    /// opposed-check core is GTW-467, the live trigger GTW-468. Adding this stream
    /// does NOT perturb any existing stream's seed (each depends only on its own
    /// label via the FNV-1a-64 derivation).
    ///
    /// No system may take `Res<ReactionRng>` — see the module binding constraint.
    ReactionRng,
    b"gdtf.rng.reaction.v1"
);

impl_sim_stream!(
    /// The **melee opposed-Fight** RNG stream (GTW-506 data substrate).
    ///
    /// Draw site: the §7 opposed-Fight roll
    /// ([`opposed_fight`](crate::melee::opposed_fight)) takes TWO uniform draws per
    /// resolve — the attacker's roll then the defender's roll, in that deterministic
    /// order. Inserted at battle setup alongside the other streams so its cursor is
    /// pinned from the GTW-506 boundary. No ECS system draws from it yet — the live
    /// melee ACT that owns the `ResMut<FightRng>` is GTW-507.
    ///
    /// **Determinism isolation (the binding orchestrator decision):** melee uses its
    /// OWN stream, NOT [`ReactionRng`]. Sharing a stream would entangle melee and
    /// reaction-fire determinism — a melee draw would shift the reaction stream's
    /// cursor and vice versa. Per-concern stream isolation is the established pattern
    /// (`ShotRng` / `SeverityRng` / `InjuryRng` / `ReactionRng` each independent).
    /// Adding this stream does NOT perturb any existing stream's seed (each depends
    /// only on its own label via the FNV-1a-64 derivation).
    ///
    /// No system may take `Res<FightRng>` — see the module binding constraint.
    FightRng,
    b"gdtf.rng.fight.v1"
);
