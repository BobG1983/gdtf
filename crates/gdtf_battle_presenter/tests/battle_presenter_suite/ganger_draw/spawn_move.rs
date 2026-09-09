use bevy::{app::App, math::Vec2, prelude::Entity, time::TimeUpdateStrategy};
use cobalt_test_utils::advance_until_mut;
use gdtf_battle_presenter::{CELL_PX, GangerSprites, Layer, cell_to_world_layered};
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Direction, Faction, Level},
    test_support::SituationBuilder,
};

use super::{harness::*, probes::*};

fn settle_sprite_at(app: &mut App, entity: Entity, target: bevy::math::Vec3) {
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(500),
    ));
    advance_until_mut(app, |app| {
        sprite_translation(app, entity).is_some_and(|t| t.distance(target) < 1.0e-3)
    });
}

#[test]
fn added_gangers_spawn_one_faction_coloured_sprite_each() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let g0_at = CellLevel::new(Cell::new(5, 6), Level::new(0));
    let g1_at = CellLevel::new(Cell::new(12, 9), Level::new(0));
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(g0_at, 0, Direction::East))
        .with_ganger(ganger_at(g1_at, 1, Direction::North))
        .build();
    drive_setup(&mut app, situation);

    let roles = character_roles(&app);
    assert!(roles.is_some(), "CharacterRoles must be resident");
    let Some(roles) = roles else { return };

    assert_ne!(
        roles.base_for(Faction::new(0)),
        roles.base_for(Faction::new(1)),
        "the two factions must resolve to two distinct actor base indices",
    );

    let drawn = drawn_gangers(&mut app);
    assert_eq!(
        drawn.len(),
        2,
        "exactly two ganger sprites spawn (one per authored ganger), got {}",
        drawn.len(),
    );

    let g0_sim = sim_entity_at(&mut app, g0_at);
    let g1_sim = sim_entity_at(&mut app, g1_at);
    assert!(
        g0_sim.is_some() && g1_sim.is_some(),
        "both authored gangers must have spawned sim entities",
    );

    for d in &drawn {
        assert_eq!(
            d.custom_size,
            Some(Vec2::splat(CELL_PX)),
            "every ganger sprite must be custom_size Some(Vec2::splat(CELL_PX))",
        );
        let mirrors_authored = Some(d.sim_entity) == g0_sim || Some(d.sim_entity) == g1_sim;
        assert!(
            mirrors_authored,
            "every drawn sprite must mirror one of the two authored gangers",
        );
        if Some(d.sim_entity) == g0_sim {
            assert_eq!(
                d.atlas_index,
                Some(expected_index(&roles, 0, Direction::East)),
                "faction-0 ganger sprite index = faction_0 base + East(RIGHT) offset",
            );
            assert_eq!(
                d.translation,
                cell_to_world_layered(Cell::new(5, 6), Level::new(0), Layer::Actor),
                "faction-0 sprite at the Actor-layer projection of its authored Position",
            );
        } else if Some(d.sim_entity) == g1_sim {
            assert_eq!(
                d.atlas_index,
                Some(expected_index(&roles, 1, Direction::North)),
                "faction-1 ganger sprite index = faction_1 base + North(UP) offset",
            );
            assert_eq!(
                d.translation,
                cell_to_world_layered(Cell::new(12, 9), Level::new(0), Layer::Actor),
                "faction-1 sprite at the Actor-layer projection of its authored Position",
            );
        }
    }
}

#[test]
fn changed_position_moves_the_same_sprite() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let start = CellLevel::new(Cell::new(5, 6), Level::new(0));
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(start, 0, Direction::East))
        .build();
    drive_setup(&mut app, situation);

    let drawn_before = drawn_gangers(&mut app);
    assert_eq!(drawn_before.len(), 1, "one ganger sprite before the move");
    let sprite_before = drawn_before[0].sprite_entity;

    let sim = sim_entity_at(&mut app, start);
    assert!(sim.is_some(), "the ganger sim entity must exist");
    let Some(sim) = sim else { return };

    let mapped_before = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim));
    assert_eq!(
        mapped_before,
        Some(sprite_before),
        "the map links the sim ganger to its one sprite",
    );

    let dest = CellLevel::new(Cell::new(7, 8), Level::new(0));
    set_drawn_position(&mut app, sim, dest);
    let dest_world = cell_to_world_layered(Cell::new(7, 8), Level::new(0), Layer::Actor);
    settle_sprite_at(&mut app, sprite_before, dest_world);

    let drawn_after = drawn_gangers(&mut app);
    assert_eq!(
        drawn_after.len(),
        1,
        "still exactly one ganger sprite after the move (not respawned), got {}",
        drawn_after.len(),
    );
    assert_eq!(
        drawn_after[0].sprite_entity, sprite_before,
        "the move reuses the SAME presenter sprite entity",
    );
    assert!(
        drawn_after[0].translation.distance(dest_world) < 1.0e-3,
        "the sprite glided to the Actor-layer projection of the new Position (got {:?}, \
         expected {dest_world:?})",
        drawn_after[0].translation,
    );
    let mapped_after = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim));
    assert_eq!(
        mapped_after,
        Some(sprite_before),
        "the map still links the sim ganger to its one (moved) sprite",
    );
}
