//! The §7 melee STRIKE resolution verb — the pure, render-free function that sequences
//! the connecting-hit synthesis for a live melee act (GTW-507, child GTW-37c of GTW-37).
//!
//! `docs/combat/resolution.md` §7 designs a melee attack as an **opposed roll** whose
//! relative margin scales the blow as a multiplier; a connecting hit then runs the normal
//! §5 damage → §6 wound steps. This verb COMPOSES the already-landed combat-math pieces in
//! that exact order — it REIMPLEMENTS NONE of them:
//!
//! 1. [`roll_body_part`](crate::hit_location::roll_body_part) — the §4 weighted part roll
//!    (the defender's struck location), drawn from the injected [`ShotRng`](crate::rng::ShotRng).
//! 2. [`opposed_fight`](crate::melee::opposed_fight) — the §7 opposed-Fight (GTW-506), two
//!    [`FightRng`](crate::rng::FightRng) draws → a [`FightOutcome`](crate::melee::FightOutcome).
//! 3. on a **miss** (`!connect`) — NO damage, NO further draw: the verb returns a no-effect
//!    [`MeleeStrike`] (the §7 "connect if atk > def" gate).
//! 4. on a **connect** — [`resolve_hit`](crate::resolve_hit::resolve_hit) (§5) →
//!    [`apply_melee_multiplier`](crate::melee::apply_melee_multiplier)
//!    ([`melee_damage_mult`](crate::melee::melee_damage_mult), GTW-506) → the scaled
//!    [`HitResult`](crate::resolve_hit::HitResult).
//! 5. [`roll_severity`](crate::severity::roll_severity) (§6) over the SCALED penetrating
//!    damage, drawn from the injected [`SeverityRng`](crate::rng::SeverityRng).
//! 6. [`apply_hit`](crate::apply_hit::apply_hit) (§6) — folds HP loss + Wounds-by-tier +
//!    armor wear + the terminal gates onto the target in place.
//!
//! Pure model logic: no systems, no `&mut World`, no ECS trigger, no pixel. The owning
//! [`dispatch_melee`](crate::acts::dispatch_melee) system (GTW-507) assembles the borrow-views
//! from queried components, gates 8-adjacency + LOS + alive + opposing faction, spends the
//! weapon's fight-mode TU, and emits the presenter signal — this verb is the combat core it
//! calls once the gates pass.

use crate::{
    apply_hit::{GangerHitTarget, apply_hit},
    armor::{ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType},
    armor_wear::ArmorWearOutcome,
    ganger::{Fight, Luck},
    hit_location::roll_body_part,
    matchup::{Matchup, matchup},
    melee::{apply_melee_multiplier, melee_damage_mult, opposed_fight},
    resolve_and_apply::{StruckPiece, TargetGanger},
    resolve_hit::{HpDamage, resolve_hit},
    rng::{FightRng, SeverityRng, ShotRng},
    severity::{Severity, SeverityInputs, part_severity_mod, roll_severity},
    tuning::CombatTuning,
    weapon::{DamageType, FatalBias, WeaponDamage, WeaponPunch, WeaponShred},
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

/// The wielded **melee weapon's** per-hit damage stats the strike resolves through — the
/// shared §5 weapon numbers a melee weapon carries (GTW-505: the damage group + the §6
/// [`FatalBias`]), bundled into one borrow-view so [`resolve_melee_strike`] stays under
/// clippy's argument-count gate.
///
/// A transparent borrow record over the weapon entity's existing named newtypes (no bare
/// primitive) — the melee mirror of the ranged [`WeaponStats`](crate::weapon::WeaponStats),
/// minus the ranged-only handling fields a melee weapon has none of. Assembled by
/// [`dispatch_melee`](crate::acts::dispatch_melee) from the wielded melee weapon entity's
/// components.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeleeWeaponHit<'a> {
    /// The base damage a strike deals before armor — the §5 `damage` term.
    pub damage:      &'a WeaponDamage,
    /// The armor protection a strike ignores — the §5 `punch` (penetration) term.
    pub punch:       &'a WeaponPunch,
    /// The extra integrity damage a strike deals to armor durability — the §5 `shred` term.
    pub shred:       &'a WeaponShred,
    /// The damage type the weapon emits — the §5 matchup-wheel node.
    pub damage_type: &'a DamageType,
    /// The severity-score addend — the §6 `fatal_bias` term.
    pub fatal_bias:  &'a FatalBias,
}

/// The two combatants' opposed-Fight inputs — each side's effective [`Fight`] plus the
/// attacker's [`Luck`] (the §6 score's nasty-wound term), bundled so the verb's signature
/// stays under clippy's argument-count gate.
///
/// A transparent argument record of `Copy` ganger newtypes (no bare primitive). The defender's
/// [`Fight`] feeds the §7 opposed roll; the defender's `Toughness`/`Luck` ride the
/// [`TargetGanger`] the verb folds onto.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Combatants {
    /// The attacker's effective [`Fight`] — the §7 `Fight_attacker`.
    pub attacker_fight: Fight,
    /// The defender's effective [`Fight`] — the §7 `Fight_defender`.
    pub defender_fight: Fight,
    /// The attacker's [`Luck`] — the §6 score's shooter-Luck (nasty-wound push) term.
    pub attacker_luck:  Luck,
}

/// The frozen verdict one [`resolve_melee_strike`] produces — whether the strike connected
/// and, on a connect, the rolled [`Severity`] plus the applied HP loss (GTW-507 / GTW-572).
///
/// A `Copy` value object of named domain types (no bare primitive, no pixel) the owning
/// [`dispatch_melee`](crate::acts::dispatch_melee) reads to decide its output: a `connect`
/// (the §7 `atk > def` verdict) is the gate the presenter [`MeleeResolved`](crate::acts::MeleeResolved)
/// signal + the per-strike effects key off, `severity` is the rolled §6 bucket on a
/// connecting hit (the wound the §6 step already spent onto the target), and `hp_damage`
/// is the SCALED per-hit HP loss the §6 fold applied (GTW-572 — surfaced so the
/// [`MeleeStruck`](crate::acts::MeleeStruck) fact can carry the damage number; before,
/// the number was computed and dropped inside the verb). A miss carries
/// `connect == false`, [`Severity::None`], and a zero `hp_damage` (nothing was applied).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeleeStrike {
    /// `true` iff the §7 opposed roll connected (`atk > def`) — the strike landed.
    pub connect:   bool,
    /// The rolled §6 wound severity on a connecting hit; [`Severity::None`] on a miss.
    pub severity:  Severity,
    /// The §5→§7-multiplied HP loss the connecting hit applied; zero on a miss (GTW-572).
    pub hp_damage: HpDamage,
    /// The §6 armor-wear outcome the fold applied onto the struck worn piece —
    /// [`ArmorWearOutcome::Unaffected`] on a miss (GTW-572: previously computed and dropped
    /// inside the verb, now surfaced so a melee armor BREAK emits the
    /// [`ArmorBroken`](crate::armor_wear::ArmorBroken) fact like a ranged one).
    pub wear:      ArmorWearOutcome,
}

impl MeleeStrike {
    /// A **missed** strike — the opposed roll was lost, so no damage was applied, no
    /// severity was drawn ([`Severity::None`], zero HP loss), and no armor was worn.
    const MISS: Self = Self {
        connect:   false,
        severity:  Severity::None,
        hp_damage: HpDamage::new(0),
        wear:      ArmorWearOutcome::Unaffected,
    };
}

/// Resolve the worn struck piece (if any) to the `(`[`ArmorPiece`]`, `[`Matchup`]`)` the §5
/// formula runs against — the armored branch or zeroed **bare flesh** (the
/// [`crate::resolve_and_apply`] `struck_piece` resolution, mirrored for the melee verb so it
/// reaches no private item).
///
/// A worn piece that still [`protects`](StruckPiece::protects) yields its stats + the §3.2
/// [`matchup`] of the weapon's [`DamageType`] vs the piece's [`ArmorType`]; otherwise (no
/// piece, or it is worn through) the zeroed [`BARE_FLESH`] piece under [`Matchup::Neutral`]
/// (no wheel advantage; the strike lands as full weapon damage).
fn melee_struck_piece(
    piece: Option<&StruckPiece<'_>>,
    weapon: MeleeWeaponHit<'_>,
) -> (ArmorPiece, Matchup) {
    match piece {
        Some(p) if p.protects() => {
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

/// Resolve ONE §7 melee strike onto `target` — the connecting-hit synthesis (GTW-507).
///
/// Sequences the six steps (`docs/combat/resolution.md` §7), REUSING every landed combat-math
/// piece verbatim (it reimplements none):
///
/// 1. **Part roll** — [`roll_body_part`] picks the struck [`BodyPart`](crate::armor::BodyPart)
///    from `tuning.body_part_weights`, drawing from `shot_rng` (the §4 location roll).
/// 2. **Opposed Fight** — [`opposed_fight`] (GTW-506) runs the §7 roll, two `fight_rng` draws
///    → a [`FightOutcome`](crate::melee::FightOutcome) (connect + margin).
/// 3. **Miss** — when the opposed roll is LOST (`!connect`) the verb returns
///    a no-effect (no-connect) [`MeleeStrike`] immediately: NO damage, NO `severity_rng` draw
///    (a clean miss).
/// 4. **Damage + multiply** — on a connect, [`resolve_hit`] (§5) resolves the per-hit
///    damage against the struck piece (or bare flesh), then [`apply_melee_multiplier`]
///    (GTW-506, via [`melee_damage_mult`]) scales the [`HitResult`](crate::resolve_hit::HitResult)
///    by the §7 margin multiplier (so a dominant blow reaches a worse wound bucket).
/// 5. **Severity** — [`roll_severity`] (§6) buckets the scaled penetrating damage, drawing
///    the ONE `severity_rng` term (the same §6 inputs `fold_ganger` assembles: both Lucks,
///    the defender's Toughness, the struck part's mod, the weapon's fatal bias).
/// 6. **Apply** — [`apply_hit`] (§6) folds HP loss + Wounds-by-tier + armor wear + the
///    terminal gates onto `target` in place, wearing the struck piece's `&mut ArmorIntegrity`.
///
/// Returns the frozen [`MeleeStrike`] (the connect verdict + the rolled severity). It is
/// deterministic for a fixed seed + inputs (the three injected streams are the only entropy)
/// and owns no mutation after return beyond the in-place fold onto `target`. A corpse target
/// is handled by [`apply_hit`]'s own corpse-skip (it mutates nothing); the owning dispatcher's
/// alive-gate already excludes a dead target before this is called.
///
/// # Draw-stream discipline
///
/// On a **connect** it draws exactly: the §4 part roll (`shot_rng`), the two §7 opposed-Fight
/// rolls (`fight_rng`), and the ONE §6 severity term (`severity_rng`). On a **miss** it draws
/// the §4 part roll + the two §7 rolls but NOT the severity term — the §7 `connect` gate
/// short-circuits before §5/§6. (The part roll is taken up front, mirroring the ranged
/// pipeline where `roll_body_part` runs inside the march before the wound fold.)
#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "the §7 synthesis composes the irreducible input set: the two combatants' Fight + \
              attacker Luck (Combatants), the wielded weapon's §5 stats (MeleeWeaponHit), the \
              target's §6 fold surfaces (TargetGanger) + its entity, the tuning, and the three \
              draw streams the §4 / §7 / §6 steps each advance — already grouped where cohesive \
              (the resolve_and_apply argument-count precedent); the streams are distinct types"
)]
pub fn resolve_melee_strike(
    combatants: Combatants,
    weapon: MeleeWeaponHit<'_>,
    target: TargetGanger<'_>,
    target_entity: bevy::prelude::Entity,
    tuning: &CombatTuning,
    fight_rng: &mut FightRng,
    shot_rng: &mut ShotRng,
    severity_rng: &mut SeverityRng,
) -> MeleeStrike {
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

    // (3) Miss — the opposed roll was lost: no damage, no severity draw (a clean miss).
    if !outcome.connect {
        return MeleeStrike::MISS;
    }

    // (4) §5 damage → §7 multiply. Resolve the per-hit damage against the struck piece (or
    // bare flesh), then scale it by the §7 margin multiplier (GTW-506) so a dominant blow
    // reaches a worse wound bucket — the whole point of §7.
    let (piece, resolved_matchup) = melee_struck_piece(target.piece.as_ref(), weapon);
    let raw_hit = resolve_hit(
        *weapon.damage,
        *weapon.punch,
        *weapon.shred,
        &piece,
        resolved_matchup,
        tuning,
    );
    let mult = melee_damage_mult(outcome.margin, &tuning.melee);
    let hit = apply_melee_multiplier(raw_hit, mult);

    // (5) §6 severity — the ONE SeverityRng draw, over the SCALED penetrating damage so the
    // multiplied blow can reach a worse bucket. Same input assembly as `fold_ganger`.
    let inputs = SeverityInputs::new(
        hit.penetrating,
        target.toughness,
        part_severity_mod(part),
        *weapon.fatal_bias,
        combatants.attacker_luck,
        target.luck,
    );
    let severity = roll_severity(&inputs, &tuning.severity_scaling, severity_rng);

    // (6) §6 apply — fold HP loss + Wounds-by-tier + armor wear + the terminal gates onto the
    // target in place. The wear outcome is SURFACED on the verdict (GTW-572 — the owning
    // dispatcher emits the ArmorBroken fact on a Broke crossing, mirroring the ranged
    // bridge); the struck part drives both the §6 wound record and the armor-piece wear.
    let wear: ArmorWearOutcome = apply_hit(
        GangerHitTarget {
            hp:        target.hp,
            wounds:    target.wounds,
            life:      target.life,
            integrity: target.piece.map(|p| p.integrity),
            inflicted: target.inflicted,
        },
        &hit,
        severity,
        part,
        target_entity,
        tuning,
    );

    MeleeStrike {
        connect: true,
        severity,
        // Surface the SCALED applied HP loss (GTW-572) — the same number `apply_hit` folded
        // onto the target — so the owning dispatcher's MeleeStruck fact can carry it.
        hp_damage: hit.hp_damage,
        wear,
    }
}
