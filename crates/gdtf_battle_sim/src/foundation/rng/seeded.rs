//! The [`BattleSeed`] newtype — the root seed that fixes a battle's entire RNG
//! derivation. The per-subsystem stream resources ([`super::streams`]) are derived
//! from this value via `fnv1a64(root, label)` → `ChaCha12Rng::seed_from_u64`.
//!
//! `SimRng` has been DELETED in GTW-14: the single shared stream is replaced by five
//! independent per-subsystem stream resources ([`ShotRng`](super::ShotRng),
//! [`SeverityRng`](super::SeverityRng), [`LootRng`](super::LootRng),
//! [`InjuryRng`](super::InjuryRng), [`ProcgenRng`](super::ProcgenRng)) derived from
//! this seed. See [`super::streams`] for the derivation and draw surfaces.

/// The `u64` root seed that fixes a battle's entire RNG derivation — a deterministic
/// replay handle.
///
/// `docs/combat/resolution.md`: a battle's RNG is "randomized per battle by default;
/// a `battle_seed` / `GDTF_BATTLE_SEED` env pins a replay". This is that value,
/// wrapped so a bare `u64` never crosses an API boundary
/// (`.claude/rules/no-bare-types.md`). Each per-subsystem stream ([`ShotRng`] /
/// [`SeverityRng`] / …) is derived from this root via `fnv1a64(root, label)` →
/// `ChaCha12Rng::seed_from_u64`; the same root always reproduces the same per-stream
/// draw sequences (the pinned replay property — see the tests). The composition root
/// (`gdtf_app`) produces one via [`BattleSeed::new`] from either the
/// `GDTF_BATTLE_SEED` environment variable or a wall-clock–derived `u64` (GTW-14).
///
/// [`ShotRng`]: super::ShotRng
/// [`SeverityRng`]: super::SeverityRng
///
/// This type also derives [`bevy::prelude::Resource`] so a test harness (or a future
/// "seed pinning" UI feature) can pre-insert it into the [`bevy::app::App`] world;
/// `request_battle_setup` reads it as `Option<Res<BattleSeed>>` and uses it when
/// present, falling back to `resolve_root_seed()` (wall-clock / env var) otherwise.
/// The production GUI path inserts NO `BattleSeed` resource, so the normal entropy
/// path is taken; the headless test path inserts one to guarantee replay determinism.
#[derive(
    bevy::prelude::Resource, bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default,
)]
pub struct BattleSeed(u64);

impl BattleSeed {
    /// Build a battle seed from its raw `u64`.
    ///
    /// The one constructor for the seed identity, keeping the inner `u64` private so a
    /// seed can never be confused with another `u64` domain value.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The raw `u64` root — the `const`-context accessor the [`fnv1a64`] derivation reads
    /// (the derived [`Deref`](bevy::prelude::Deref) is not usable in a `const fn`).
    ///
    /// [`fnv1a64`]: super::derivation::fnv1a64
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for BattleSeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
