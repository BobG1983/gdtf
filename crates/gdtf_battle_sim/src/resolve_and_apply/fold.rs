//! The E3.9 fold act — [`resolve_and_apply`] composes matchup → [`resolve_hit`] →
//! [`roll_severity`] → [`apply_hit`] into ONE model-side act, plus the
//! [`struck_piece`] armored-vs-bare-flesh resolution it runs against.

use bevy::prelude::Entity;

use crate::{
    apply_hit::{GangerHitTarget, apply_hit},
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType,
        BodyPart, WornArmor,
    },
    armor_wear::ArmorWearOutcome,
    ganger::{LifeState, Luck},
    matchup::{Matchup, matchup},
    resolve_and_apply::report::{AppliedDamage, HitReport, TargetGanger},
    resolve_coarse::{ShotKind, ShotOutcome},
    resolve_hit::resolve_hit,
    rng::SimRng,
    severity::{SeverityInputs, part_severity_mod, roll_severity},
    tuning::CombatTuning,
    weapon::WeaponStats,
};

/// The **bare-flesh** armor piece — a zeroed soak used when the struck location no
/// longer [`protects`](WornArmor::protects) (`weapons-and-armor.md` §"Per-hit
/// resolution": "later hits on that location resolve as bare flesh").
///
/// Floor / protection / hardness are all `0` so the per-hit formula soaks nothing
/// (on bare flesh `dmg == damage` regardless of matchup, since protection `0`),
/// integrity is `0` (the piece is already worn through — there is nothing to wear),
/// and the type is [`ArmorType::DEFAULT`] (an explicit placeholder — the matchup on
/// bare flesh is forced to [`Matchup::Neutral`], so the armor type is never matched
/// against). A `const` since every stat is a constant zero.
const BARE_FLESH: ArmorPiece = ArmorPiece::new(
    ArmorFloor::new(0),
    ArmorProtection::new(0),
    ArmorIntegrity::new(0),
    ArmorHardness::new(0),
    ArmorType::DEFAULT,
);

/// Resolve the struck location to the `(`[`ArmorPiece`]`, `[`Matchup`]`)` the
/// per-hit formula runs against — the armored branch or **bare flesh**.
///
/// If the worn piece at `part` still [`protects`](WornArmor::protects), returns that
/// piece and the E3.2 [`matchup`] of the weapon's
/// [`DamageType`](crate::weapon::DamageType) vs the piece's [`ArmorType`] (the wheel
/// advantage applies). Otherwise returns the zeroed [`BARE_FLESH`] piece under
/// [`Matchup::Neutral`] — there is no armor type to match against, so no wheel
/// advantage, and the zeroed soak means the hit lands as full weapon damage
/// (`weapons-and-armor.md` §"Per-hit resolution").
fn struck_piece(
    worn: &WornArmor,
    part: BodyPart,
    weapon: WeaponStats<'_>,
) -> (ArmorPiece, Matchup) {
    if worn.protects(part) {
        let piece = worn.at(part);
        let resolved = matchup(*weapon.damage_type, piece.armor_type);
        (piece, resolved)
    } else {
        // Bare flesh: no protection / hardness, no armor type to match → Neutral.
        (BARE_FLESH, Matchup::Neutral)
    }
}

/// Fold **one [`ShotOutcome`]** through damage → severity → application into ONE
/// frozen [`HitReport`] — the E3.9 capstone integrator (`docs/combat/resolution.md`
/// §5 / §6).
///
/// Composes the already-built E3 verbs (E3.2 [`matchup`] → E3.3 [`resolve_hit`] →
/// E3.4 [`roll_severity`] → E3.6 [`apply_hit`]); it rebuilds none of them. The
/// flow, in order:
///
/// 1. **Kind gate** — a non-[`ShotKind::Ganger`] outcome returns a no-effect
///    report with **no draw and no mutation** (a non-ganger never wounds a ganger).
/// 2. **Corpse-skip — before any draw** — a target already at [`LifeState::Dead`]
///    returns a no-effect report with **no draw and no mutation**, so a corpse
///    never consumes an RNG draw and determinism is preserved.
/// 3. **Part** — the struck [`BodyPart`] is
///    [`ShotOutcome::body_part`](crate::resolve_coarse::ShotOutcome::body_part)
///    (drawn upstream); a defensive `None` returns a no-effect report.
/// 4. **Armor / bare flesh** — [`struck_piece`] picks the armored piece + matchup
///    or the zeroed bare-flesh piece under [`Matchup::Neutral`].
/// 5. **Damage** — [`resolve_hit`] → [`HitResult`](crate::resolve_hit::HitResult).
/// 6. **Severity (the ONE draw)** — [`roll_severity`] over the assembled
///    [`SeverityInputs`], drawing from the injected [`SimRng`] — the single draw
///    point (no `thread_rng`, no ad-hoc entropy).
/// 7. **Apply** — [`apply_hit`] folds the hit onto the target in place.
/// 8. **Freeze** — returns the [`HitReport`] of named newtypes.
///
/// Pure, render-free model logic. It mutates the target ganger's battle state in
/// place (and advances the injected [`SimRng`] by **exactly one** severity draw,
/// only on a ganger hit) and **owns no mutation after return**. Same
/// [`BattleSeed`](crate::rng::BattleSeed) → identical report for identical inputs
/// (the seeded-replay property). Charging TU and looping the burst is the E4
/// `fire()` act — **out of scope** here.
#[must_use]
pub fn resolve_and_apply(
    outcome: &ShotOutcome,
    weapon: WeaponStats<'_>,
    shooter_luck: Luck,
    target: TargetGanger<'_>,
    target_entity: Entity,
    tuning: &CombatTuning,
    rng: &mut SimRng,
) -> HitReport {
    // (1) Kind gate: only a Ganger outcome can wound. Non-ganger → no draw, no
    // mutation (the round struck cover / a slab / the ground / nothing — never a
    // ganger).
    let ShotKind::Ganger(_) = outcome.kind else {
        return HitReport::no_effect(outcome.kind);
    };

    // (2) Corpse-skip BEFORE any draw: a dead target is final — no draw is taken
    // (a corpse never consumes an RNG draw), nothing mutates.
    if *target.life == LifeState::Dead {
        return HitReport::no_effect(outcome.kind);
    }

    // (3) The struck part rode along on the §4 part roll (drawn upstream); a Ganger
    // outcome carries Some. A defensive None folds to a no-effect report (no draw).
    let Some(part) = outcome.body_part else {
        return HitReport::no_effect(outcome.kind);
    };

    // (4) Armored piece + matchup, or zeroed bare flesh under Neutral.
    let (piece, resolved_matchup) = struck_piece(target.worn, part, weapon);

    // (5) The per-hit damage formula (E3.3) — pure, mutates nothing.
    let hit = resolve_hit(
        *weapon.damage,
        *weapon.punch,
        *weapon.shred,
        &piece,
        resolved_matchup,
        tuning,
    );

    // (6) The ONE RNG draw: the wound-severity roll (E3.4). Both gangers' Luck, the
    // defender's Toughness, the struck part's mod, and the weapon's fatal bias feed
    // it; the injected SimRng is the single draw point.
    let inputs = SeverityInputs::new(
        hit.penetrating,
        target.toughness,
        part_severity_mod(part),
        *weapon.fatal_bias,
        shooter_luck,
        target.luck,
    );
    let severity = roll_severity(&inputs, &tuning.severity_scaling, rng);

    // (7) Apply the resolved hit onto the target in place (E3.6) — HP loss +
    // Wounds-by-tier + armor wear + the terminal gates; capture the per-hit
    // ArmorWearOutcome (the break crossing / a non-breaking reduction / nothing).
    let wear_outcome = apply_hit(
        GangerHitTarget {
            hp:        target.hp,
            wounds:    target.wounds,
            life:      target.life,
            worn:      target.worn,
            inflicted: target.inflicted,
        },
        &hit,
        severity,
        part,
        target_entity,
        tuning,
    );

    // Map the mutually-exclusive outcome onto the report's two sibling armor fields:
    // a Broke hit sets `broken` (worn None), a Worn hit sets `worn` (broken None),
    // an Unaffected hit leaves BOTH None (GTW-313). At most one is ever Some.
    let (broken, worn) = match wear_outcome {
        ArmorWearOutcome::Broke(broken) => (Some(broken), None),
        ArmorWearOutcome::Worn(worn) => (None, Some(worn)),
        ArmorWearOutcome::Unaffected => (None, None),
    };

    // (8) Freeze the verdict — a Copy record of named newtypes, no pixel.
    HitReport {
        kind:    outcome.kind,
        part:    Some(part),
        applied: Some(AppliedDamage {
            matchup: resolved_matchup,
            hit,
            severity,
            life_after: *target.life,
            broken,
            worn,
        }),
    }
}
