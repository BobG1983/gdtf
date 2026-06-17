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
//! [`Resource`](bevy::prelude::Resource) wrapping a [`StdRng`](rand::rngs::StdRng)
//! in a **private** field — the concrete RNG type never escapes the boundary (no
//! public field, no accessor that names or returns it). Downstream combat-math
//! systems take `ResMut<SimRng>` and draw through its thin surface: the convenience
//! draw methods ([`SimRng::next_u64`] / [`SimRng::random_range`] /
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

mod seeded;
#[cfg(test)]
mod test;

pub use seeded::{BattleSeed, SimRng};
