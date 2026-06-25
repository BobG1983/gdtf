//! The §4 **weighted hit-location roll** — `roll_body_part`.
//!
//! When the coarse march stops a round on a ganger (`docs/combat/resolution.md`
//! §2), this is the entire "where on them" decision: a **weighted pick** over the
//! six [`BodyPart`](crate::armor::BodyPart)s (Head · Torso · L-Arm · R-Arm · L-Leg ·
//! R-Leg) by the existing tuning
//! [`BodyPartWeights`](crate::tuning::BodyPartWeights) (resolution.md §4 + "What's
//! pure math vs sim" line 152: `roll_body_part(body_part_weights, rng)`). There is
//! **no per-part geometry**: the coarse model already decided *which* ganger by the
//! march's band clearance, and *where* on them is purely this chance roll — the
//! prior on-silhouette / exposure-area model is **retired** (resolution.md §4
//! "RETIRED — both prior models").
//!
//! Every draw bottoms out in the injected [`crate::rng::ShotRng`] via the
//! `&mut impl rand::Rng` handle (`docs/testing.md`: "anything random takes an RNG
//! by parameter … never a global/thread RNG"), so the roll is deterministic and
//! seed-replayable. The roll reads **only** the passed
//! [`BodyPartWeights`](crate::tuning::BodyPartWeights) — no weight is hardcoded
//! here; the magnitudes (and even the per-part order of their defaults) are tunable
//! balance data (resolution.md §"Coefficients live in the combat-tuning data"). The
//! struck part is the §4 **location** input the E3 Injury table and the severity
//! part-mod consume.

mod roll;
#[cfg(test)]
mod test;

pub use roll::roll_body_part;
