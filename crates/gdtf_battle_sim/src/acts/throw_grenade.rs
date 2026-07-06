//! The **throw-grenade** dispatch system (GTW-546, child GTW-41d of GTW-41) —
//! [`dispatch_throw_grenade`], which drains the buffered [`ThrowGrenadeRequested`] and, per
//! message, LOBS one [`TrajectoryStyle::Arc`](crate::weapon::TrajectoryStyle) grenade at a
//! target cell by RE-GATING in the sim and REUSING the deterministic arc march
//! ([`march_arc`](crate::march::march_arc)) + the GTW-541 blast resolver
//! ([`resolve_blast`](crate::fire::resolve_blast)).
//!
//! This MIRRORS the shove / enter-emplacement dispatch precedents — the deliberate act's
//! input-seam offer is advisory, so the sim RE-GATES before acting, spends a TU leaf, and
//! resolves the effect. The throw differs in ONE way: it is a BLIND lob, so there is NO
//! line-of-sight / facing / arc gate (unlike ranged fire's `decide_fire_arc`) — a grenade may
//! be thrown over walls at an unseen cell (`docs/combat/combat.md` names lobbed grenades among
//! the advanced-effect weapons). The gate is: the thrower exists + wields an `Arc` weapon with
//! a loaded round + affords the [`ThrowTu`](crate::tuning::ThrowTu) leaf.
//!
//! On pass it spends the [`ThrowTu`] leaf + one magazine round, marches the arc to its landing
//! (blocked by an intact roof, passing holes / windows), fans the weapon's
//! [`HitType::Blast`](crate::weapon::HitType) at the landing through the SAME GTW-541
//! `resolve_and_apply` fold the fire path uses, and emits a [`ThrowResolved`] presenter signal
//! at the landing cell. The arc geometry draws NO RNG (deterministic); the blast's per-ganger
//! wound rolls use the existing model streams. Param-only (`bevy-traps.md` #7 — no
//! `&mut World`); fail-closed on missing components / resources.

use bevy::{
    ecs::query::With,
    prelude::{MessageReader, MessageWriter, Query, Res, ResMut},
};

use crate::{
    acts::request::{ThrowGrenadeRequested, ThrowResolved},
    fire::{BattleGrids, MeleeQuery, PieceQuery, TargetQuery, WearsQuery, resolve_blast},
    ganger::{Luck, Position, Tu},
    injuries::{InjuryRegistry, InjuryTables},
    magazine::Magazine,
    march::{MarchResult, march_arc},
    rng::{InjuryRng, SeverityRng, ShotRng},
    slab::{BraceStairCells, SlabLedger},
    surface::SurfaceGrid,
    tu::spend_tu,
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageType, DotProfile, FatalBias, FireMode, HitType, Kickback,
        Stable, TrajectoryStyle, WeaponBraceBonus, WeaponDamage, WeaponPunch, WeaponShred,
        WeaponStats, WieldedBy, Wields,
    },
};

/// The **throwable-weapon query** [`dispatch_throw_grenade`] reads the grenade's stats +
/// trajectory + fire-mode + magazine through — the weapon entity's stat columns,
/// `With<`[`WieldedBy`]`>` (the [`WeaponQuery`](crate::fire::WeaponQuery) shape, plus the
/// [`TrajectoryStyle`] + [`FireMode`] the throw reads and MINUS the fire-only columns it does
/// not).
///
/// Read-only on the stat columns; the [`Magazine`] is mutable (the throw spends one round).
/// Operates on the **weapon entities** (disjoint from the ganger entities of the thrower /
/// target queries), so it never conflicts with the ganger-state mutation.
type ThrowWeaponQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        &'static BaseSpread,
        &'static Accuracy,
        &'static Kickback,
        &'static FatalBias,
        &'static WeaponDamage,
        &'static WeaponPunch,
        &'static WeaponShred,
        &'static DamageType,
        &'static Stable,
        // GTW-549: the weapon's optional per-item WeaponBraceBonus attachment (SUPERSEDES the
        // GTW-542 Scoped / WeaponSightBonus columns — a sight now boosts AIM, not stability).
        Option<&'static WeaponBraceBonus>,
        Option<&'static DotProfile>,
        &'static TrajectoryStyle,
        &'static FireMode,
        &'static mut Magazine,
    ),
    With<WieldedBy>,
>;

/// The thrower's read state + the injury-roll resources, grouped so the dispatch stays under
/// Bevy's 16-param limit — the thrower's `(&Position, &Luck, &mut Tu)` plus the OPTIONAL
/// battle-lifetime tuning + injury tables/registry.
///
/// A Bevy [`SystemParam`](bevy::ecs::system::SystemParam) bundle: the thrower query is
/// mutable on [`Tu`] (the up-front TU charge) and read-only on `Position` / `Luck`. The tuning
/// and injury reads are `Option<Res>` so a focused harness that opens a battle without the Load
/// flow fails closed (never a panic — `bevy-traps.md` #1).
#[derive(bevy::ecs::system::SystemParam)]
pub struct ThrowActor<'w, 's> {
    /// The thrower's `(&Position, &Luck, &mut Tu)` — the arc origin, the §6 nasty-wound term
    /// the blast fold reads, and the TU pool the throw charges.
    pub thrower:  Query<'w, 's, (&'static Position, &'static Luck, &'static mut Tu)>,
    /// The battle-lifetime combat tuning the arc march + throw-TU cost read (`Option<Res>`, so
    /// a battle opened without it throws nothing — fail-closed).
    pub tuning:   Option<Res<'w, CombatTuning>>,
    /// The Load-owned injury tables the blast fold rolls against (`Option<Res>` — an absent
    /// table folds to no injury but still takes the aligned draw).
    pub tables:   Option<Res<'w, InjuryTables>>,
    /// The Load-owned injury registry the blast fold resolves names through (`Option<Res>`).
    pub registry: Option<Res<'w, InjuryRegistry>>,
}

/// The three battle-lifetime grid resources + the RNG streams the blast fold reads, grouped so
/// the dispatch stays under Bevy's 16-param limit.
///
/// A Bevy [`SystemParam`](bevy::ecs::system::SystemParam) bundle mirroring the fire path's
/// grid + RNG set: the read [`OccupancyGrid`](crate::occupancy::OccupancyGrid) /
/// [`SurfaceGrid`] + the mutable [`SlabLedger`] / cover ledger the blast fold spends, plus the
/// three distinct injected RNG streams (shot / severity / injury) so the same seed reproduces
/// a byte-equal blast. `'w`-only (every field is a `Res`/`ResMut`, none a `Query`), the
/// [`BattleGridsParam`](crate::acts::BattleGridsParam) precedent.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ThrowWorld<'w> {
    /// The change-driven occupancy grid the arc march + blast fold read (occupant peeks).
    pub occupancy:    Res<'w, crate::occupancy::OccupancyGrid>,
    /// The persistent floor/roof-slab + ground surface grid the arc march flies through.
    pub surface:      Res<'w, SurfaceGrid>,
    /// The model cover ledger — spent by the blast fold on a cover hit.
    pub cover:        ResMut<'w, crate::cover::CoverLedger>,
    /// The model slab ledger — spent by the blast fold on a slab hit.
    pub slab:         ResMut<'w, SlabLedger>,
    /// The brace-stair-cell set the [`BattleGrids`] bundle carries (unused by the throw's
    /// blast, but part of the shared grid bundle shape).
    pub brace_cells:  Res<'w, BraceStairCells>,
    /// The shot RNG stream — the blast's per-ganger §4 body-part roll.
    pub shot_rng:     ResMut<'w, ShotRng>,
    /// The severity RNG stream — the blast fold's §6 severity roll.
    pub severity_rng: ResMut<'w, SeverityRng>,
    /// The injury RNG stream — the blast fold's injury roll.
    pub injury_rng:   ResMut<'w, InjuryRng>,
}

/// The grenade weapon's OWNED stat snapshot — the copied-out weapon numbers the blast fold
/// reads, plus the fired mode's [`HitType`] template, taken off the weapon row BEFORE the
/// magazine's `&mut` re-borrow ends it (so the assembled [`WeaponStats`] borrow-view references
/// only these owned locals, never the query row).
///
/// A call-site snapshot (the fire path's `ShooterSnapshot` precedent): every field is an owned
/// weapon-number newtype (no bare primitive), read out of the weapon query so the borrow ends
/// before `resolve_blast`'s disjoint queries run.
struct GrenadeStats {
    base_spread: BaseSpread,
    accuracy:    Accuracy,
    kickback:    Kickback,
    fatal_bias:  FatalBias,
    damage:      WeaponDamage,
    punch:       WeaponPunch,
    shred:       WeaponShred,
    damage_type: DamageType,
    stable:      Stable,
    brace_bonus: Option<WeaponBraceBonus>,
    dot:         Option<DotProfile>,
    hit_type:    HitType,
}

impl GrenadeStats {
    /// Assemble the transient [`WeaponStats`] borrow-view the blast fold reads — refs into this
    /// owned snapshot (the [`WeaponBundle::stats`](crate::weapon::WeaponBundle::stats) idiom).
    const fn stats(&self) -> WeaponStats<'_> {
        WeaponStats {
            base_spread: &self.base_spread,
            accuracy:    &self.accuracy,
            kickback:    &self.kickback,
            fatal_bias:  &self.fatal_bias,
            damage:      &self.damage,
            punch:       &self.punch,
            shred:       &self.shred,
            damage_type: &self.damage_type,
            stable:      &self.stable,
            brace_bonus: self.brace_bonus.as_ref(),
            dot:         self.dot.as_ref(),
        }
    }
}

/// **Dispatch** buffered [`ThrowGrenadeRequested`] messages — the deliberate THROW-GRENADE act
/// (GTW-546), resolved by RE-GATING in the sim then REUSING the arc march + GTW-541 blast.
///
/// For each request:
///
/// 1. **Gate the thrower + weapon.** Fetch the `thrower`'s `(&Position, &Luck, &mut Tu)` and
///    its wielded weapon entity's `(TrajectoryStyle, FireMode, DamageType, damage stats,
///    &mut Magazine)`. A missing thrower / weapon, a NON-`Arc` weapon, an EMPTY magazine, or a
///    thrower that cannot afford the [`ThrowTu`](crate::tuning::ThrowTu) leaf is a no-op
///    (fail-closed). There is NO line-of-sight / facing gate — the lob is BLIND.
/// 2. **Charge.** Spend the [`ThrowTu`](crate::tuning::ThrowTu) off the thrower's `&mut Tu`
///    (saturating) + one round off the weapon's [`Magazine`] (saturating).
/// 3. **Arc + blast.** March the deterministic parabola ([`march_arc`]) from the thrower cell
///    to the target — blocked by an intact roof, passing holes / windows — to its landing
///    cell, then fan the weapon's [`HitType::Blast`](crate::weapon::HitType) at the landing
///    through [`resolve_blast`](crate::fire::resolve_blast) (the SAME GTW-541 fold the fire
///    path uses). A non-`Blast` mode (a mis-authored grenade) still lands + fans the resolver's
///    singleton (the landing cell only).
/// 4. **Signal.** Emit a [`ThrowResolved`] at the landing with the weapon's [`DamageType`] (the
///    presenter's impact FX selector).
///
/// The thrower's `&mut Tu` (a ganger entity) + the weapon's `&mut Magazine` (a weapon entity) +
/// the target gangers (via [`TargetQuery`]) live on disjoint entities, so the queries never
/// conflict. Fail-closed: a missing tuning / component throws nothing (never a panic). The arc
/// geometry draws NO RNG; only the blast's per-ganger wound rolls advance the model streams.
/// Param-only (`bevy-traps.md` #7).
#[expect(
    clippy::too_many_arguments,
    reason = "the throw needs its request reader + the disjoint thrower-actor / weapon / \
              target / wears / pieces queries + the grouped world-grid & RNG bundle + the \
              ThrowResolved signal writer — each a distinct Bevy SystemParam the arc march + \
              blast fold reads; the actor + world bundles already group the resources to stay \
              under Bevy's 16-param limit"
)]
pub fn dispatch_throw_grenade(
    mut requests: MessageReader<ThrowGrenadeRequested>,
    mut actor: ThrowActor,
    mut weapons: ThrowWeaponQuery,
    wields: Query<&Wields>,
    // GTW-505 C5: the melee-weapon marker probe — the throw resolves the grenade through
    // `ranged_weapon` (EXCLUDING the ganger's melee weapon / fists), so a ganger wielding BOTH
    // never resolves the fists entity (which carries no TrajectoryStyle) as its "grenade".
    melee: MeleeQuery,
    mut targets: TargetQuery,
    wears: WearsQuery,
    mut pieces: PieceQuery,
    mut world: ThrowWorld,
    mut resolved: MessageWriter<ThrowResolved>,
) {
    // `bevy-traps.md` #1: fail closed on the REQUIRED battle-lifetime CombatTuning the throw-TU
    // cost + arc march read. A harness that opens a battle without it throws nothing (no panic).
    let Some(tuning) = actor.tuning.as_deref() else {
        return;
    };
    // Empty fallbacks for an asset-less harness (no Load flow → no InjuryTables/Registry), the
    // same fallback the fire path uses.
    let empty_tables = InjuryTables::default();
    let empty_registry = InjuryRegistry::default();
    let tables: &InjuryTables = actor.tables.as_deref().unwrap_or(&empty_tables);
    let registry: &InjuryRegistry = actor.registry.as_deref().unwrap_or(&empty_registry);

    for request in requests.read() {
        // (1a) Gate the thrower: it must exist (have Position + Luck + Tu).
        let Ok((thrower_pos, thrower_luck, mut thrower_tu)) =
            actor.thrower.get_mut(request.thrower)
        else {
            continue;
        };
        let thrower_cell = **thrower_pos;
        let thrower_luck = *thrower_luck;

        // (1b) Resolve the wielded RANGED weapon entity (the grenade), EXCLUDING the ganger's
        //      melee weapon (fists) — a thrower wielding no ranged weapon throws nothing. The
        //      grenade is a ranged-style weapon (carries the ranged stat columns +
        //      TrajectoryStyle), so `ranged_weapon` finds it; `weapon` (the first-related) could
        //      return the melee entity instead (GTW-505).
        let Some(weapon_entity) = wields
            .get(request.thrower)
            .ok()
            .and_then(|w| w.ranged_weapon(|entity| melee.get(entity).is_ok()))
        else {
            continue;
        };
        let Ok((
            base_spread,
            accuracy,
            kickback,
            fatal_bias,
            damage,
            punch,
            shred,
            damage_type,
            stable,
            brace_bonus,
            dot,
            trajectory,
            fire_mode,
            mut magazine,
        )) = weapons.get_mut(weapon_entity)
        else {
            continue;
        };

        // (1c) Gate the weapon: it must be an ARC weapon (a grenade / launcher) with a loaded
        //      round. A straight-firing weapon or an empty magazine is a no-op — the throw is
        //      blind, so there is NO line-of-sight / facing gate here (the one divergence from
        //      the ranged fire path's `decide_fire_arc`).
        if !trajectory.is_arc() || magazine.is_empty() {
            continue;
        }

        // (1d) Gate the economy: the thrower must afford the ThrowTu leaf.
        let cost = Tu::new(*tuning.throw_tu);
        if **thrower_tu < *cost {
            continue;
        }

        // Copy out the mode's blast template + the damage numbers BEFORE the magazine's
        // `&mut` re-borrow ends the weapon-row borrow, so the assembled WeaponStats below
        // references only these owned locals (no borrow overlaps the resolve_blast queries).
        let grenade = GrenadeStats {
            base_spread: *base_spread,
            accuracy:    *accuracy,
            kickback:    *kickback,
            fatal_bias:  *fatal_bias,
            damage:      *damage,
            punch:       *punch,
            shred:       *shred,
            damage_type: *damage_type,
            stable:      *stable,
            brace_bonus: brace_bonus.copied(),
            dot:         dot.copied(),
            hit_type:    fire_mode.single().hit_type,
        };

        // (2) Charge: spend the ThrowTu (saturating) off the thrower + one magazine round.
        spend_tu(&mut thrower_tu, cost);
        magazine.spend_round();

        // (3) March the deterministic arc to its landing (blocked by an intact roof, passing
        //     holes / windows), then fan the blast at the landing.
        let MarchResult { at: landing, .. } =
            march_arc(thrower_cell, request.target, &world.surface, tuning);
        let mut grids = BattleGrids {
            occupancy:   &world.occupancy,
            surface:     &world.surface,
            cover:       &mut world.cover,
            slab:        &mut world.slab,
            brace_cells: &world.brace_cells,
        };
        resolve_blast(
            landing,
            thrower_cell,
            grenade.stats(),
            grenade.hit_type,
            thrower_luck,
            &mut grids,
            &mut targets,
            &wears,
            &mut pieces,
            tuning,
            &mut world.shot_rng,
            &mut world.severity_rng,
            tables,
            registry,
            &mut world.injury_rng,
        );

        // (4) Signal the landing for the presenter's impact / blast FX (the seam/app phase draws
        //     it). The blast's HP/wound mutations are observed via change-detection.
        resolved.write(ThrowResolved::new(landing, grenade.damage_type));
    }
}
