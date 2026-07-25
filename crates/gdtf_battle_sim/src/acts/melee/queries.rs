//! The melee dispatch's query aliases + [`SystemParam`] bundles — the geometry /
//! target-surfaces / weapon-stat query shapes and the grouped world, draw streams,
//! and GTW-572 fact writers.

use bevy::{
    ecs::system::SystemParam,
    prelude::{MessageWriter, Query, Res, ResMut},
};

use crate::{
    acts::{InjuryInflicted, request::MeleeStruck},
    armor_wear::ArmorBroken,
    cover::CoverLedger,
    ganger::{Facing, Faction, Fight, Hp, LifeState, Luck, Position, Stance, Toughness, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::{InflictedInjuries, InjuryRegistry, InjuryTables},
    occupancy::OccupancyGrid,
    rng::{FightRng, InjuryRng, SeverityRng, ShotRng},
    surface::SurfaceGrid,
    tuning::CombatTuning,
    weapon::{DamageType, FatalBias, FightMode, Shove, WeaponDamage, WeaponPunch, WeaponShred},
};

/// The read-only geometry/stat snapshot query — every `Copy` read the gates + the §7 opposed
/// roll need off BOTH the attacker and the target ganger, factored into a `type` so
/// [`dispatch_melee`](super::dispatch::dispatch_melee) stays under clippy's type-complexity gate.
///
/// Reads `Position` / `Stance` / `Facing` / `Fight` / `Faction` / `Luck` — all IMMUTABLE, so
/// this query is disjoint from the mutable [`MeleeTargetQuery`] (which writes
/// `Hp`/`Wounds`/`LifeState`/`InflictedWounds`, a different mutable set; `Luck` is read-only in
/// both, which never conflicts) and the attacker's `&mut Tu` query — all coexist with no
/// `B0001` conflict. The system snapshots its reads as `Copy` values before any mutation, so
/// the gating reads (8-adjacency, LOS, faction, the two Fights) are taken once up front.
pub(super) type MeleeGeomQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        &'static Position,
        &'static Stance,
        &'static Facing,
        &'static Fight,
        &'static Faction,
        &'static Luck,
    ),
>;

/// The mutable TARGET surfaces query — the struck ganger's four `&mut` battle surfaces plus
/// the read attribute stats + its injury ledger the §6 fold needs, factored into a `type`
/// (the [`crate::fire::TargetQuery`] precedent).
///
/// Mutable on `Hp`/`Wounds`/`LifeState`/`InflictedWounds` (the [`apply_hit`](crate::apply_hit::apply_hit)
/// fold writes these) and read-only on `Toughness`/`Luck`/`InflictedInjuries` (the §6 severity
/// inputs). Disjoint from [`MeleeGeomQuery`] (no shared MUTABLE component) and the attacker's
/// `&mut Tu` query (a different mutable component), so no `ParamSet` is needed. Used via
/// `get_mut(target)` after the gates pass — the alive gate already read the target's snapshotted
/// `LifeState` from [`MeleeGeomQuery`], so this only takes the exclusive borrow once, at apply
/// time.
pub(super) type MeleeTargetQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        &'static mut Hp,
        &'static mut Wounds,
        &'static mut LifeState,
        &'static mut InflictedWounds,
        &'static Toughness,
        &'static Luck,
        Option<&'static InflictedInjuries>,
    ),
>;

/// The wielded MELEE-weapon stat query — the per-hit §5 damage columns, the §6 [`FatalBias`],
/// and the [`FightMode`] selector the dispatch reads off the related melee weapon entity,
/// factored into a `type` (the [`crate::fire::WeaponQuery`] precedent).
///
/// Read-only over the MELEE weapon entities (resolved `attacker → Wields → the MeleeWeapon
/// entity`), disjoint from every ganger-entity query above — so it coexists with no `B0001`
/// conflict. Assembled into a [`MeleeWeaponHit`](crate::melee::MeleeWeaponHit) borrow-view + read for the primary fight-mode
/// TU cost.
pub(super) type MeleeWeaponQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        &'static WeaponDamage,
        &'static WeaponPunch,
        &'static WeaponShred,
        &'static DamageType,
        &'static FatalBias,
        &'static FightMode,
        // GTW-525: the `shove` knockback tag — a connecting strike with it auto-shoves the target.
        &'static Shove,
    ),
>;

/// The change-driven world state the melee dispatch reads — the three grids the
/// [`has_los`](crate::los::has_los) gate marches, the [`CombatTuning`], and the §8 injury
/// content — bundled into one [`SystemParam`] so
/// [`dispatch_melee`](super::dispatch::dispatch_melee) stays under Bevy's 16-param system
/// limit (the [`crate::acts::BattleGridsParam`] grouping precedent).
///
/// The three grids the LOS march flies through ([`OccupancyGrid`] / [`SurfaceGrid`] /
/// [`CoverLedger`]) + the [`CombatTuning`] all the §4/§5/§6/§7 reads consume — every one a
/// battle-lifetime `Res<T>` (the band's `BattleInProgress` `run_if` keeps them present) — plus
/// the GTW-821 §8 injury content, taken `Option<Res<…>>` so an asset-less harness falls back
/// to EMPTY content instead of panicking. A transparent system-param bundle of named
/// world-state resources, not itself a wrapped domain scalar.
#[derive(SystemParam)]
pub struct MeleeWorld<'w> {
    /// The coarse 3D occupancy grid — the LOS march's collision / occupant-band surface (also
    /// the stair-eye-offset lookup for the observer eye).
    pub(super) occupancy: Res<'w, OccupancyGrid>,
    /// The persistent floor/roof-slab + ground surface grid the LOS march flies through.
    pub(super) surface:   Res<'w, SurfaceGrid>,
    /// The model cover ledger — peeked (read only) for the LOS march's cover bands on the
    /// ganger path, and SPENT (the depletion writer) on the GTW-508 cover-smash path. A
    /// `ResMut` because the structural strike arm calls
    /// [`CoverLedger::deplete_cover`](crate::cover::CoverLedger::deplete_cover) — the LOS march
    /// reads it through `&*` (a `ResMut` derefs to `&CoverLedger`), so the ganger path is
    /// unchanged.
    pub(super) cover:     ResMut<'w, CoverLedger>,
    /// The combat tuning the §4 body-part weights, §5 damage formula, §6 severity scaling, §7
    /// melee curve, and the LOS view geometry all read.
    pub(super) tuning:    Res<'w, CombatTuning>,
    /// The weighted `(category, context, severity)` injury tables — the §8 roll's pool; a
    /// connecting strike samples the [`Melee`](crate::injuries::DamageContext::Melee)
    /// per-source rows (GTW-821). `Option` — an asset-less harness (no `Load` flow) has none,
    /// and the strike then rolls against EMPTY content (the `dispatch_fire` /
    /// `dispatch_shove` fallback precedent), never a panic.
    pub(super) tables:    Option<Res<'w, InjuryTables>>,
    /// The injury registry — resolves the §8 roll's picked key to its authored def (GTW-821).
    /// `Option` for the same asset-less-harness reason as [`tables`](MeleeWorld::tables).
    pub(super) registry:  Option<Res<'w, InjuryRegistry>>,
}

/// The four seeded draw streams the §7 / §4 / §6 / §8 melee synthesis advances, bundled into
/// one [`SystemParam`] so [`dispatch_melee`](super::dispatch::dispatch_melee) stays under
/// Bevy's 16-param system limit.
///
/// Every field is a [`ResMut`] (drawing advances the cursor — never `Res`, the `rng::streams`
/// binding constraint): [`FightRng`] (the two §7 opposed-Fight rolls), [`ShotRng`] (the §4
/// body-part roll), [`SeverityRng`] (the §6 severity term), [`InjuryRng`] (the §8 injury term,
/// GTW-821). A transparent system-param bundle of the named stream resources, not itself a
/// wrapped domain scalar.
///
/// Each is taken `Option<ResMut<…>>` so a focused harness that opens `BattleInProgress` WITHOUT
/// the full setup flow (the fire/cover bridge tests insert only the streams `dispatch_fire`
/// needs) does not panic this runtime system on a stream's absence (`bevy-traps.md` #1; the
/// `reaction_trigger` `Option<ResMut<ReactionRng>>` precedent). With ANY of the four absent,
/// [`dispatch_melee`](super::dispatch::dispatch_melee) resolves no strike (a safe, defined
/// fallback — never a panic). In the real app all four are sim-set (inserted at
/// `setup_battle`), so the live melee act always has them.
#[derive(SystemParam)]
pub struct MeleeRngs<'w> {
    /// The §7 opposed-Fight stream — two draws per resolve (GTW-506).
    pub(super) fight:    Option<ResMut<'w, FightRng>>,
    /// The §4 body-part-roll stream — one draw per resolve.
    pub(super) shot:     Option<ResMut<'w, ShotRng>>,
    /// The §6 severity-roll stream — one draw per CONNECTING resolve (zero on a miss).
    pub(super) severity: Option<ResMut<'w, SeverityRng>>,
    /// The §8 injury-roll stream (GTW-821) — one draw per CONNECTING resolve whose §6 wound
    /// is non-graze / non-fatal (zero on a miss / graze / fatal).
    pub(super) injury:   Option<ResMut<'w, InjuryRng>>,
}

/// The melee FACT writers, bundled into one [`SystemParam`] so
/// [`dispatch_melee`](super::dispatch::dispatch_melee) stays under Bevy's 16-param system limit (the [`MeleeWorld`] /
/// [`MeleeRngs`] grouping precedent):
///
/// - [`MeleeStruck`] — the NUMBER-BEARING melee fact (attacker + target + applied HP loss)
///   the combat log's melee-damage line reads, one per CONNECTING ganger strike;
/// - [`ArmorBroken`] — the protecting→broken crossing the §6 fold surfaced on the
///   [`MeleeStrike`](crate::melee::MeleeStrike) verdict (GTW-572: previously computed and
///   dropped inside the verb), emitted so a melee break pops/logs exactly like a ranged one;
/// - [`InjuryInflicted`] — the EXISTING injury message (GTW-821), carrying the named injury
///   the connecting strike's §8 roll drew, so a melee injury reaches the target's ledger and
///   the battle report exactly as a ranged or fall one does.
///
/// A transparent system-param bundle of the named writers, not itself a wrapped domain value.
#[derive(SystemParam)]
pub struct MeleeFacts<'w> {
    /// The per-connecting-strike number-bearing fact (GTW-572).
    pub(super) struck:   MessageWriter<'w, MeleeStruck>,
    /// The armor-broken crossing a connecting strike's §6 wear surfaced (GTW-572).
    pub(super) breaks:   MessageWriter<'w, ArmorBroken>,
    /// The injury a connecting strike's §8 roll drew (GTW-821) — the EXISTING message the
    /// `apply_injury` boundary drains onto the target's ledger.
    pub(super) injuries: MessageWriter<'w, InjuryInflicted>,
}
