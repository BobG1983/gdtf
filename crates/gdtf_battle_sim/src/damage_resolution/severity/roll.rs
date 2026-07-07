//! The §6 score math + the roll — [`SeverityInputs`], the seeded random term, the
//! score formula, and the [`roll_severity`] entry point.

use super::kind::{PartSeverityMod, Severity, bucket};
use crate::{
    ganger::{Luck, Toughness},
    resolve_hit::PenetratingDamage,
    rng::SeverityRng,
    tuning::SeverityScaling,
    weapon::FatalBias,
};

/// The six per-roll inputs to the §6 severity score, bundled into one named
/// struct (resolution.md §6).
///
/// Groups the score's logical input set so [`roll_severity`] stays under clippy's
/// argument-count gate (the same precedent as `ShotInputs` / `DamageProfile`
/// — the tuning `&SeverityScaling` and the entropy `&mut SeverityRng` stay their own
/// params). Every field is a named domain type sourced off the resolved hit and
/// the two gangers' entity components — none is a bare literal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeverityInputs {
    /// The hit's penetrating damage — gates the score (E3.3, pre-floor).
    pub pen_damage:    PenetratingDamage,
    /// The defender's Toughness — its mitigation term (off the entity, E3.0).
    pub toughness:     Toughness,
    /// The struck part's severity modifier ([`part_severity_mod`](super::kind::part_severity_mod)).
    pub part_mod:      PartSeverityMod,
    /// The weapon's Fatal-bias — stacks the table toward nastier buckets.
    pub fatal_bias:    FatalBias,
    /// The **shooter's** Luck — pushes the score up (off the entity, E3.0).
    pub luck_shooter:  Luck,
    /// The **defender's** Luck — extends the roll's floor down (off the entity).
    pub luck_defender: Luck,
}

impl SeverityInputs {
    /// Build the severity-score input bundle from its six per-roll inputs.
    ///
    /// The constructor for the bundle — the call site assembles it from the
    /// resolved hit ([`PenetratingDamage`]), the struck part's mod, the weapon's
    /// [`FatalBias`], and the gangers' [`Toughness`] / [`Luck`] read off their
    /// entity components.
    #[must_use]
    pub const fn new(
        pen_damage: PenetratingDamage,
        toughness: Toughness,
        part_mod: PartSeverityMod,
        fatal_bias: FatalBias,
        luck_shooter: Luck,
        luck_defender: Luck,
    ) -> Self {
        Self {
            pen_damage,
            toughness,
            part_mod,
            fatal_bias,
            luck_shooter,
            luck_defender,
        }
    }
}

/// Cast a non-negative count-like `i32` to `f32` for the severity score's
/// `f32` arithmetic, clamped so a wild value cannot lose its sign.
///
/// [`PenetratingDamage`] is an `i32` (the per-hit formula is signed) but reaches
/// the §6 score as `j × pen` in `f32`. Penetrating damage is `max(0, inner)`, so
/// it is non-negative and far inside `f32`'s exact-integer range; the localized
/// `#[expect]` is the crate's guarded-cast idiom (`metric::floor_to_i32` /
/// `resolve_hit::round_to_i32`), so no `unwrap`/`expect` is needed.
#[expect(
    clippy::cast_precision_loss,
    reason = "penetrating damage is a small non-negative count, far inside f32's exact-integer range"
)]
const fn pen_to_f32(pen: i32) -> f32 {
    pen as f32
}

/// Draw the §6 random roll term — a **uniform** `f32` over the floor-extend range
/// `[−L × Luck_defender, R]` (resolution.md §6).
///
/// The lower bound is `−defender_luck_scale × Luck_defender` (the defender's Luck
/// extends the floor below 0); the upper bound is the fixed `random_spread` (`R`).
/// Both bounds are TUNABLE-DRIVEN (hot-reloadable tuning × per-defender Luck), so
/// the roll takes the stream's safe-draw verb
/// ([`SeverityRng::random_range_or_midpoint`](crate::rng::SeverityRng)): a
/// DEGENERATE range (`lo >= hi` — e.g. an `R ≤ 0` tuning, or a negative Luck)
/// never panics and STILL consumes exactly one draw (draw-count stability,
/// GTW-644 — the old guard here SKIPPED the degenerate draw, shearing the stream
/// for every downstream draw), collapsing the term to the bounds' midpoint. Drawn
/// from the one seeded stream, so the roll is deterministic and replayable.
pub(super) fn roll_term(
    scaling: &SeverityScaling,
    luck_defender: Luck,
    rng: &mut SeverityRng,
) -> f32 {
    let lo = -*scaling.defender_luck_scale * *luck_defender;
    let hi = *scaling.random_spread;
    rng.random_range_or_midpoint(lo..hi)
}

/// Compute the §6 severity score for `inputs` under `scaling`, drawing the random
/// term from `rng` — the bucketing input of [`roll_severity`] (resolution.md §6).
///
/// `score = j·pen − k·Toughness + part_mod + fatal_bias + I·Luck_shooter +
/// roll(−L·Luck_defender .. R)`. Split out from [`roll_severity`] so the tests can
/// assert the score relations (monotonicity, directional Luck, part-mod ordering)
/// without re-deriving the formula. Pure `f32` math; the only entropy is the one
/// seeded `rng` draw.
pub(super) fn severity_score(
    inputs: &SeverityInputs,
    scaling: &SeverityScaling,
    rng: &mut SeverityRng,
) -> f32 {
    let pen_term = *scaling.pen_damage_scale * pen_to_f32(*inputs.pen_damage);
    let toughness_term = *scaling.toughness_mitigation * *inputs.toughness;
    let shooter_term = *scaling.shooter_luck_scale * *inputs.luck_shooter;
    let roll = roll_term(scaling, inputs.luck_defender, rng);

    pen_term - toughness_term + *inputs.part_mod + *inputs.fatal_bias + shooter_term + roll
}

/// Roll a hit's wound severity — the §6 score → [`Severity`] bucket, in the
/// floor-extend form (resolution.md §6).
///
/// Computes the score
///
/// ```text
/// score = j·pen − k·Toughness + part_mod + fatal_bias + I·Luck_shooter
///         + roll(−L·Luck_defender .. R)
/// ```
///
/// (the random term is a uniform `f32` over `[−L·Luck_defender, R]`, drawn from
/// the injected seeded `rng`), then buckets it by the ascending §6 edges
/// `e0..e3`. **Penetration-gated**: a near-zero `pen_damage` keeps the score
/// below `e0` (a graze, [`Severity::None`]). The defender's Luck extends the
/// roll's floor down (a shrug-off chance) without capping the worst roll; the
/// shooter's Luck pushes the score up. Pure, deterministic for a fixed seed and
/// inputs — and carries no pixel.
#[must_use]
pub fn roll_severity(
    inputs: &SeverityInputs,
    scaling: &SeverityScaling,
    rng: &mut SeverityRng,
) -> Severity {
    let score = severity_score(inputs, scaling, rng);
    bucket(score, scaling)
}
