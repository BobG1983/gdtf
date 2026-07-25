//! The §7 strike sequence itself — [`resolve_melee_strike`], plus the melee path's own
//! armor-input resolution (the armored piece or zeroed bare flesh).

use crate::{
    armor::{ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType},
    hit_location::roll_body_part,
    injuries::DamageContext,
    matchup::{Matchup, matchup},
    melee::{
        Connected, melee_damage_mult, opposed_fight,
        strike::{Combatants, MeleeStrike, MeleeStrikeEnv, MeleeWeaponHit},
    },
    resolve_and_apply::{StruckPiece, TargetGanger, WoundBlow, WoundCoreInputs, synthesize_wound},
};

/// The **bare-flesh** armor piece — a zeroed soak used when the struck location wears no
/// protecting piece (the [`crate::resolve_and_apply`] `BARE_FLESH` shape, mirrored here so the
/// melee verb stays self-contained and never reaches a private item).
///
/// Floor / protection / hardness / integrity are all `0` so the per-hit formula soaks nothing
/// (`dmg == damage`), and the type is [`ArmorType::DEFAULT`] (the matchup is forced
/// [`Matchup::Neutral`] on bare flesh, so the type is never matched against).
const BARE_FLESH: ArmorPiece = ArmorPiece::new(
    ArmorFloor::new(0),
    ArmorProtection::new(0),
    ArmorIntegrity::new(0),
    ArmorHardness::new(0),
    ArmorType::DEFAULT,
);

/// Resolve the worn struck piece (if any) to the `(`[`ArmorPiece`]`, `[`Matchup`]`)` the §5
/// formula runs against — the armored branch or zeroed **bare flesh** (the
/// [`crate::resolve_and_apply`] `struck_piece` resolution, mirrored for the melee verb so it
/// reaches no private item).
///
/// A worn piece that still [`protects`](StruckPiece::protects) yields its stats + the §3.2
/// [`matchup`] of the weapon's [`DamageType`](crate::weapon::DamageType) vs the piece's
/// [`ArmorType`]; otherwise (no piece, or it is worn through) the zeroed [`BARE_FLESH`] piece
/// under [`Matchup::Neutral`] (no wheel advantage; the strike lands as full weapon damage).
fn melee_struck_piece(
    piece: Option<&StruckPiece<'_>>,
    weapon: MeleeWeaponHit<'_>,
) -> (ArmorPiece, Matchup) {
    match piece {
        Some(p) if *p.protects() => {
            let assembled = ArmorPiece::new(
                p.floor,
                p.protection,
                p.integrity_value(),
                p.hardness,
                p.armor_type,
            );
            let resolved = matchup(*weapon.damage_type, p.armor_type);
            (assembled, resolved)
        }
        _ => (BARE_FLESH, Matchup::Neutral),
    }
}

/// Resolve ONE §7 melee strike onto `target` — the connecting-hit synthesis (GTW-507,
/// routed through the shared wound core by GTW-821).
///
/// Sequences the §7 steps (`docs/combat/resolution.md` §7), REUSING every landed
/// combat-math piece verbatim (it reimplements none):
///
/// 1. **Part roll** — [`roll_body_part`] picks the struck [`BodyPart`](crate::armor::BodyPart)
///    from `tuning.body_part_weights`, drawing from `shot_rng` (the §4 location roll).
/// 2. **Opposed Fight** — [`opposed_fight`](crate::melee::opposed_fight) (GTW-506) runs the §7
///    roll, two `fight_rng` draws → a [`FightOutcome`](crate::melee::FightOutcome) (connect +
///    margin).
/// 3. **Miss** — when the opposed roll is LOST (`!connect`) the verb returns a no-effect
///    (no-connect) [`MeleeStrike`] immediately: NO damage, NO `severity_rng` draw, NO
///    `injury_rng` draw (a clean miss).
/// 4. **Armor input + margin multiplier** — on a connect, the struck worn piece (or bare
///    flesh) is resolved, and [`melee_damage_mult`](crate::melee::melee_damage_mult)
///    (GTW-506) turns the §7 margin into the [`MeleeDamageMult`](crate::melee::MeleeDamageMult)
///    the blow carries.
/// 5. **The shared wound core** — the resolved blow is handed to
///    `synthesize_wound`, which runs the ONE §5 → §6 → §8 fold the ranged fire path and
///    the fall path also run: corpse-skip → `resolve_hit` (§5) →
///    [`apply_melee_multiplier`](crate::melee::apply_melee_multiplier) over the RESOLVED hit (§7, the post-armor placement
///    GTW-506/507 chose) → `roll_severity` (§6, the ONE `severity_rng` draw) → `apply_hit`
///    (§6, folding HP loss + Wounds-by-tier + armor wear + the terminal gates onto `target`
///    in place) → `roll_injury` (§8, the ONE `injury_rng` draw on a non-graze / non-fatal
///    wound), sampling the **melee** per-source weighting table
///    ([`DamageContext::Melee`](crate::injuries::DamageContext::Melee), GTW-452/GTW-821).
///
/// Returns the frozen [`MeleeStrike`] (the connect verdict, the rolled severity, the applied
/// HP loss, the armor-wear outcome, and the rolled injury the owning dispatcher bridges into
/// the EXISTING [`InjuryInflicted`](crate::acts::InjuryInflicted) message). It is
/// deterministic for a fixed seed + inputs (the four injected streams are the only entropy)
/// and owns no mutation after return beyond the in-place fold onto `target`. A corpse target
/// is corpse-skipped INSIDE the shared core (no §6 / §8 draw, no mutation) and reports a
/// connect with nothing applied; the owning dispatcher's alive-gate already excludes a dead
/// target before this is called.
///
/// # Draw-stream discipline
///
/// On a **connect** onto a live target it draws exactly: the §4 part roll (`shot_rng`), the
/// two §7 opposed-Fight rolls (`fight_rng`), the ONE §6 severity term (`severity_rng`), and —
/// gated on that severity — the ONE §8 injury term (`injury_rng`): a `Minor`/`Major`/`Critical`
/// wound ALWAYS takes exactly one (even on an empty/missing bucket — content-independent
/// stream alignment), a graze ([`Severity::None`](crate::severity::Severity::None)) or a
/// `Fatal` takes NONE. On a **miss** it draws the §4 part roll + the two §7 rolls but neither
/// the severity nor the injury term — the §7 `connect` gate short-circuits before §5/§6/§8.
/// (The part roll is taken up front, mirroring the ranged pipeline where `roll_body_part` runs
/// inside the march before the wound fold.)
#[must_use]
pub fn resolve_melee_strike(
    combatants: Combatants,
    weapon: MeleeWeaponHit<'_>,
    target: TargetGanger<'_>,
    target_entity: bevy::prelude::Entity,
    env: MeleeStrikeEnv<'_>,
) -> MeleeStrike {
    let MeleeStrikeEnv {
        tuning,
        tables,
        registry,
        fight_rng,
        shot_rng,
        severity_rng,
        injury_rng,
    } = env;

    // (1) §4 part roll — which location the strike lands on (defender stance is folded into
    // the body-part weights at tuning time; the verb draws the weighted part here).
    let part = roll_body_part(&tuning.body_part_weights, shot_rng.rng());

    // (2) §7 opposed-Fight (GTW-506) — two FightRng draws → connect + margin.
    let outcome = opposed_fight(
        combatants.attacker_fight,
        combatants.defender_fight,
        tuning.melee.variance,
        fight_rng,
    );

    // (3) Miss — the opposed roll was lost: no damage, no severity draw, no injury draw.
    if !*outcome.connect {
        return MeleeStrike::MISS;
    }

    // (4) The melee path's own inputs: the struck piece (or bare flesh) + the §7 margin
    // multiplier (GTW-506) the blow carries into the core, so a dominant blow reaches a
    // worse wound bucket — the whole point of §7.
    let (piece, resolved_matchup) = melee_struck_piece(target.piece.as_ref(), weapon);
    let mult = melee_damage_mult(outcome.margin, &tuning.melee);

    // (5) The SHARED wound-synthesis core (GTW-523, joined by melee in GTW-821): corpse-skip
    // → resolve_hit (§5) → apply_melee_multiplier (§7, post-armor) → roll_severity (the ONE
    // SeverityRng draw) → apply_hit (§6) → roll_injury (the ONE severity-gated InjuryRng
    // draw) against the MELEE weighting tables. No melee-local copy of that sequence exists.
    let Some(synthesis) = synthesize_wound(WoundCoreInputs {
        blow: WoundBlow {
            part,
            damage: *weapon.damage,
            punch: *weapon.punch,
            shred: *weapon.shred,
            piece,
            matchup: resolved_matchup,
            fatal_bias: *weapon.fatal_bias,
            shooter_luck: combatants.attacker_luck,
            // A §7 strike is a MELEE wound (GTW-452 / GTW-821) — the §8 injury roll samples
            // the melee per-source weighting table over the shared per-category pool.
            context: DamageContext::Melee,
            damage_mult: Some(mult),
        },
        target,
        target_entity,
        tuning,
        severity_rng,
        tables,
        registry,
        injury_rng,
    }) else {
        // The core's corpse-skip (a target already Dead): nothing drawn, nothing applied.
        return MeleeStrike::CORPSE;
    };

    MeleeStrike {
        connect:   Connected::new(true),
        severity:  synthesis.severity,
        // The SCALED applied HP loss (GTW-572) — the same number the core folded onto the
        // target — so the owning dispatcher's MeleeStruck fact can carry it.
        hp_damage: synthesis.hit.hp_damage,
        wear:      synthesis.wear,
        // The §8 draw (GTW-821) the dispatcher bridges into InjuryInflicted.
        injury:    synthesis.injury,
    }
}
