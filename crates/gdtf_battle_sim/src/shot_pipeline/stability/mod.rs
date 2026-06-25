//! The §1a **stability layer** — a continuous 0–100 stability score and its
//! two-curve read (`docs/combat/resolution.md` §1a + "What's pure math vs sim"
//! line 148: `stability(stance, brace, emplacement, …) → (cone_mult,
//! recoil_growth)`).
//!
//! Stability is a **continuous score derived from the situation**, not a binary
//! state (resolution.md §1a). [`stability`] sums three contributions — the
//! **per-stance** contribution (prone 40 / kneel 25 / stand 10, read off
//! [`crate::tuning::StanceStability`]); the **automatic brace** contribution
//! (+30, [`crate::tuning::BraceContribution`]), applied when the faced cell's
//! [`crate::cover::CoverEntry::height_band`] satisfies the per-stance brace
//! min-height gate ([`crate::tuning::BraceMinHeight`]: prone↔LOW+, kneel↔MID+,
//! stand↔HIGH) **OR** the weapon carries the [`crate::weapon::Stable`] tag (a
//! stable weapon engages the brace UNCONDITIONALLY — bipod-mounted /
//! braced-by-design — regardless of faced cover or stance); and the
//! **emplacement** seam ([`EmplacementStability`] — no entities yet, a
//! zero/identity term carried so the signature is complete) — then
//! **clamps/normalises** the sum into the `0..=100` domain ([`StabilityScore`])
//! and reads **both** [`crate::tuning::StabilityCurves`] at that score: the
//! cone-mult curve ([`ConeMult`], steadier → narrower, < 1) and the
//! recoil-growth curve ([`RecoilGrowth`], steadier → climbs strictly less).
//!
//! There is **no** weapon-intrinsic stability *points* term: a weapon's only
//! contribution to the score is the boolean [`crate::weapon::Stable`] tag, which
//! engages the brace contribution unconditionally (the user-corrected model —
//! weapons carry no intrinsic stability points).
//!
//! The curve *form* (a clamped, piecewise-linear read over the authored sample
//! points) lives here in code; every *coefficient* — the contributions, the
//! gate, and both curves — comes from [`crate::tuning::CombatTuning`], so no
//! stability magnitude or band constant is hardcoded
//! (resolution.md §"Coefficients live in the combat-tuning data"). The whole
//! layer is **angular / dimensionless — zero pixels**: it never touches the
//! cubic-voxel metric, only the abstract score and the cover
//! [`crate::cover::HeightBand`].
//!
//! The brace gate compares the faced cell's [`crate::cover::HeightBand`]
//! **directly** against the per-stance minimum band from tuning —
//! [`crate::cover::band_for`] is NOT called here (that helper maps a within-level
//! [`crate::cover::BandFraction`] to a [`crate::cover::HeightBand`] for the
//! projectile clearance path, a different concern).

mod curve;
mod gate;
mod score;
mod types;

#[cfg(test)]
mod test;

pub use score::stability;
pub use types::{ConeMult, EmplacementStability, RecoilGrowth, StabilityScore};
