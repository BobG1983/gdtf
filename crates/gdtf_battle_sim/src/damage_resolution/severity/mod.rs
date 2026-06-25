//! The §6 wound-severity roll — the score → [`Severity`] bucket, in the
//! **floor-extend** form (resolution.md §6, resolved 2026-06-15).
//!
//! This is the E3.4 slice. **Every hit rolls severity** ([`roll_severity`]) — the
//! old `damage > Toughness` hard gate is gone (resolution.md §6). The score is
//! gated by **penetrating damage** ([`crate::resolve_hit::PenetratingDamage`], the
//! pre-floor `max(0, inner)` of the per-hit formula), so a weak hit cannot reach
//! the severe buckets — no 1-damage amputations. The score is:
//!
//! ```text
//! severity_score = j × pen_damage − k × Toughness + part_mod + fatal_bias
//!                  + I × Luck_shooter + roll(−L × Luck_defender .. R)
//! ```
//!
//! where the random term is a uniform draw over `[−L × Luck_defender, R]`: the
//! defender's Luck extends the roll's **floor** down (a chance to shrug the hit
//! off — it can pull a would-be Major down to Minor or None) while the **ceiling
//! stays `R`**, so a genuinely bad roll is always still possible and variance
//! grows with the defender's Luck. There is **no** `R_eff` / `R_min` — the lower
//! bound *moves*, the spread is not merely shrunk. The shooter's Luck pushes the
//! score up (nastier wounds). The score is bucketed by the ascending §6 edges
//! `e0..e3` ([`crate::tuning::SeverityEdges`]): `< e0 → None`, `< e1 → Minor`,
//! `< e2 → Major`, `< e3 → Critical`, `≥ e3 → Fatal`.
//!
//! Pure, render-free sim math: it reads its coefficients from
//! [`crate::tuning::SeverityScaling`], draws from the injected seeded
//! [`crate::rng::SeverityRng`], and carries **no pixel**. The defender's Toughness and
//! both gangers' Luck are the E3.0 ganger attribute components
//! ([`crate::ganger::Toughness`] / [`crate::ganger::Luck`]) — sourced off the
//! entity, not bare literals. The per-part severity mod is a **placeholder** code
//! const ([`part_severity_mod`]), distinct from the §4 hit-location
//! [`crate::tuning::BodyPartWeights`] (this is severity *escalation*, not
//! hit-likelihood).

mod kind;
mod roll;

pub use kind::{PartSeverityMod, Severity, part_severity_mod};
pub use roll::{SeverityInputs, roll_severity};

#[cfg(test)]
mod test;
