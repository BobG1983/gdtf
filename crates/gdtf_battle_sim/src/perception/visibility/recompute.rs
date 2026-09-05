//! When and how to recompute player squad FOV.

use bevy::prelude::{
    Changed, Entity, MessageReader, Or, Query, RemovedComponents, Res, ResMut, With,
};

use crate::{
    battle::{BattleReady, PlayerFaction},
    cover::CoverLedger,
    ganger::{Facing, Faction, LifeState, Position, Stance},
    los::PeekOffset,
    occupancy::{OccupancyGrid, VisionOccluderChanged},
    occupancy_sync::TerrainPieceDestroyed,
    surface::SurfaceGrid,
    terrain::entity::{BlocksVision, TerrainCell},
    tuning::CombatTuning,
    visibility::{FovObserver, SquadVisibility, accrue, union_fov},
};

type MovedReposedOrFlipped = Or<(
    Changed<Position>,
    Changed<Stance>,
    Changed<LifeState>,
    Changed<PeekOffset>,
)>;

/// True when something that affects player FOV changed this frame.
#[must_use]
pub fn should_recompute_visibility(
    moved: Query<&Faction, MovedReposedOrFlipped>,
    player: Option<Res<PlayerFaction>>,
    occluder_changed: Query<(), (With<TerrainCell>, VisionOccluderChanged)>,
    mut occluder_removed: RemovedComponents<BlocksVision>,
    mut destroyed: MessageReader<TerrainPieceDestroyed>,
    mut ready: MessageReader<BattleReady>,
) -> bool {
    let terrain_changed = destroyed.read().count() > 0;
    let battle_ready = ready.read().count() > 0;
    let occluder_added_or_retuned = !occluder_changed.is_empty();
    let occluder_removed = occluder_removed.read().count() > 0;
    let vision_occluder_changed = occluder_added_or_retuned || occluder_removed;
    let player_observer_changed = player
        .as_deref()
        .is_some_and(|player| moved.iter().any(|faction| *faction == **player));
    player_observer_changed || terrain_changed || battle_ready || vision_occluder_changed
}

/// Recompute player squad visibility from all living player-faction observers.
pub fn recompute_visibility(
    observers: Query<(&Position, &Stance, &Facing, &LifeState, &Faction)>,
    occupancy: Res<OccupancyGrid>,
    surface: Res<SurfaceGrid>,
    cover: Res<CoverLedger>,
    tuning: Res<CombatTuning>,
    player: Res<PlayerFaction>,
    mut squad: ResMut<SquadVisibility>,
) {
    let is_floored = |entity: Entity| {
        observers
            .get(entity)
            .is_ok_and(|(.., life, _)| !*life.is_active())
    };

    let fov_observers: Vec<FovObserver> = observers
        .iter()
        .filter(|(.., faction)| **faction == **player)
        .map(|(position, stance, facing, life, _)| FovObserver {
            position,
            stance,
            facing,
            life: *life,
            stair_eye_offset: occupancy.stair_eye_offset_at(position),
        })
        .collect();

    let visible_next = union_fov(
        &fov_observers,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        is_floored,
    );
    *squad = accrue(&squad, visible_next);
}
