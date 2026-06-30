//! The §7 melee opposed-Fight tuning layer — the roll variance and the
//! margin → damage-multiplier curve coefficients (GTW-506).
//!
//! `docs/combat/resolution.md` §7 (lines 146-150) designs close combat as an
//! **opposed roll** whose relative margin scales the blow as a **multiplier**:
//!
//! ```text
//! atk = Fight_attacker × roll        (roll ∈ [1−v, 1+v], variance v = tunable)
//! def = Fight_defender × roll
//! connect if atk > def
//! margin      = atk / def − 1        (relative dominance — scale-independent, unbounded)
//! damage_mult = clamp(mult_min + k_margin × margin, mult_min, mult_max)
//! ```
//!
//! This dir-module holds the **data substrate** for that math:
//!
//! - [`leaves`] — the four tuning leaves ([`MeleeKMargin`] / [`MeleeMultMin`] /
//!   [`MeleeMultMax`] / [`FightVariance`]) and the [`MeleeTuning`] group that
//!   embeds in [`crate::tuning::CombatTuning`].
//!
//! The pure §7 functions that CONSUME these leaves
//! ([`opposed_fight`](crate::melee::opposed_fight) /
//! [`melee_damage_mult`](crate::melee::melee_damage_mult)) live in the
//! [`crate::melee`] combat-math module — they read [`MeleeTuning`] and draw from
//! the dedicated [`FightRng`](crate::rng::FightRng) stream. Everything is **pure
//! data + pure functions** — no systems, no `&mut World`, no ECS trigger. The
//! live melee ACT (adjacency / LOS gate / input / presenter) is GTW-507.

mod leaves;

pub use leaves::{FightVariance, MeleeKMargin, MeleeMultMax, MeleeMultMin, MeleeTuning};
