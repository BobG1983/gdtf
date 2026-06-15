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
//! [`crate::rng::SimRng`], and carries **no pixel**. The defender's Toughness and
//! both gangers' Luck are the E3.0 ganger attribute components
//! ([`crate::ganger::Toughness`] / [`crate::ganger::Luck`]) — sourced off the
//! entity, not bare literals. The per-part severity mod is a **placeholder** code
//! const ([`part_severity_mod`]), distinct from the §4 hit-location
//! [`crate::tuning::BodyPartWeights`] (this is severity *escalation*, not
//! hit-likelihood).

use crate::{
    armor::BodyPart,
    ganger::{Luck, Toughness},
    resolve_hit::PenetratingDamage,
    rng::SimRng,
    tuning::SeverityScaling,
    weapon::FatalBias,
};

/// The **severity bucket** a hit's §6 score falls into (resolution.md §6).
///
/// A named domain enum (no bare integer): the score is tiered by the ascending
/// edges `e0..e3` into one of these five outcomes. The ladder ascends from a
/// harmless graze ([`None`](Severity::None)) to death
/// ([`Fatal`](Severity::Fatal)); each non-`None` bucket costs the defender Wounds
/// (the per-tier Wounds cost is a later E3 slice).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// A **graze** (`< e0`): HP loss only, **no Wound** is spent — the
    /// penetration-gated floor of the ladder.
    None,
    /// A **minor** wound (`< e1`): the lightest injury that costs a Wound.
    Minor,
    /// A **major** wound (`< e2`): a worse injury, costing more Wounds.
    Major,
    /// A **critical** wound (`< e3`): a severe injury.
    Critical,
    /// A **fatal** hit (`≥ e3`): empties the Wounds pool — death in battle.
    Fatal,
}

impl Severity {
    /// The five severity buckets in ascending order (graze → death) — the
    /// canonical ordering the bucketing climbs through. Iteration order for
    /// tests and any per-severity lookup.
    pub const ALL: [Self; 5] = [
        Self::None,
        Self::Minor,
        Self::Major,
        Self::Critical,
        Self::Fatal,
    ];

    /// This bucket's rank on the ascending ladder (`None` = 0 … `Fatal` = 4) —
    /// the order used to compare two severities as relations (monotone /
    /// directional tests), without pinning any score magnitude.
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Minor => 1,
            Self::Major => 2,
            Self::Critical => 3,
            Self::Fatal => 4,
        }
    }
}

/// The **per-part severity modifier** — how much the struck [`BodyPart`] pushes
/// the §6 severity score (resolution.md §6: head +12 / torso +5 / arms −2 /
/// legs 0).
///
/// A named domain newtype (no bare `f32`): a signed score addend (the head
/// amplifies severity, the arms slightly dampen it). This is a **placeholder code
/// const** (resolution.md §6: "the per-part mods currently sit as a placeholder
/// code const"), **distinct** from the §4 hit-location
/// [`crate::tuning::BodyPartWeights`] — that decides *which* part is struck
/// (hit-likelihood), this decides how much a hit *there* escalates severity. An
/// `f32` so it sums directly into the `f32` score. Private inner + derived
/// [`Deref`].
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq)]
pub struct PartSeverityMod(f32);

impl PartSeverityMod {
    /// Build a per-part severity modifier from its signed score-addend magnitude
    /// (a placeholder; resolution.md §6).
    #[must_use]
    pub const fn new(part_mod: f32) -> Self {
        Self(part_mod)
    }
}

/// The placeholder per-part severity modifier for the struck [`BodyPart`]
/// (resolution.md §6: head +12 / torso +5 / arms −2 / legs 0).
///
/// A `const` accessor — the head amplifies a hit's severity, the torso a little,
/// the arms slightly dampen it, the legs are neutral. Both arms share the arm
/// value and both legs the leg value (the parts are split L/R, but the severity
/// escalation is the same per limb pair). This is **severity escalation**, NOT
/// the §4 hit-location weight ([`crate::tuning::BodyPartWeights`]). A placeholder
/// until the mods move to tuning data.
#[must_use]
pub const fn part_severity_mod(part: BodyPart) -> PartSeverityMod {
    match part {
        BodyPart::Head => PartSeverityMod(12.0),
        BodyPart::Torso => PartSeverityMod(5.0),
        BodyPart::LeftArm | BodyPart::RightArm => PartSeverityMod(-2.0),
        BodyPart::LeftLeg | BodyPart::RightLeg => PartSeverityMod(0.0),
    }
}

/// The six per-roll inputs to the §6 severity score, bundled into one named
/// struct (resolution.md §6).
///
/// Groups the score's logical input set so [`roll_severity`] stays under clippy's
/// argument-count gate (the same precedent as `ShotInputs` / `DamageProfile`
/// — the tuning `&SeverityScaling` and the entropy `&mut SimRng` stay their own
/// params). Every field is a named domain type sourced off the resolved hit and
/// the two gangers' entity components — none is a bare literal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeverityInputs {
    /// The hit's penetrating damage — gates the score (E3.3, pre-floor).
    pub pen_damage:    PenetratingDamage,
    /// The defender's Toughness — its mitigation term (off the entity, E3.0).
    pub toughness:     Toughness,
    /// The struck part's severity modifier ([`part_severity_mod`]).
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
/// When the bounds are degenerate (`lo >= hi` — e.g. an `R ≤ 0` tuning, or a
/// negative Luck), the term collapses to the upper bound `hi` rather than asking
/// the RNG for an empty range (which would be invalid). Drawn from the one seeded
/// stream, so the roll is deterministic and replayable.
fn roll_term(scaling: &SeverityScaling, luck_defender: Luck, rng: &mut SimRng) -> f32 {
    let lo = -*scaling.defender_luck_scale * *luck_defender;
    let hi = *scaling.random_spread;
    if lo >= hi {
        // Degenerate range: no spread to draw over — collapse to the ceiling.
        return hi;
    }
    rng.random_range(lo..hi)
}

/// Compute the §6 severity score for `inputs` under `scaling`, drawing the random
/// term from `rng` — the bucketing input of [`roll_severity`] (resolution.md §6).
///
/// `score = j·pen − k·Toughness + part_mod + fatal_bias + I·Luck_shooter +
/// roll(−L·Luck_defender .. R)`. Split out from [`roll_severity`] so the tests can
/// assert the score relations (monotonicity, directional Luck, part-mod ordering)
/// without re-deriving the formula. Pure `f32` math; the only entropy is the one
/// seeded `rng` draw.
fn severity_score(inputs: &SeverityInputs, scaling: &SeverityScaling, rng: &mut SimRng) -> f32 {
    let pen_term = *scaling.pen_damage_scale * pen_to_f32(*inputs.pen_damage);
    let toughness_term = *scaling.toughness_mitigation * *inputs.toughness;
    let shooter_term = *scaling.shooter_luck_scale * *inputs.luck_shooter;
    let roll = roll_term(scaling, inputs.luck_defender, rng);

    pen_term - toughness_term + *inputs.part_mod + *inputs.fatal_bias + shooter_term + roll
}

/// Bucket a §6 severity `score` into a [`Severity`] via the ascending edges
/// `e0..e3` (resolution.md §6: `< e0 → None`, `< e1 → Minor`, `< e2 → Major`,
/// `< e3 → Critical`, `≥ e3 → Fatal`).
///
/// Split out so the cutpoints live in one place; relies on the edges climbing
/// (`e0 < e1 < e2 < e3`, the [`crate::tuning::SeverityEdges`] invariant) for the
/// buckets to be monotone in the score.
fn bucket(score: f32, scaling: &SeverityScaling) -> Severity {
    let edges = &scaling.edges;
    if score < *edges.e0 {
        Severity::None
    } else if score < *edges.e1 {
        Severity::Minor
    } else if score < *edges.e2 {
        Severity::Major
    } else if score < *edges.e3 {
        Severity::Critical
    } else {
        Severity::Fatal
    }
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
    rng: &mut SimRng,
) -> Severity {
    let score = severity_score(inputs, scaling, rng);
    bucket(score, scaling)
}

#[cfg(test)]
mod tests {
    use bevy::ecs::world::World;

    use super::*;
    use crate::rng::BattleSeed;

    /// A fixed seed for the per-test RNG streams — determinism is the property, so
    /// the same seed must reproduce the same draws (an arbitrary value, not tuned).
    const SEED: u64 = 0xC0FF_EE15;

    /// Build a `SimRng` from the shared fixed seed (a fresh stream per call).
    fn rng() -> SimRng {
        SimRng::from_seed(BattleSeed::new(SEED))
    }

    /// Build a severity-input bundle from arbitrary inputs — a helper so each test
    /// varies only the field it exercises. Magnitudes are mechanism inputs, never
    /// asserted as values.
    fn inputs(
        pen: i32,
        toughness: f32,
        part: BodyPart,
        luck_shooter: f32,
        luck_defender: f32,
    ) -> SeverityInputs {
        SeverityInputs::new(
            PenetratingDamage::new(pen),
            Toughness::new(toughness),
            part_severity_mod(part),
            FatalBias::new(0.0),
            Luck::new(luck_shooter),
            Luck::new(luck_defender),
        )
    }

    /// AC1 — bucketing is monotone in the score: with a **fixed seed** (so the
    /// random term is identical across calls), rising `pen_damage` yields a
    /// non-decreasing [`Severity`] rank. Asserts the relation, never an edge value.
    #[test]
    fn severity_is_monotone_in_pen_damage() {
        let scaling = SeverityScaling::default();
        let mut prev = Severity::None;
        // A rising pen sweep; each call gets a FRESH same-seeded RNG so the roll
        // term is held identical and only pen moves the score.
        for pen in [0, 2, 5, 10, 20, 40, 80] {
            let mut r = rng();
            let got = roll_severity(
                &inputs(pen, 0.0, BodyPart::Torso, 0.0, 0.0),
                &scaling,
                &mut r,
            );
            assert!(
                got.rank() >= prev.rank(),
                "severity must not decrease as pen rises: {prev:?} -> {got:?} at pen {pen}",
            );
            prev = got;
        }
    }

    /// AC2 — penetration-gated, two ways (mechanism, never a magnitude):
    ///
    /// 1. The `< e0` graze branch is reachable for a near-zero-pen hit: with pen 0
    ///    and the score driven below `e0` by a dominating Toughness mitigation
    ///    (`−k·Toughness` outweighs the bounded `[lo, R]` roll), the bucket is
    ///    [`Severity::None`] regardless of the draw.
    /// 2. A near-zero-pen hit cannot reach the **severe** buckets that a high-pen
    ///    hit reaches: holding the seed and all else, the near-zero-pen bucket is
    ///    `≤` the high-pen bucket — pen genuinely gates the climb.
    #[test]
    fn near_zero_pen_cannot_reach_severe_buckets() {
        let scaling = SeverityScaling::default();

        // (1) None is reachable: a huge Toughness drives −k·Toughness below the
        // roll ceiling so the score is negative for ANY draw — a graze. The
        // Toughness magnitude is a test fixture (a dominating value), not a pinned
        // tuning number; we assert the branch, not a score.
        let mut graze_rng = rng();
        let graze = roll_severity(
            &inputs(0, 1.0e9, BodyPart::LeftLeg, 0.0, 0.0),
            &scaling,
            &mut graze_rng,
        );
        assert_eq!(
            graze,
            Severity::None,
            "a near-zero-pen hit whose score is driven below e0 must be a graze (None)",
        );

        // (2) Pen gates the climb: at the SAME seed, a near-zero-pen hit buckets
        // no higher than a high-pen hit (it cannot reach the severe buckets the
        // high-pen hit can). Identical inputs but pen — a relation, not a value.
        let mut low_rng = rng();
        let low_pen = roll_severity(
            &inputs(0, 0.0, BodyPart::Torso, 0.0, 0.0),
            &scaling,
            &mut low_rng,
        );
        let mut high_rng = rng();
        let high_pen = roll_severity(
            &inputs(100, 0.0, BodyPart::Torso, 0.0, 0.0),
            &scaling,
            &mut high_rng,
        );
        assert!(
            low_pen.rank() <= high_pen.rank(),
            "a near-zero-pen hit must not out-bucket a high-pen hit: {low_pen:?} > {high_pen:?}",
        );

        // STRICT (pins pen_term into the score): at a fixed same-seed RNG the roll
        // term is byte-identical, so high − low pen score is deterministically
        // j·Δpen = j·100 (> 0 for the default scaling). `>` fails iff `pen_term`
        // is reverted out of the score — the non-strict bucket relation above is
        // satisfied by a pen-constant (reverted) score, this raw-score check is not.
        let mut low_score_rng = rng();
        let low_pen_score = severity_score(
            &inputs(0, 0.0, BodyPart::Torso, 0.0, 0.0),
            &scaling,
            &mut low_score_rng,
        );
        let mut high_score_rng = rng();
        let high_pen_score = severity_score(
            &inputs(100, 0.0, BodyPart::Torso, 0.0, 0.0),
            &scaling,
            &mut high_score_rng,
        );
        assert!(
            high_pen_score > low_pen_score,
            "a higher pen_damage must strictly raise the score (pen_term wired in): {high_pen_score} <= {low_pen_score}",
        );
    }

    /// AC3 — directional Luck: holding the seed, a higher **shooter** Luck yields a
    /// `≥` bucket (pushes the score up) and a higher **defender** Luck yields a `≤`
    /// bucket (extends the floor down, ceiling fixed). Both directions asserted as
    /// relations with a same-seeded RNG per call (so only Luck moves).
    #[test]
    fn luck_is_directional() {
        let scaling = SeverityScaling::default();
        // A mid-range pen so there's room to move up or down between buckets.
        let base = |shooter: f32, defender: f32| {
            let mut r = rng();
            roll_severity(
                &inputs(8, 0.0, BodyPart::Torso, shooter, defender),
                &scaling,
                &mut r,
            )
        };

        // Shooter Luck: more ⇒ ≥ bucket.
        let low_shooter = base(0.0, 0.0);
        let high_shooter = base(6.0, 0.0);
        assert!(
            high_shooter.rank() >= low_shooter.rank(),
            "higher shooter Luck must yield >= severity: {low_shooter:?} -> {high_shooter:?}",
        );

        // STRICT (pins shooter_term into the score): at a fixed same-seed RNG the
        // roll term is byte-identical, so high − low shooter score is
        // deterministically I·Δluck = I·6.0 (> 0 for the default scaling). `>`
        // fails iff `shooter_term` is reverted out of the score — the bucket
        // relation above is vacuous on revert (equal scores), this is not.
        let mut low_shooter_rng = rng();
        let low_shooter_score = severity_score(
            &inputs(8, 0.0, BodyPart::Torso, 0.0, 0.0),
            &scaling,
            &mut low_shooter_rng,
        );
        let mut high_shooter_rng = rng();
        let high_shooter_score = severity_score(
            &inputs(8, 0.0, BodyPart::Torso, 6.0, 0.0),
            &scaling,
            &mut high_shooter_rng,
        );
        assert!(
            high_shooter_score > low_shooter_score,
            "higher shooter Luck must strictly raise the score (shooter_term wired in): {high_shooter_score} <= {low_shooter_score}",
        );

        // Defender Luck: more ⇒ ≤ bucket (floor extends down; ceiling fixed).
        let low_defender = base(0.0, 0.0);
        let high_defender = base(0.0, 6.0);
        assert!(
            high_defender.rank() <= low_defender.rank(),
            "higher defender Luck can only lower or hold severity: {low_defender:?} -> {high_defender:?}",
        );
    }

    /// AC4 — the random term is `roll(−L·Luck_defender .. R)`: over a stream of
    /// draws, a high-Luck defender's realized roll term goes **negative** (the
    /// floor is below 0) AND **never exceeds `R`** (the ceiling is fixed) —
    /// confirming NO `R_eff` / NO `R_min` (the lower bound moves, the spread is not
    /// merely shrunk). Exercised through the crate-internal [`roll_term`].
    #[test]
    fn roll_term_floor_extends_below_zero_and_ceiling_is_fixed() {
        let scaling = SeverityScaling::default();
        let ceiling = *scaling.random_spread;
        let mut r = rng();

        let mut saw_negative = false;
        // Many draws so the negative tail is virtually certain to appear.
        for _ in 0..1_000 {
            let term = roll_term(&scaling, Luck::new(6.0), &mut r);
            assert!(
                term <= ceiling,
                "the roll term must never exceed R (the fixed ceiling): {term} > {ceiling}",
            );
            if term < 0.0 {
                saw_negative = true;
            }
        }
        assert!(
            saw_negative,
            "a high-Luck defender's roll term must be able to go negative (floor below 0)",
        );
    }

    /// AC4 (counterpart) — with **zero** defender Luck the floor is exactly 0, so
    /// the roll term is always non-negative and still bounded by `R`. This proves
    /// the floor is `−L·Luck_defender` (not a fixed negative), so the lower bound
    /// genuinely *moves* with the defender's Luck.
    #[test]
    fn roll_term_floor_is_zero_without_defender_luck() {
        let scaling = SeverityScaling::default();
        let ceiling = *scaling.random_spread;
        let mut r = rng();
        for _ in 0..1_000 {
            let term = roll_term(&scaling, Luck::new(0.0), &mut r);
            assert!(
                (0.0..=ceiling).contains(&term),
                "with no defender Luck the term stays in [0, R]: {term}",
            );
        }
    }

    /// AC5 — `part_mod` is location-dependent: at an identical seed and inputs, a
    /// **head** hit yields a `≥` score (and bucket) than a **leg** hit (head +12 vs
    /// legs 0). Asserts the raw score relation (the part mod is wired in) AND the
    /// bucket relation. Same-seeded RNG per call so only the part moves.
    #[test]
    fn head_hit_outscores_leg_hit() {
        let scaling = SeverityScaling::default();

        let mut head_rng = rng();
        let head_score = severity_score(
            &inputs(8, 0.0, BodyPart::Head, 0.0, 0.0),
            &scaling,
            &mut head_rng,
        );
        let mut leg_rng = rng();
        let leg_score = severity_score(
            &inputs(8, 0.0, BodyPart::LeftLeg, 0.0, 0.0),
            &scaling,
            &mut leg_rng,
        );
        // STRICT: at a fixed same-seed RNG the roll term is byte-identical, so
        // head_score − leg_score is deterministically +12.0 (head +12 vs leg 0).
        // `>` (not `>=`) is therefore always correct here AND fails iff the
        // `+ part_mod` term is reverted out of the score — pinning the wiring.
        assert!(
            head_score > leg_score,
            "a head hit must strictly out-score a leg hit at identical seed/inputs (part_mod wired in): {head_score} <= {leg_score}",
        );

        let mut head_b = rng();
        let head_sev = roll_severity(
            &inputs(8, 0.0, BodyPart::Head, 0.0, 0.0),
            &scaling,
            &mut head_b,
        );
        let mut leg_b = rng();
        let leg_sev = roll_severity(
            &inputs(8, 0.0, BodyPart::LeftLeg, 0.0, 0.0),
            &scaling,
            &mut leg_b,
        );
        assert!(
            head_sev.rank() >= leg_sev.rank(),
            "a head hit must yield >= severity than a leg hit: {leg_sev:?} -> {head_sev:?}",
        );
    }

    /// AC6 — seeded determinism: two `roll_severity` SEQUENCES from the same
    /// [`BattleSeed`] produce the identical [`Severity`] sequence (replay equality).
    /// Drawing a sequence (advancing one shared RNG) proves the stream — not just a
    /// single draw — is reproduced.
    #[test]
    fn same_seed_reproduces_the_severity_sequence() {
        let scaling = SeverityScaling::default();
        let sequence = |seed: u64| {
            let mut r = SimRng::from_seed(BattleSeed::new(seed));
            // Varying inputs across the sequence so the stream is genuinely walked.
            (0..32)
                .map(|i| {
                    let pen = i % 12;
                    roll_severity(
                        &inputs(pen, 1.0, BodyPart::Torso, 1.0, 2.0),
                        &scaling,
                        &mut r,
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            sequence(SEED),
            sequence(SEED),
            "the same battle seed must reproduce the identical severity sequence (replay equality)",
        );
    }

    /// AC7 (mechanism) / no-bare-types — [`Severity`] is a named enum with the five
    /// buckets in ascending rank, and [`PartSeverityMod`] derefs to its inner
    /// `f32`. Pins the vocabulary + the Deref mechanism (arbitrary values), never a
    /// tuning magnitude.
    #[test]
    fn severity_vocabulary_and_part_mod_newtype() {
        assert_eq!(Severity::ALL.len(), 5);
        // Ascending ranks, distinct per bucket.
        let ranks: Vec<u8> = Severity::ALL.iter().map(|s| s.rank()).collect();
        assert_eq!(ranks, vec![0, 1, 2, 3, 4]);
        // The part-mod newtype derefs to its inner f32 (arbitrary literal).
        assert!((*PartSeverityMod::new(3.5) - 3.5).abs() < f32::EPSILON);
        // The placeholder per-part ordering: head > torso > legs >= arms-or-below.
        assert!(*part_severity_mod(BodyPart::Head) > *part_severity_mod(BodyPart::Torso));
        assert!(*part_severity_mod(BodyPart::Torso) > *part_severity_mod(BodyPart::LeftLeg));
        assert!(*part_severity_mod(BodyPart::LeftLeg) > *part_severity_mod(BodyPart::LeftArm));
    }

    /// Contract — the defender's Toughness and both gangers' Luck are **sourced off
    /// the entity** (the E3.0 components), then fed to `roll_severity`. A bare
    /// `World` spawns a shooter and a defender each carrying the attribute
    /// components, a `Query`/`World` access reads them back, and they drive the
    /// roll — proving the stats come off the entity, not bare literals.
    #[test]
    fn stats_are_sourced_off_the_entity() {
        let scaling = SeverityScaling::default();

        let mut world = World::new();
        let shooter = world.spawn(Luck::new(2.0)).id();
        let defender = world.spawn((Toughness::new(3.0), Luck::new(4.0))).id();

        // Read the shooter's Luck off its entity.
        let mut shooter_q = world.query::<&Luck>();
        let shooter_read = shooter_q.get(&world, shooter);
        assert!(
            shooter_read.is_ok(),
            "shooter Luck must be queryable off the entity",
        );
        let Ok(&luck_shooter) = shooter_read else {
            return;
        };

        // Read the defender's Toughness + Luck off its entity.
        let mut defender_q = world.query::<(&Toughness, &Luck)>();
        let defender_read = defender_q.get(&world, defender);
        assert!(
            defender_read.is_ok(),
            "defender Toughness + Luck must be queryable off the entity",
        );
        let Ok((&toughness, &luck_defender)) = defender_read else {
            return;
        };

        // Feed the entity-sourced stats into the roll — the real read shape.
        let bundle = SeverityInputs::new(
            PenetratingDamage::new(10),
            toughness,
            part_severity_mod(BodyPart::Torso),
            FatalBias::new(0.0),
            luck_shooter,
            luck_defender,
        );
        let mut r = rng();
        let got = roll_severity(&bundle, &scaling, &mut r);
        // Value-agnostic: a real Severity came back from entity-sourced stats.
        assert!(
            Severity::ALL.contains(&got),
            "roll_severity must return a Severity from entity-sourced stats: {got:?}",
        );
    }
}
