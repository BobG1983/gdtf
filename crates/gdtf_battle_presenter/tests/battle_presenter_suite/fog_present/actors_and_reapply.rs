use bevy::prelude::Visibility;
use gdtf_battle_presenter::ActiveLevel;
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Direction, Faction, Level},
    surface::{SlabState, SurfaceGrid},
    test_support::SituationBuilder,
};

use super::harness::*;

#[test]
fn enemy_hard_cuts_player_always_shown() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let player_cell = CellLevel::new(Cell::new(5, 5), l0);
    let enemy_cell = CellLevel::new(Cell::new(20, 20), l0);

    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(player_cell, 0, Direction::East))
        .with_ganger(ganger_at(enemy_cell, 1, Direction::West))
        .player_faction(Faction::new(0))
        .build();
    assert!(drive_setup(&mut app, situation), "setup must complete");

    let player_sim = sim_entity_at(&mut app, player_cell);
    let enemy_sim = sim_entity_at(&mut app, enemy_cell);
    assert!(
        player_sim.is_some() && enemy_sim.is_some(),
        "both gangers must have spawned",
    );
    assert!(
        settle_actor(&mut app, player_sim) && settle_actor(&mut app, enemy_sim),
        "both ganger sprites must have materialized",
    );

    set_fog(&mut app, &[player_cell], &[]);
    app.update();

    assert_eq!(
        actor_visibility(&mut app, player_sim),
        Some(Visibility::Inherited),
        "a player ganger is always shown",
    );
    assert_eq!(
        actor_visibility(&mut app, enemy_sim),
        Some(Visibility::Hidden),
        "an enemy on a non-VISIBLE cell is hard-hidden (no ghost)",
    );

    set_fog(&mut app, &[player_cell, enemy_cell], &[]);
    app.update();

    assert_eq!(
        actor_visibility(&mut app, enemy_sim),
        Some(Visibility::Inherited),
        "an enemy on a VISIBLE cell shows",
    );
    assert_eq!(
        actor_visibility(&mut app, player_sim),
        Some(Visibility::Inherited),
        "the player ganger still shows",
    );
}

#[test]
fn fog_reapplies_after_level_cycle() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let l1 = Level::new(1);
    let unseen_l1 = CellLevel::new(Cell::new(8, 8), l1);
    let visible_l1 = CellLevel::new(Cell::new(9, 9), l1);

    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(
            CellLevel::new(Cell::new(5, 5), l0),
            0,
            Direction::East,
        ))
        .player_faction(Faction::new(0))
        .build();
    assert!(drive_setup(&mut app, situation), "setup must complete");
    assert!(
        settle_terrain_at(&mut app, CellLevel::new(Cell::new(5, 5), l0)),
        "the level-0 terrain field must have drawn",
    );

    {
        let mut surface = app.world_mut().resource_mut::<SurfaceGrid>();
        surface.set_slab(unseen_l1, SlabState::Present);
        surface.set_slab(visible_l1, SlabState::Present);
    }

    set_fog(&mut app, &[visible_l1], &[]);

    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(l1);
    assert!(
        settle_terrain_at(&mut app, unseen_l1),
        "after the level change, the level-1 terrain field must have respawned",
    );
    app.update();

    let visible_terrain = terrain_at(&mut app, visible_l1);
    assert!(
        visible_terrain.is_some(),
        "the level-1 VISIBLE cell must have a respawned tile + material"
    );
    let Some((vis_saturation, vis_flag)) = visible_terrain else {
        return;
    };
    assert!(
        (vis_saturation - 1.0).abs() < f32::EPSILON,
        "the freshly-respawned VISIBLE level-1 terrain is at full colour (saturation 1.0); \
         got {vis_saturation}",
    );
    assert_eq!(
        vis_flag,
        Visibility::Inherited,
        "VISIBLE level-1 terrain shown"
    );

    let unseen_terrain = terrain_at(&mut app, unseen_l1);
    assert!(
        unseen_terrain.is_some(),
        "the level-1 UNSEEN cell must have a respawned tile + material"
    );
    let Some((_s, unseen_flag)) = unseen_terrain else {
        return;
    };
    assert_eq!(
        unseen_flag,
        Visibility::Hidden,
        "the freshly-respawned UNSEEN level-1 terrain is hidden — the fog re-applied on the \
         same cycle (proves present_fog runs .after draw_static_battlefield)",
    );
}
