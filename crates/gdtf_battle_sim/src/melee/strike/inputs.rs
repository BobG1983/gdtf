//! The input records one [`resolve_melee_strike`](super::resolve_melee_strike) call takes —
//! the wielded melee weapon's §5/§6 stats, the two combatants' §7 Fight terms, and the shared
//! world content + draw streams the §4 / §7 / §6 / §8 steps read.

use crate::{
    ganger::{Fight, Luck},
    injuries::{InjuryRegistry, InjuryTables},
    rng::{FightRng, InjuryRng, SeverityRng, ShotRng},
    tuning::CombatTuning,
    weapon::{DamageType, FatalBias, WeaponDamage, WeaponPunch, WeaponShred},
};

/// The wielded **melee weapon's** per-hit damage stats the strike resolves through — the
/// shared §5 weapon numbers a melee weapon carries (GTW-505: the damage group + the §6
/// [`FatalBias`]), bundled into one borrow-view so
/// [`resolve_melee_strike`](super::resolve_melee_strike) stays under clippy's
/// argument-count gate.
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
/// [`TargetGanger`](crate::resolve_and_apply::TargetGanger) the verb folds onto.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Combatants {
    /// The attacker's effective [`Fight`] — the §7 `Fight_attacker`.
    pub attacker_fight: Fight,
    /// The defender's effective [`Fight`] — the §7 `Fight_defender`.
    pub defender_fight: Fight,
    /// The attacker's [`Luck`] — the §6 score's shooter-Luck (nasty-wound push) term.
    pub attacker_luck:  Luck,
}

/// The shared combat + injury environment one strike reads — the [`CombatTuning`], the
/// injury [`InjuryTables`] / [`InjuryRegistry`] content, and the FOUR seeded draw streams
/// the §4 part roll / §7 opposed roll / §6 severity / §8 injury steps advance.
///
/// One named bundle (no bare types, no 11-argument signature) — the falls `FallWoundEnv`
/// precedent — so
/// [`resolve_melee_strike`](super::resolve_melee_strike) passes clippy's argument-count gate
/// with NO suppression. The streams are distinct types so they can never be swapped, and all
/// four are `&mut` (each is advanced by the steps that draw from it). Lifetime `'a` ties the
/// content reads + stream borrows to the caller's frame.
pub struct MeleeStrikeEnv<'a> {
    /// The shared combat tuning — the §4 part weights, §5 formula, §6 severity scaling, §7
    /// melee curve.
    pub tuning:       &'a CombatTuning,
    /// The shared weighted `(category, context, severity)` injury tables — the §8 roll's pool.
    pub tables:       &'a InjuryTables,
    /// The injury registry — resolves the §8 roll's picked key to its authored def.
    pub registry:     &'a InjuryRegistry,
    /// The §7 opposed-Fight stream — two draws per resolve.
    pub fight_rng:    &'a mut FightRng,
    /// The §4 body-part-roll stream — one draw per resolve.
    pub shot_rng:     &'a mut ShotRng,
    /// The §6 severity stream — one draw per CONNECTING resolve on a live target.
    pub severity_rng: &'a mut SeverityRng,
    /// The §8 injury stream — one draw per CONNECTING resolve whose wound is non-graze,
    /// non-fatal (zero draws otherwise — the §8 tabling rule).
    pub injury_rng:   &'a mut InjuryRng,
}
