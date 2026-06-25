//! The E4.3 **stability / cone composers** — the two shared methods that compute
//! a shooter's stability read and dispersion-cone WIDTH off ganger state + the
//! MODEL cover ledger (`docs/combat/resolution.md` §1a + §"What's pure math vs
//! sim": "`cone_for` / `stability_for` compose `θ_cone` off ganger state + model
//! cover (the HUD stability readout reads the same method)").
//!
//! These are the **single callable surface** the HUD stability readout (GTW-11 /
//! E6) reads and the E4.5 `fire()` burst loop reuses per round — defined once
//! here so the two never re-derive the math. They **wrap** the landed E2 pipeline,
//! composing nothing new:
//!
//! - [`stability_for`] finds the faced cell via E4.2
//!   [`crate::faced_cell::faced_cell`], peeks the model [`crate::cover::CoverLedger`]
//!   there for the faced [`crate::cover::CoverEntry`] (`None` when no cover is
//!   faced), and calls the landed [`crate::stability::stability`] verbatim — the
//!   §1a brace gate engages exactly when the faced cover's
//!   [`crate::cover::HeightBand`] satisfies the per-stance gate (already inside
//!   `stability`).
//! - [`cone_for`] composes the full §1a chain by calling [`stability_for`] for
//!   the `(cone_mult, recoil_growth)` pair, [`crate::cone::aim_cone_mult`] for the
//!   aim term, then [`crate::cone::cone_angle`] over the five factors verbatim.
//!
//! The composers READ the model cover ledger via
//! [`CoverLedger::peek`](crate::cover::CoverLedger::peek) — it is never rebuilt
//! (the change-driven contract). Angular / dimensionless — **zero pixels**: every
//! output is the abstract stability multiplier or the angular
//! [`crate::cone::ConeAngle`], never a pixel.
//!
//! ## Weapon stability — the `stable` tag
//!
//! A weapon's contribution to the §1a stability score is the boolean
//! [`crate::weapon::Stable`] tag (the user-corrected model: weapons carry **no**
//! intrinsic stability *points*). A `stable` weapon engages the brace contribution
//! UNCONDITIONALLY (regardless of faced cover or stance). [`cone_for`] sources the
//! tag off the [`crate::weapon::WeaponStats`] borrow-view it receives (GTW-200: the
//! weapon is ECS components, read through a transient view, not a packed struct) and
//! threads it to [`stability_for`]; [`stability_for`] (which receives no weapon)
//! takes the [`crate::weapon::Stable`] tag as an explicit param and passes it to
//! [`crate::stability::stability`]. There is no longer a caller-supplied
//! weapon-points value — the gap the E4.3 slice noted is now closed by the tag.

mod compose;
mod shooter;

#[cfg(test)]
mod test;

pub use compose::{cone_for, stability_for};
pub use shooter::Shooter;
