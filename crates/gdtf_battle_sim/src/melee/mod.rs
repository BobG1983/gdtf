//! The §7 melee opposed-Fight combat-math **core** (GTW-506).
//!
//! `docs/combat/resolution.md` §7 (lines 146-150) designs close combat as an
//! **opposed roll** whose relative margin scales the blow as a **multiplier**
//! (never a flat add):
//!
//! ```text
//! atk = Fight_attacker × roll        (roll ∈ [1−v, 1+v], variance v = tunable)
//! def = Fight_defender × roll
//! connect if atk > def
//! margin      = atk / def − 1        (relative dominance — scale-independent, unbounded)
//! damage_mult = clamp(mult_min + k_margin × margin, mult_min, mult_max)
//! ```
//!
//! This module is the deterministic, render-free CORE of melee — **pure functions
//! over plain data + injected seeded RNG**, no systems, no `&mut World`, no ECS
//! trigger:
//!
//! - [`opposed_fight`] — the opposed roll (two [`FightRng`](crate::rng::FightRng)
//!   draws, attacker then defender) → a [`FightOutcome`] (`connect` + [`FightMargin`]).
//! - [`melee_damage_mult`] — the margin → [`MeleeDamageMult`] curve, clamped to
//!   `[mult_min, mult_max]`.
//! - [`apply_melee_multiplier`] — the PURE seam that scales a resolved
//!   [`HitResult`](crate::resolve_hit::HitResult) by the multiplier, between
//!   [`resolve_hit`](crate::resolve_hit::resolve_hit) and the §6 wound step.
//!
//! The variance + curve coefficients live in [`MeleeTuning`](crate::tuning::MeleeTuning);
//! the dedicated [`FightRng`](crate::rng::FightRng) stream isolates melee
//! determinism from reaction fire. The live melee ACT (adjacency / LOS gate /
//! input / presenter) is GTW-507 — it owns the `ResMut<FightRng>` and composes
//! `opposed_fight → resolve_hit → apply_melee_multiplier → §6 wound`.

mod fight;

#[cfg(test)]
mod tests;

pub use fight::{
    FightMargin, FightOutcome, MeleeDamageMult, apply_melee_multiplier, melee_damage_mult,
    opposed_fight,
};
