use bevy::prelude::{
    Changed, Entity, MessageReader, Or, Query, RemovedComponents, Res, ResMut, With,
};

use crate::{
    battle::{BattleReady, PlayerFaction},
    cover::CoverLedger,
    ganger::{Facing, Faction, LifeState, Position, Stance},
    los::PeekOffset,
    occupancy::{OccupancyGrid, VisionOccluderChanged},
    occupancy_sync::{CoverDestroyed, SlabDestroyed},
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

#[must_use]
pub fn should_recompute_visibility(
    moved: Query<&Faction, MovedReposedOrFlipped>,
    player: Option<Res<PlayerFaction>>,
    occluder_changed: Query<(), (With<TerrainCell>, VisionOccluderChanged)>,
    mut occluder_removed: RemovedComponents<BlocksVision>,
    mut cover_destroyed: MessageReader<CoverDestroyed>,
    mut slab_destroyed: MessageReader<SlabDestroyed>,
    mut ready: MessageReader<BattleReady>,
) -> bool {
    let cover_changed = cover_destroyed.read().count() > 0;
    let slab_changed = slab_destroyed.read().count() > 0;
    let battle_ready = ready.read().count() > 0;
    let occluder_added_or_retuned = !occluder_changed.is_empty();
    let occluder_removed = occluder_removed.read().count() > 0;
    let vision_occluder_changed = occluder_added_or_retuned || occluder_removed;
    let player_observer_changed = player
        .as_deref()
        .is_some_and(|player| moved.iter().any(|faction| *faction == **player));
    player_observer_changed
        || cover_changed
        || slab_changed
        || battle_ready
        || vision_occluder_changed
}

pub fn recompute_visibility(
    observers: Query<(&Position, &Stance, &Facing, &LifeState, &Faction)>,
    occupancy: Res<OccupancyGrid>,
    surface: Res<SurfaceGrid>,
    cover: Res<CoverLedger>,
    tuning: Res<CombatTuning>,
    player: Res<PlayerFaction>,
    mut squad: ResMut<SquadVisibility>,
) {
    let is_dead = |entity: Entity| {
        observers
            .get(entity)
            .is_ok_and(|(.., life, _)| *life == LifeState::Dead)
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
        is_dead,
    );
    *squad = accrue(&squad, visible_next);
}

#[cfg(test)]
mod test {
    use bevy::{
        ecs::system::RunSystemOnce as _,
        prelude::{Entity, Messages, World},
    };

    use crate::{
        battle::{BattleReady, PlayerFaction},
        ganger::{Faction, Position, Stance, StanceKind},
        los::PeekOffset,
        metric::{Cell, CellLevel, Level},
        occupancy_sync::{CoverDestroyed, SlabDestroyed},
        visibility::should_recompute_visibility,
    };

        const PLAYER: Faction = Faction::new(0);
        const ENEMY: Faction = Faction::new(1);

        fn ground(x: i32, y: i32) -> CellLevel {
        CellLevel::new(Cell::new(x, y), Level::new(0))
    }

                    fn run_predicate(world: &mut World) -> bool {
        world.init_resource::<Messages<CoverDestroyed>>();
        world.init_resource::<Messages<SlabDestroyed>>();
        world.init_resource::<Messages<BattleReady>>();
        world
            .run_system_once(should_recompute_visibility)
            .unwrap_or(false)
    }

    #[test]
    fn battle_ready_message_fires_the_gate() {
        let mut world = World::new();
        world.init_resource::<Messages<CoverDestroyed>>();
        world.init_resource::<Messages<SlabDestroyed>>();
        world.init_resource::<Messages<BattleReady>>();
        world
            .resource_mut::<Messages<BattleReady>>()
            .write(BattleReady);
        assert!(
            run_predicate(&mut world),
            "a buffered BattleReady must fire the recompute gate (spawn-FOV trigger)"
        );
    }

    #[test]
    fn cover_destroyed_message_fires_the_gate() {
        let mut world = World::new();
        world.init_resource::<Messages<CoverDestroyed>>();
        world.init_resource::<Messages<SlabDestroyed>>();
        world.init_resource::<Messages<BattleReady>>();
        world
            .resource_mut::<Messages<CoverDestroyed>>()
            .write(CoverDestroyed::new(ground(3, 3)));
        assert!(
            run_predicate(&mut world),
            "a buffered CoverDestroyed must fire the recompute gate"
        );
    }

    #[test]
    fn slab_destroyed_message_fires_the_gate() {
        let mut world = World::new();
        world.init_resource::<Messages<CoverDestroyed>>();
        world.init_resource::<Messages<SlabDestroyed>>();
        world.init_resource::<Messages<BattleReady>>();
        world
            .resource_mut::<Messages<SlabDestroyed>>()
            .write(SlabDestroyed::new(ground(3, 3)));
        assert!(
            run_predicate(&mut world),
            "a buffered SlabDestroyed must fire the recompute gate (the opened-hole sightline)"
        );
    }

    #[test]
    fn player_observer_move_fires_the_gate() {
        let mut world = World::new();
        world.insert_resource(PlayerFaction::new(PLAYER));
        world.spawn((
            Position::new(ground(5, 5)),
            Stance::new(StanceKind::Standing),
            PLAYER,
        ));
        assert!(
            run_predicate(&mut world),
            "a player-faction observer that moved/posed must fire the gate"
        );
    }

    #[test]
    fn enemy_only_move_does_not_fire_the_gate() {
        let mut world = World::new();
        world.insert_resource(PlayerFaction::new(PLAYER));
        world.spawn((
            Position::new(ground(9, 9)),
            Stance::new(StanceKind::Standing),
            ENEMY,
        ));
        assert!(
            !run_predicate(&mut world),
            "an enemy-only move must NOT fire the recompute gate"
        );
    }

    #[test]
    fn quiet_update_does_not_fire_the_gate() {
        let mut world = World::new();
        let _no_observer: Entity = world.spawn_empty().id();
        assert!(
            !run_predicate(&mut world),
            "a quiet update with no trigger must NOT fire the recompute gate"
        );
    }

    #[test]
    fn missing_player_faction_with_observer_does_not_fire() {
        let mut world = World::new();
        world.spawn((
            Position::new(ground(5, 5)),
            Stance::new(StanceKind::Standing),
            PLAYER,
        ));
        assert!(
            !run_predicate(&mut world),
            "with no PlayerFaction (no battle) the gate must stay closed, not panic"
        );
    }

                                #[test]
    fn player_peek_change_fires_the_gate() {
        let mut world = World::new();
        world.insert_resource(PlayerFaction::new(PLAYER));
        world.spawn((
            Position::new(ground(5, 5)),
            Stance::new(StanceKind::Standing),
            PeekOffset::default(),
            PLAYER,
        ));
        assert!(
            run_predicate(&mut world),
            "a player-faction observer with a Changed<PeekOffset> must fire the gate              (GTW-393 C3 trigger)"
        );
    }

                #[test]
    fn enemy_peek_change_does_not_fire() {
        let mut world = World::new();
        world.insert_resource(PlayerFaction::new(PLAYER));
        world.spawn((
            Position::new(ground(9, 9)),
            Stance::new(StanceKind::Standing),
            PeekOffset::default(),
            ENEMY,
        ));
        assert!(
            !run_predicate(&mut world),
            "an enemy-faction observer with a Changed<PeekOffset> must NOT fire the gate              (only player observers feed the squad fog — GTW-393 C3 faction filter)"
        );
    }
}
