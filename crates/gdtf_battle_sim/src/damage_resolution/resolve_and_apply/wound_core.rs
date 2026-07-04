//! The **attacker-agnostic wound-synthesis core** (GTW-523 remediation) —
//! [`synthesize_wound`], the ONE shared §5 → §6 → §8 fold that BOTH the weapon path's
//! ganger kind module ([`kinds::ganger`](super::kinds::ganger)) and the no-attacker
//! fall path ([`resolve_fall_hit`](crate::falls::resolve_fall_hit)) route through, so
//! the wound-math orchestration lives in exactly ONE place and the two paths cannot
//! drift (`docs/combat/resolution.md` §5 / §6 / §8).
//!
//! Both paths reduce to the SAME sequence once the per-path armor resolution is done:
//! `resolve_hit` → `roll_severity` (one [`SeverityRng`](crate::rng::SeverityRng) draw) →
//! `apply_hit` → `roll_injury` (one [`InjuryRng`](crate::rng::InjuryRng) draw, gated on
//! the rolled [`Severity`](crate::severity::Severity)). The ONLY thing the two callers
//! differ on is the INPUTS they synthesize:
//!
//! - the **weapon path** passes its real weapon-derived damage / punch / shred, the
//!   [`struck_piece`](super::fold)-resolved armored `(ArmorPiece, Matchup)`, the weapon's
//!   [`FatalBias`](crate::weapon::FatalBias), and the shooter's [`Luck`](crate::ganger::Luck);
//! - the **fall path** passes a synthetic no-attacker input set — `per_storey × storeys`
//!   as the damage, punch `0` / shred `0`, the worn-piece-or-bare-flesh `ArmorPiece` under
//!   [`Matchup::Neutral`](crate::matchup::Matchup::Neutral), `fatal_bias` `0`, and
//!   `shooter_luck` `0`.
//!
//! The core takes an already-resolved input BUNDLE (never a long argument list — no bare
//! types, every field a named domain newtype), performs the **corpse-skip INSIDE the
//! core** (returns [`None`] so a corpse yields no draw and no mutation), and on a live
//! target runs the four verbs and returns the [`WoundSynthesis`] verdict both callers
//! freeze into their own report shapes.

use bevy::prelude::Entity;

use crate::{
    apply_hit::{GangerHitTarget, apply_hit},
    armor::{ArmorPiece, BodyPart},
    armor_wear::ArmorWearOutcome,
    ganger::{LifeState, Luck},
    injuries::{InjuryRegistry, InjuryTables, RolledInjury, roll_injury},
    matchup::Matchup,
    resolve_and_apply::report::TargetGanger,
    resolve_hit::{HitResult, resolve_hit},
    rng::{InjuryRng, SeverityRng},
    severity::{Severity, SeverityInputs, part_severity_mod, roll_severity},
    tuning::CombatTuning,
    weapon::{FatalBias, WeaponDamage, WeaponPunch, WeaponShred},
};

/// The **already-resolved blow** a [`synthesize_wound`] call folds — the per-path inputs
/// after the caller has resolved its armor piece + matchup, so the core need not know
/// whether the blow came from a weapon or a fall.
///
/// A named bundle (no bare types, no long argument list): the struck [`BodyPart`], the
/// three damage-formula scalars, the resolved `(ArmorPiece, Matchup)` the hit lands
/// against, and the two §6 attacker-side terms. The weapon path fills these from the
/// weapon + [`struck_piece`](super::fold); the fall path fills them from its synthetic
/// no-attacker fork (see the module docs). Every field is a `Copy` domain newtype the
/// [`resolve_hit`] / [`roll_severity`] verbs consume directly.
pub(crate) struct WoundBlow {
    /// The struck body part — feeds the §6 [`part_severity_mod`] and the §8 injury table.
    pub part:         BodyPart,
    /// The raw weapon (or synthetic fall) HP damage before armor — fed to [`resolve_hit`].
    pub damage:       WeaponDamage,
    /// The armor-penetration term — `0` on a fall (a fall does not penetrate).
    pub punch:        WeaponPunch,
    /// The armor-shred term — `0` on a fall (a fall does not shred).
    pub shred:        WeaponShred,
    /// The already-resolved armored (or bare-flesh) [`ArmorPiece`] the hit lands against.
    pub piece:        ArmorPiece,
    /// The already-resolved weapon×armor [`Matchup`] — [`Matchup::Neutral`] on bare flesh
    /// or a fall (no wheel node).
    pub matchup:      Matchup,
    /// The weapon's fatal-bias severity push — `0` on a fall (no weapon).
    pub fatal_bias:   FatalBias,
    /// The **attacker's** Luck — the §6 nasty-wound term; `0` on a fall (no attacker).
    pub shooter_luck: Luck,
}

/// The **input bundle** of a [`synthesize_wound`] call — the already-resolved [`WoundBlow`],
/// the target ganger's mutable battle surfaces + read stats ([`TargetGanger`]), the target
/// [`Entity`], the shared combat [`CombatTuning`], and the injury content + the two seeded
/// draw streams the §6 / §8 rolls advance.
///
/// One named struct (no bare types, no 10-argument signature): grouping the borrows here is
/// what lets the core be a normal function while both callers stay under clippy's
/// argument-count gate. The two draw streams are distinct types ([`SeverityRng`] /
/// [`InjuryRng`]) so they can never be swapped; both are `&mut` (the core advances each by
/// exactly one draw on a live target). Lifetime `'a` ties the target/stream borrows to the
/// caller's frame.
pub(crate) struct WoundCoreInputs<'a> {
    /// The already-resolved blow (damage / armor / §6 attacker terms).
    pub blow:          WoundBlow,
    /// The target ganger's mutable battle surfaces + read attribute stats.
    pub target:        TargetGanger<'a>,
    /// The target's entity — passed through to [`apply_hit`] for its wear message address.
    pub target_entity: Entity,
    /// The shared combat tuning — the §6 severity scaling + wound costs + the §5 formula.
    pub tuning:        &'a CombatTuning,
    /// The §6 severity-roll stream — advanced by exactly ONE draw on a live target.
    pub severity_rng:  &'a mut SeverityRng,
    /// The shared weighted `(part, severity)` injury tables — the §8 roll's pool.
    pub tables:        &'a InjuryTables,
    /// The injury registry — resolves the §8 roll's picked key to its authored def.
    pub registry:      &'a InjuryRegistry,
    /// The §8 injury-roll stream — advanced by exactly ONE draw on a non-graze / non-fatal
    /// wound (zero draws on a graze / fatal — the §8 tabling rule).
    pub injury_rng:    &'a mut InjuryRng,
}

/// The **frozen verdict** of a [`synthesize_wound`] fold on a LIVE target — the resolved
/// damage, the rolled severity, the applied armor-wear outcome, the target's post-hit
/// [`LifeState`], and the rolled injury.
///
/// A `Copy`-where-possible record of named newtypes (the [`RolledInjury`] carries an owned
/// `Vec`, so the struct is `Clone` not `Copy`). Both callers build their own report from
/// these fields: the weapon path's ganger kind module assembles the
/// [`AppliedDamage`](super::kinds::ganger::AppliedDamage) (matchup / hit / severity /
/// life-after + [`wear`](WoundSynthesis::wear) carried DIRECTLY as the closed
/// [`ArmorWearOutcome`] — GTW-573 C2) inside its boxed
/// [`GangerVerdict`](super::kinds::ganger::GangerVerdict); the fall path takes only
/// [`injury`](WoundSynthesis::injury) to bridge into the existing `InjuryInflicted`
/// message. Returned inside a [`Some`] — a corpse-skip returns [`None`] (no draw, no
/// mutation, so nothing to freeze).
pub(crate) struct WoundSynthesis {
    /// The resolved weapon×armor [`Matchup`] the hit landed under (echoed back so the
    /// weapon path can freeze it into `AppliedDamage` without re-deriving it).
    pub matchup:    Matchup,
    /// The resolved per-hit damage / penetration / wear (§5).
    pub hit:        HitResult,
    /// The rolled wound severity — the ONE [`SeverityRng`] draw's bucket (§6).
    pub severity:   Severity,
    /// The per-hit armor-wear outcome (broke / damaged / unaffected) `apply_hit` produced (§6).
    pub wear:       ArmorWearOutcome,
    /// The target's [`LifeState`] AFTER the hit was applied (§6 terminal gates).
    pub life_after: LifeState,
    /// The rolled injury (§8) — `Some` only on a non-graze / non-fatal wound that rolled a
    /// named injury; `None` on a graze / fatal / empty table (each latter still took its
    /// one [`InjuryRng`] draw — content-independent stream alignment).
    pub injury:     Option<RolledInjury>,
}

/// Synthesize a wound from an already-resolved [`WoundCoreInputs`] bundle — the ONE shared
/// §5 → §6 → §8 fold both the weapon ganger path and the fall path route through
/// (`docs/combat/resolution.md` §5 / §6 / §8).
///
/// In order:
///
/// 1. **Corpse-skip — before any draw.** A target already at [`LifeState::Dead`] returns
///    [`None`] with NO severity draw, NO injury draw, and NO mutation — so a corpse never
///    consumes an RNG draw and determinism is preserved. This is the SAME corpse-skip both
///    callers used to run inline; it lives HERE now so it cannot drift.
/// 2. **Damage (§5).** [`resolve_hit`] over the blow's damage / punch / shred against the
///    already-resolved `(ArmorPiece, Matchup)`.
/// 3. **Severity — the ONE [`SeverityRng`] draw (§6).** [`roll_severity`] over the assembled
///    [`SeverityInputs`] (the hit's penetrating damage, the defender's [`Toughness`], the
///    struck part's [`part_severity_mod`], the blow's [`FatalBias`], and BOTH gangers'
///    [`Luck`] — the attacker's off the blow, the defender's off the target).
/// 4. **Apply (§6).** [`apply_hit`] folds HP loss + Wounds-by-tier + the inflicted-wound
///    record + armor wear + the terminal gates onto the target in place, returning the
///    [`ArmorWearOutcome`].
/// 5. **Injury — the ONE [`InjuryRng`] draw (§8).** [`roll_injury`], gated on the rolled
///    severity: a [`None`](Severity::None) (graze) / [`Fatal`](Severity::Fatal) takes NO
///    injury draw; a `Minor`/`Major`/`Critical` ALWAYS takes EXACTLY ONE (even on an
///    empty/missing table — content-independent stream alignment).
///
/// Returns [`Some`] the [`WoundSynthesis`] verdict on a live target (the target's HP /
/// Wounds / [`LifeState`] / armor integrity are mutated IN PLACE through the bundle), or
/// [`None`] on the corpse-skip. Pure given the two injected stream cursors — no `thread_rng`,
/// no ad-hoc entropy. Advances the [`SeverityRng`] by exactly one draw and the [`InjuryRng`]
/// by at most one (per the §8 gate) on a live target; advances NEITHER on the corpse-skip.
#[must_use]
pub(crate) fn synthesize_wound(inputs: WoundCoreInputs<'_>) -> Option<WoundSynthesis> {
    let WoundCoreInputs {
        blow,
        target,
        target_entity,
        tuning,
        severity_rng,
        tables,
        registry,
        injury_rng,
    } = inputs;

    // (1) Corpse-skip BEFORE any draw — a dead target is final (no draw, no mutation). This
    // is the shared corpse-skip both paths used to run inline; centralizing it here is what
    // guarantees the two paths cannot drift on the discipline.
    if *target.life == LifeState::Dead {
        return None;
    }

    // (2) The per-hit damage formula (§5) — pure, mutates nothing.
    let hit = resolve_hit(
        blow.damage,
        blow.punch,
        blow.shred,
        &blow.piece,
        blow.matchup,
        tuning,
    );

    // (3) The ONE severity draw (§6). Both gangers' Luck, the defender's Toughness, the
    // struck part's mod, and the blow's fatal bias feed it; the injected SeverityRng is the
    // draw point. The attacker's Luck is nil on a fall (no attacker), supplied by the caller.
    let severity = roll_severity(
        &SeverityInputs::new(
            hit.penetrating,
            target.toughness,
            part_severity_mod(blow.part),
            blow.fatal_bias,
            blow.shooter_luck,
            target.luck,
        ),
        &tuning.severity_scaling,
        severity_rng,
    );

    // (4) Apply the resolved hit onto the target in place (§6) — HP loss + Wounds-by-tier +
    // the inflicted-wound record + armor wear + the terminal gates. Capture the per-hit
    // ArmorWearOutcome (broke / damaged / unaffected) for the weapon path to freeze.
    let wear = apply_hit(
        GangerHitTarget {
            hp:        target.hp,
            wounds:    target.wounds,
            life:      target.life,
            // The struck piece's `&mut ArmorIntegrity` (None on bare flesh / no piece) —
            // `apply_hit`'s `wear_armor` degrades it in place (GTW-323 / ADR-0004).
            integrity: target.piece.map(|p| p.integrity),
            inflicted: target.inflicted,
        },
        &hit,
        severity,
        blow.part,
        target_entity,
        tuning,
    );

    // (5) The ONE injury draw (§8), AFTER apply_hit (Wounds already spent), gated on the
    // rolled severity: a graze / fatal takes NO draw, a Minor/Major/Critical ALWAYS takes
    // EXACTLY ONE (even on an empty/missing table — content-independent stream alignment).
    let injury = roll_injury(blow.part, severity, tables, registry, injury_rng);

    Some(WoundSynthesis {
        matchup: blow.matchup,
        hit,
        severity,
        wear,
        // The life state AFTER the hit was applied (the §6 terminal gates ran inside
        // apply_hit above), read back off the target for the report freeze.
        life_after: *target.life,
        injury,
    })
}
