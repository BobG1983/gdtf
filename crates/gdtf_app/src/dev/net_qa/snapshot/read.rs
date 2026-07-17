//! The read-only world surface the snapshot builder projects (GTW-738, the T5 view
//! path).
//!
//! Two read-only bundles: the per-ganger [`GangerRow`] `QueryData` (the wide component
//! read, a `#[derive(QueryData)]` struct so it dodges the 16-tuple cap AND clippy
//! `type_complexity`), and the [`SnapshotWorld`] `SystemParam` that gathers every query
//! and battle-lifetime resource the whole [`BattleView`](gdtf_qa_protocol::view::BattleView)
//! is assembled from. Every borrow is SHARED — no field aliases another, so the bundle
//! never trips `B0001`. It reads only `gdtf_battle_sim`'s public `Query`/`Res` surface
//! (the presenter's read-only precedent) plus the input crate's `SelectedShooter`; it
//! never mutates the world.

use bevy::{
    ecs::{query::QueryData, system::SystemParam},
    prelude::*,
};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    battle::PlayerFaction,
    emplacement::EmplacementState,
    entity::TerrainCell,
    ganger::{
        Aiming, Facing, Faction, GangerName, Hp, HpMax, LifeState, Position, Stance, Tu, TuMax,
        Wounds, WoundsMax,
    },
    injuries::InflictedInjuries,
    openable::OpenState,
    turn::ActiveFaction,
    visibility::SquadVisibility,
    weapon::{FireMode, MeleeWeapon, WeaponName, Wields},
};

/// One ganger's queryable battle state, read in ONE `QueryData` struct.
///
/// The wide read the [`GangerView`](gdtf_qa_protocol::view::GangerView) projects from:
/// the entity id (minted into a [`GangerToken`](gdtf_qa_protocol::ids::GangerToken)), the
/// contract's required fields ([`position`](Self::position) / [`life`](Self::life) /
/// [`tu`](Self::tu) / [`hp`](Self::hp) / [`stance`](Self::stance) /
/// [`injuries`](Self::injuries)), the identity + posture context, the two pool ceilings,
/// and the [`wields`](Self::wields) relationship the weapon read keys through. A
/// `#[derive(QueryData)]` struct (the stat-block precedent) so it dodges the 16-tuple
/// `QueryData` cap and clippy `type_complexity`. Every field is a SHARED borrow.
#[derive(QueryData)]
pub(in crate::dev::net_qa) struct GangerRow {
    /// The ganger entity id — minted into the wire token.
    pub(super) entity:     Entity,
    /// The ganger's display name.
    pub(super) name:       &'static GangerName,
    /// The ganger's gang (faction) identity.
    pub(super) faction:    &'static Faction,
    /// The `(cell, level)` the ganger occupies.
    pub(super) position:   &'static Position,
    /// The direction the ganger faces.
    pub(super) facing:     &'static Facing,
    /// Whether the ganger is aiming.
    pub(super) aiming:     &'static Aiming,
    /// The ganger's posture.
    pub(super) stance:     &'static Stance,
    /// The ganger's terminal life state.
    pub(super) life:       &'static LifeState,
    /// The ganger's current hit points.
    pub(super) hp:         &'static Hp,
    /// The ganger's HP ceiling.
    pub(super) hp_max:     &'static HpMax,
    /// The ganger's current Wounds.
    pub(super) wounds:     &'static Wounds,
    /// The ganger's Wounds ceiling.
    pub(super) wounds_max: &'static WoundsMax,
    /// The ganger's current Time Units.
    pub(super) tu:         &'static Tu,
    /// The ganger's TU ceiling.
    pub(super) tu_max:     &'static TuMax,
    /// The ganger's durable injury ledger.
    pub(super) injuries:   &'static InflictedInjuries,
    /// The ganger's wielded-weapon relationship, if any (the weapon read keys through it).
    /// Read defensively as [`Option`] — the framework only inserts [`Wields`] once a weapon
    /// relates to the ganger (and a future unarmed ganger carries none), so an unarmed /
    /// not-yet-related ganger still projects a card (with an empty weapon view).
    pub(super) wields:     Option<&'static Wields>,
}

/// The whole read-only surface the [`build_snapshots`](super::build_snapshots) service
/// assembles a [`BattleView`](gdtf_qa_protocol::view::BattleView) from.
///
/// One `SystemParam` so the on-demand builder declares a single param (staying under the
/// argument-count ceiling). The battle-lifetime resources
/// ([`ActiveFaction`] / [`PlayerFaction`] / [`SquadVisibility`]) are inserted on the same
/// `setup_battle` `Ok` path as [`BattleInProgress`](gdtf_battle_sim::prelude::BattleInProgress)
/// and removed together on teardown, so — read under the builder's
/// `run_if(resource_exists::<BattleInProgress>)` gate — they are always present
/// (`bevy-traps.md` #1); [`SelectedShooter`] is `init_resource`-d by the input plugin for
/// the whole battle scene. Every field is a SHARED borrow, so no two alias (`B0001`-safe).
#[derive(SystemParam)]
pub(in crate::dev::net_qa) struct SnapshotWorld<'w, 's> {
    /// Every ganger on the field, read as the wide [`GangerRow`].
    pub(super) gangers:      Query<'w, 's, GangerRow>,
    /// The wielded-weapon name + fire-mode selector, read off the resolved ranged weapon
    /// entity (keyed through a ganger's [`Wields`]).
    pub(super) weapons:      Query<'w, 's, (&'static WeaponName, &'static FireMode)>,
    /// The melee-weapon marker probe — so the RANGED weapon resolves (the fire-mode list
    /// the client fires by mirrors what `NetIntent::Fire` validates against).
    pub(super) melee_marker: Query<'w, 's, (), With<MeleeWeapon>>,
    /// Every openable door — its entity (minted into a door token), cell, and open state.
    pub(super) doors:        Query<'w, 's, (Entity, &'static TerrainCell, &'static OpenState)>,
    /// Every weapon emplacement — its entity (minted into an emplacement token), cell, and
    /// occupancy.
    pub(super) emplacements:
        Query<'w, 's, (Entity, &'static TerrainCell, &'static EmplacementState)>,
    /// The player squad's fog-of-war sets.
    pub(super) fog:          Res<'w, SquadVisibility>,
    /// The current ganger selection.
    pub(super) selection:    Res<'w, SelectedShooter>,
    /// Whose turn it is.
    pub(super) active:       Res<'w, ActiveFaction>,
    /// Which faction the player controls.
    pub(super) player:       Res<'w, PlayerFaction>,
}
