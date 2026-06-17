//! The public query shapes and argument bundles [`fire`](super::fire) reads the
//! firing act through — the two disjoint Bevy queries plus the `Copy` borrow-record
//! arg bundles that keep [`fire`](super::fire)'s signature under clippy's
//! argument-count gate.

use bevy::ecs::{query::With, system::Query};

use crate::{
    cover::CoverLedger,
    ganger::{
        Aiming, Facing, Hp, LifeState, Luck, Position, Shooting, Stance, Toughness, Tu, TuMax,
        Wounds,
    },
    inflicted_wound::InflictedWounds,
    magazine::Magazine,
    metric::{Cell, Level},
    occupancy::OccupancyGrid,
    surface::SurfaceGrid,
    weapon::{
        Accuracy, BaseSpread, DamageType, FatalBias, FireModeSpec, Kickback, MagazineSize, Stable,
        Weapon, WeaponDamage, WeaponPunch, WeaponShred,
    },
};

/// The **shooter query shape** [`fire`](super::fire) reads the firing entity through
/// — the shooter's ganger state plus each GTW-200 weapon-stat
/// [`Component`](bevy::prelude::Component), `With<Weapon>`.
///
/// A type alias for the wide read+mutate tuple so [`fire`](super::fire)'s signature
/// stays readable. It is mutable on [`Tu`] (the up-front TU charge) and [`Magazine`]
/// (the per-round decrement) and read-only on everything else; it carries **no**
/// [`LifeState`] — the shooter's liveness is read from the [`TargetQuery`] (whose
/// `&mut LifeState` would clash with a `&LifeState` here). Every other field is the
/// read state [`cone_for`](crate::aim::cone_for) /
/// [`stability_for`](crate::aim::stability_for) /
/// [`concentration_p`](crate::sample_cone::concentration_p) /
/// [`mode_tu_cost`](crate::magazine::mode_tu_cost) /
/// [`resolve_coarse`](crate::resolve_coarse::resolve_coarse) consume, assembled into
/// the borrow-views at the call site.
pub type ShooterQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        // Ganger state the cone / concentration / muzzle composition reads.
        (
            &'static Position,
            &'static Facing,
            &'static Stance,
            &'static Aiming,
            &'static Shooting,
            &'static Luck,
            &'static TuMax,
        ),
        // The mutable firing economy — the up-front TU charge + per-round ammo.
        (&'static mut Tu, &'static mut Magazine),
        // The GTW-200 weapon-stat components the WeaponStats view borrows.
        (
            &'static BaseSpread,
            &'static Accuracy,
            &'static Kickback,
            &'static FatalBias,
            &'static WeaponDamage,
            &'static WeaponPunch,
            &'static WeaponShred,
            &'static DamageType,
            &'static MagazineSize,
            &'static Stable,
        ),
    ),
    With<Weapon>,
>;

/// The **target query shape** [`fire`](super::fire) folds a hit onto — the struck
/// ganger's four mutable battle surfaces plus the two read attribute stats the
/// severity roll needs.
///
/// A type alias for the disjoint mutable set so [`fire`](super::fire)'s signature
/// stays readable. It shares **no** mutable component with [`ShooterQuery`] (the
/// shooter writes [`Tu`] / [`Magazine`]; the target writes [`Hp`] / [`Wounds`] /
/// [`LifeState`] / [`WornArmor`](crate::armor::WornArmor) /
/// [`InflictedWounds`]), and [`Luck`] is read-only in both — so the two queries
/// coexist with no `B0001` conflict (AC1). It carries no [`Weapon`] filter (a target
/// need not be armed). The shooter's own liveness is read through this query too
/// (`targets.get(shooter)`), since the shooter is also a ganger.
pub type TargetQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        &'static mut Hp,
        &'static mut Wounds,
        &'static mut LifeState,
        &'static mut crate::armor::WornArmor,
        &'static mut InflictedWounds,
        &'static Toughness,
        &'static Luck,
    ),
>;

/// The **change-driven world grids** [`fire`](super::fire) marches each round through
/// — bundled into one named ref-struct so [`fire`](super::fire) stays under clippy's
/// argument-count gate.
///
/// The three grids [`resolve_coarse`](crate::resolve_coarse::resolve_coarse) reads
/// (never rebuilds — the change-driven contract; the change-driven sim↔app seam
/// recorded in ADR-0001, `docs/decisions/0001-rust-bevy-rewrite.md`): the coarse
/// [`OccupancyGrid`], the persistent [`SurfaceGrid`], and the model [`CoverLedger`].
/// Grouping the cohesive world-state refs into one value (the
/// [`ShotInputs`](crate::resolve_coarse::ShotInputs) /
/// [`TargetGanger`](crate::resolve_and_apply::TargetGanger) bundle precedent) keeps
/// [`fire`](super::fire)'s parameter list under the 8-arg gate. The struct is a
/// transparent borrow record, not itself a wrapped domain scalar; every field is an
/// existing named world-state type (no bare primitive).
#[derive(Debug, Clone, Copy)]
pub struct BattleGrids<'a> {
    /// The coarse 3D occupancy grid — the march's collision / occupant-band surface.
    pub occupancy: &'a OccupancyGrid,
    /// The persistent floor/roof-slab + ground surface grid the march flies through.
    pub surface:   &'a SurfaceGrid,
    /// The model cover ledger — peeked for the faced cell (stability) and the target
    /// cell's cover band (the aim point).
    pub cover:     &'a CoverLedger,
}

/// The **firing order** — what the shooter is firing and where
/// (`docs/combat/resolution.md` §1: the per-action selected fire mode + the aim cell).
///
/// The act args bundled into one named value (the
/// [`ShotInputs`](crate::resolve_coarse::ShotInputs) / [`BattleGrids`] bundle
/// precedent) so [`fire`](super::fire) stays under clippy's argument-count gate: the
/// selected [`FireModeSpec`], the aim [`Cell`], and the aim [`Level`]. A transparent
/// argument record, not itself a wrapped domain scalar; every field is an existing
/// named domain type.
#[derive(Debug, Clone, Copy)]
pub struct FireOrder<'a> {
    /// The selected fire mode's per-mode numbers (cone mult / TU% / shot count).
    pub mode:         &'a FireModeSpec,
    /// The target cell the player aimed at (the §2 aim cell's x/y).
    pub target_cell:  Cell,
    /// The target storey the player aimed at (the aim cell's z).
    pub target_level: Level,
}
