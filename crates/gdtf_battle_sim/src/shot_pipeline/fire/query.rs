//! The public query shapes and argument bundles [`fire`](super::fire) reads the
//! firing act through — the two disjoint Bevy queries plus the `Copy` borrow-record
//! arg bundles that keep [`fire`](super::fire)'s signature under clippy's
//! argument-count gate.

use bevy::ecs::{query::With, system::Query};

use crate::{
    armor::{PieceArmorMut, Wears, WornBy},
    cover::CoverLedger,
    ganger::{
        Aiming, Facing, Hp, LifeState, Luck, Position, Shooting, Stance, Toughness, Tu, TuMax,
        Wounds,
    },
    inflicted_wound::InflictedWounds,
    injuries::InflictedInjuries,
    magazine::Magazine,
    metric::{Cell, Level},
    occupancy::OccupancyGrid,
    slab::{BraceStairCells, SlabLedger},
    surface::SurfaceGrid,
    weapon::{
        Accuracy, BaseSpread, DamageType, FatalBias, FireModeSpec, Handedness, Kickback,
        MeleeWeapon, Stable, WeaponDamage, WeaponPunch, WeaponShred, WieldedBy, Wields,
    },
};

/// The **shooter query shape** [`fire`](super::fire) reads the firing ganger through —
/// the shooter's ganger state plus the mutable [`Tu`] up-front TU charge.
///
/// A type alias for the read+mutate tuple so [`fire`](super::fire)'s signature stays
/// readable. It is mutable on [`Tu`] (the up-front TU charge) and read-only on
/// everything else; it carries **no** [`LifeState`] — the shooter's liveness is read
/// from the [`TargetQuery`] (whose `&mut LifeState` would clash with a `&LifeState`
/// here). Every other field is the read state [`cone_for`](crate::aim::cone_for) /
/// [`stability_for`](crate::aim::stability_for) /
/// [`concentration_p`](crate::sample_cone::concentration_p) /
/// [`mode_tu_cost`](crate::magazine::mode_tu_cost) /
/// [`resolve_coarse`](crate::resolve_coarse::resolve_coarse) consume, assembled into
/// the borrow-views at the call site.
///
/// Since GTW-323 slice 2 (ADR-0004) this query NO LONGER carries the GTW-200 weapon-stat
/// components or the [`Magazine`] — those live on the related **weapon entity**, read +
/// worn through the disjoint [`WieldsQuery`] / [`WeaponQuery`] (`ganger → Wields → the
/// weapon entity`). The transient on-ganger weapon components still ride on the ganger
/// (removed in slice 3) but are no longer queried for the firing math. There is also no
/// longer a `With<Weapon>` filter here (the marker is on the weapon entity now); a
/// shooter that wields no weapon is caught by the [`WieldsQuery`] resolution failing,
/// not by the ganger row being absent.
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
        // The shooter's injury ledger (GTW-436), read OPTIONALLY: the shooter's Luck —
        // the §6 score's nasty-wound term — is read through
        // [`effective_luck`](crate::ganger::effective_luck) over this ledger, so a
        // `Modify(Luck)` injury shifts the wounds the shooter deals. `Option` because a
        // ganger need not carry the ledger (an absent ledger = the zero-delta identity).
        Option<&'static InflictedInjuries>,
        // The mutable firing economy — the up-front TU charge. The per-round ammo
        // decrement now lives on the weapon entity (the `WeaponQuery`'s `&mut Magazine`).
        &'static mut Tu,
    ),
>;

/// The **wielded-weapon relationship query** [`fire`](super::fire) resolves a shooter's
/// weapon entity through — read-only access to each ganger's [`Wields`] collection
/// (GTW-323 slice 2 / ADR-0004).
///
/// A type alias for the `Query<&Wields>` the fire path keys `ganger → Wields → the
/// weapon entity` with (mirroring the armor side's [`WearsQuery`]). Read-only and
/// disjoint from both the [`ShooterQuery`] (a different component on the ganger) and the
/// [`WeaponQuery`] (a different entity — the weapon) — so all coexist with no `B0001`
/// conflict.
pub type WieldsQuery<'world, 'state> = Query<'world, 'state, &'static Wields>;

/// The **melee-weapon marker probe** the ranged-firing path filters a wielded weapon
/// against (GTW-505 C5) — read-only `With<`[`MeleeWeapon`]`>` access over the weapon
/// entities, so `Wields::ranged_weapon(|e| melee.get(e).is_ok())` can EXCLUDE the
/// melee weapon a ganger also wields.
///
/// A type alias for `Query<(), With<MeleeWeapon>>`: it carries NO component data (the
/// unit `()` query item), only the archetype filter, so it is the cheapest possible
/// "is this entity a melee weapon?" probe. Read-only and disjoint from the
/// [`WeaponQuery`] (which filters `With<WieldedBy>` and reads the RANGED stat columns a
/// melee weapon lacks) and the [`WieldsQuery`] — so all coexist with no `B0001`
/// conflict. Without this filter, a ganger wielding BOTH a ranged and a melee weapon
/// could resolve the melee entity as its "weapon" and fire nothing (the zero-ranged-
/// regression mechanism).
pub type MeleeQuery<'world, 'state> = Query<'world, 'state, (), With<MeleeWeapon>>;

/// The **wielded-weapon query** [`fire`](super::fire) reads the GTW-200 weapon-stat
/// components + decrements the [`Magazine`] through — the weapon entity's stat columns,
/// `With<`[`WieldedBy`]`>` (GTW-323 slice 2 / ADR-0004).
///
/// A type alias for the weapon entity's read stats plus its **one mutable** [`Magazine`]
/// (the per-round ammo decrement). `fire()` assembles a transient
/// [`WeaponStats`](crate::weapon::WeaponStats) borrow-view from the read columns and
/// spends a round off the [`Magazine`] per fired iteration. Operates on the **weapon
/// entities** (disjoint from the ganger entities of [`ShooterQuery`] / [`TargetQuery`] /
/// [`WieldsQuery`]), so it never conflicts with the ganger-state mutation. The magazine
/// CAPACITY is the `size` leaf of this mutable [`Magazine`] grouping (GTW-275), not a
/// standalone component.
pub type WeaponQuery<'world, 'state> = Query<
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
        // GTW-443: the weapon's Handedness — read into the FireActor's can_fire gate
        // (a TwoHanded weapon is refused below two available hands).
        &'static Handedness,
        &'static mut Magazine,
    ),
    With<WieldedBy>,
>;

/// The **target query shape** [`fire`](super::fire) folds a hit onto — the struck
/// ganger's four mutable battle surfaces plus the two read attribute stats the
/// severity roll needs.
///
/// A type alias for the disjoint mutable set so [`fire`](super::fire)'s signature
/// stays readable. It shares **no** mutable component with [`ShooterQuery`] (the
/// shooter writes [`Tu`] / [`Magazine`]; the target writes [`Hp`] / [`Wounds`] /
/// [`LifeState`] / [`InflictedWounds`]), and [`Luck`] is read-only in both — so the
/// two queries coexist with no `B0001` conflict (AC1). It carries no
/// [`Weapon`](crate::equipment::weapon::Weapon) filter
/// (a target need not be armed). The shooter's own liveness is read through this query
/// too (`targets.get(shooter)`), since the shooter is also a ganger.
///
/// Since GTW-323 (ADR-0004) the struck location's armor is no longer a `&mut
/// WornArmor` column here — armor lives on related **piece entities** read through the
/// disjoint [`WearsQuery`] / [`PieceQuery`]. The transient ganger-side `WornArmor`
/// blob still rides on the entity (removed in slice 3) but is no longer queried for
/// combat.
pub type TargetQuery<'world, 'state> = Query<
    'world,
    'state,
    (
        &'static mut Hp,
        &'static mut Wounds,
        &'static mut LifeState,
        &'static mut InflictedWounds,
        &'static Toughness,
        &'static Luck,
        // The target's injury ledger (GTW-436), read OPTIONALLY: the defender's
        // Toughness (mitigation) + Luck (floor-extend) enter the §6 severity roll
        // through [`effective_toughness`](crate::ganger::effective_toughness) /
        // [`effective_luck`](crate::ganger::effective_luck) over this ledger, so a
        // `Modify(Toughness)` / `Modify(Luck)` injury shifts the roll. `Option` because a
        // ganger need not carry the ledger (an absent ledger = the zero-delta identity).
        Option<&'static InflictedInjuries>,
    ),
>;

/// The **worn-armor relationship query** [`fire`](super::fire) resolves a struck
/// ganger's worn pieces through — read-only access to each ganger's [`Wears`]
/// collection (GTW-323 / ADR-0004).
///
/// A type alias for the `Query<&Wears>` the fire path keys `ganger → Wears → the
/// BodyPart-tagged piece` with. Read-only and disjoint from both the mutable
/// [`TargetQuery`] (a different component on the ganger) and the [`PieceQuery`] (a
/// different set of entities) — so all three coexist with no `B0001` conflict.
pub type WearsQuery<'world, 'state> = Query<'world, 'state, &'static Wears>;

/// The **worn-armor-piece query** [`fire`](super::fire) reads + wears each struck
/// piece entity through — the mutable [`PieceArmorMut`] view, `With<`[`WornBy`]`>`
/// (GTW-323 / ADR-0004).
///
/// A type alias for `Query<PieceArmorMut, With<WornBy>>`: the `struck_piece` lookup
/// reads the four piece stats off it and `wear_armor` degrades its mutable
/// [`ArmorIntegrity`](crate::armor::ArmorIntegrity) in place. Operates on the **piece
/// entities** (disjoint from the ganger entities of [`TargetQuery`] / [`WearsQuery`]),
/// so it never conflicts with the ganger-state mutation.
pub type PieceQuery<'world, 'state> = Query<'world, 'state, PieceArmorMut, With<WornBy>>;

/// The **change-driven world grids** [`fire`](super::fire) marches each round through
/// — bundled into one named ref-struct so [`fire`](super::fire) stays under clippy's
/// argument-count gate.
///
/// The three grids [`resolve_coarse`](crate::resolve_coarse::resolve_coarse) reads
/// (never rebuilds — the change-driven contract; the change-driven sim↔app seam
/// recorded in ADR-0001, `docs/decisions/0001-rust-bevy-rewrite.md`): the coarse
/// [`OccupancyGrid`], the persistent [`SurfaceGrid`], and the model [`CoverLedger`].
/// The GTW-392 [`BraceStairCells`] resource is also bundled here — the lower-endpoint
/// stair-cell set the terrain-brace gate reads at fire time. Grouping the cohesive
/// world-state refs into one value (the
/// [`ShotInputs`](crate::resolve_coarse::ShotInputs) /
/// [`TargetGanger`](crate::resolve_and_apply::TargetGanger) bundle precedent) keeps
/// [`fire`](super::fire)'s parameter list under the 8-arg gate. The struct is a
/// transparent borrow record, not itself a wrapped domain scalar; every field is an
/// existing named world-state type (no bare primitive).
///
/// The [`cover`](BattleGrids::cover) ledger is held **mutably** (GTW-364): the march /
/// aim path still only *reads* it (`fire` reborrows it `&` for
/// [`resolve_coarse`](crate::resolve_coarse::resolve_coarse) / `cone_for` /
/// `stability_for` — the change-driven read contract is unchanged), but a round that
/// strikes cover SPENDS its HP through [`CoverLedger::deplete_cover`](crate::cover::CoverLedger::deplete_cover)
/// in `resolve_and_apply` — so the one ledger reference is `&mut`. The
/// [`slab`](BattleGrids::slab) ledger is held **mutably** for the SAME reason (GTW-365):
/// a round that strikes a slab spends its HP through
/// [`SlabLedger::deplete_slab`](crate::slab::SlabLedger::deplete_slab). The `&mut`
/// fields also drop the `Copy` derive, so [`BattleGrids`] is passed by `&mut` into the
/// per-round verb (reborrowed each round) rather than copied.
#[derive(Debug)]
pub struct BattleGrids<'a> {
    /// The coarse 3D occupancy grid — the march's collision / occupant-band surface.
    pub occupancy:   &'a OccupancyGrid,
    /// The persistent floor/roof-slab + ground surface grid the march flies through.
    pub surface:     &'a SurfaceGrid,
    /// The model cover ledger — peeked (read) for the faced cell (stability) and the
    /// target cell's cover band (the aim point), and **spent** (write) when a round
    /// strikes a piece of cover (GTW-364: the cover-hit depletion path).
    pub cover:       &'a mut CoverLedger,
    /// The model slab ledger — **spent** (write) when a round strikes a floor/roof slab
    /// (GTW-365: the slab-hit depletion path). The march reads slab *existence* from the
    /// [`SurfaceGrid`](crate::surface::SurfaceGrid) (above); this ledger holds the struck
    /// slab's HP/armor, lazily seeded from the `SlabDefaults` tuning leaf.
    pub slab:        &'a mut SlabLedger,
    /// The GTW-392 brace-stair-cell set — the lower-endpoint stair cells whose overhead
    /// slab grants the terrain brace. The terrain-brace gate reads this at fire time
    /// (see [`crate::shot_pipeline::stability::terrain_brace::terrain_braces`]).
    pub brace_cells: &'a BraceStairCells,
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
