use bevy::{
    app::App,
    prelude::{Entity, Visibility},
};
use gdtf_battle_presenter::{ActiveLevel, GangerSprites, Layer, cell_to_world_layered};
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Direction, Faction, Level, Position},
    test_support::SituationBuilder,
};

use super::{harness::*, probes::*};

fn translation_of_sim(app: &mut App, sim: Option<Entity>) -> Option<bevy::math::Vec3> {
    let sim = sim?;
    let sprite = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim))?;
    sprite_translation(app, sprite)
}

fn position_of_sim(app: &mut App, sim: Option<Entity>) -> Option<CellLevel> {
    let sim = sim?;
    let mut q = app.world_mut().query::<&Position>();
    q.get(app.world(), sim).ok().map(|p| **p)
}

#[test]
fn player_ganger_on_lower_storey_stays_visible_at_own_z_after_fog() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let l0_cell = Cell::new(5, 6);
    let l0_at = CellLevel::new(l0_cell, l0);
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(l0_at, 0, Direction::East))
        .player_faction(Faction::new(0))
        .slab_at(l0_at)
        .build();
    assert!(drive_setup(&mut app, situation), "setup must complete");

    let l0_sim = sim_entity_at(&mut app, l0_at);
    assert!(
        l0_sim.is_some(),
        "the ground-floor ganger must have spawned"
    );
    assert!(
        settle_actor(&mut app, l0_sim),
        "the ganger sprite must have materialized",
    );

    set_fog(&mut app, &[l0_at], &[]);
    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(Level::new(1));
    app.update();

    assert_eq!(
        visibility_of_sim(&mut app, l0_sim),
        Some(Visibility::Inherited),
        "a player ganger on a LOWER drawn storey (0) is shown at active level 1",
    );
    let expected_z = cell_to_world_layered(l0_cell, l0, Layer::Actor).z;
    let drawn_z = translation_of_sim(&mut app, l0_sim).map(|t| t.z);
    assert!(
        drawn_z.is_some_and(|z| (z - expected_z).abs() < 1.0e-3),
        "the lower-storey ganger draws at its OWN storey-0 Z ({expected_z}), got {drawn_z:?}",
    );
}

#[test]
fn ganger_above_active_is_hidden_after_fog() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let l2 = Level::new(2);
    let l0_at = CellLevel::new(Cell::new(5, 6), l0);
    let l2_at = CellLevel::new(Cell::new(7, 8), l2);
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(l0_at, 0, Direction::East))
        .with_ganger(ganger_at(l2_at, 0, Direction::West))
        .player_faction(Faction::new(0))
        .slab_at(l0_at)
        .slab_at(l2_at)
        .build();
    assert!(drive_setup(&mut app, situation), "setup must complete");

    let l2_sim = sim_entity_at(&mut app, l2_at);
    assert!(l2_sim.is_some(), "the storey-2 ganger must have spawned");
    assert!(
        settle_actor(&mut app, l2_sim),
        "the storey-2 ganger sprite must have materialized",
    );

    set_fog(&mut app, &[l0_at, l2_at], &[]);
    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(Level::new(1));
    app.update();

    assert_eq!(
        visibility_of_sim(&mut app, l2_sim),
        Some(Visibility::Hidden),
        "a ganger on storey 2 (strictly above active level 1) is hidden",
    );
}

#[test]
fn unseen_enemy_on_lower_storey_stays_hidden_fog_preserved() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let player_at = CellLevel::new(Cell::new(5, 6), l0);
    let enemy_at = CellLevel::new(Cell::new(20, 20), l0);
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(player_at, 0, Direction::East))
        .with_ganger(ganger_at(enemy_at, 1, Direction::West))
        .player_faction(Faction::new(0))
        .slab_at(player_at)
        .slab_at(enemy_at)
        .build();
    assert!(drive_setup(&mut app, situation), "setup must complete");

    let player_sim = sim_entity_at(&mut app, player_at);
    let enemy_sim = sim_entity_at(&mut app, enemy_at);
    assert!(
        player_sim.is_some() && enemy_sim.is_some(),
        "both gangers must have spawned",
    );
    assert!(
        settle_actor(&mut app, player_sim) && settle_actor(&mut app, enemy_sim),
        "both ganger sprites must have materialized",
    );

    set_fog(&mut app, &[player_at], &[]);
    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(Level::new(1));
    app.update();

    assert_eq!(
        visibility_of_sim(&mut app, enemy_sim),
        Some(Visibility::Hidden),
        "an UNSEEN enemy on a LOWER drawn storey stays Hidden — fog cut, not storey band",
    );
    assert_eq!(
        visibility_of_sim(&mut app, player_sim),
        Some(Visibility::Inherited),
        "a player ganger on the same LOWER drawn storey is shown",
    );
}

#[test]
fn hover_over_lower_storey_ganger_does_not_select_it_active_level_pick() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let active = Level::new(1);
    let ganger_cell = Cell::new(9, 9);
    let l0_at = CellLevel::new(ganger_cell, l0);
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(l0_at, 0, Direction::East))
        .player_faction(Faction::new(0))
        .slab_at(l0_at)
        .build();
    assert!(drive_setup(&mut app, situation), "setup must complete");

    let l0_sim = sim_entity_at(&mut app, l0_at);
    assert!(
        l0_sim.is_some(),
        "the ground-floor ganger must have spawned"
    );
    assert!(
        settle_actor(&mut app, l0_sim),
        "the ganger sprite must have materialized",
    );

    set_fog(&mut app, &[l0_at], &[]);
    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(active);
    app.update();
    assert_eq!(
        visibility_of_sim(&mut app, l0_sim),
        Some(Visibility::Inherited),
        "precondition: the lower-storey ganger is drawn at active level 1",
    );

    let hovered_at_active = CellLevel::new(ganger_cell, active);
    let ganger_pos = position_of_sim(&mut app, l0_sim);
    assert_eq!(
        ganger_pos,
        Some(l0_at),
        "the drawn lower-storey ganger's sim Position stays on storey 0",
    );
    assert_ne!(
        Some(hovered_at_active),
        ganger_pos,
        "active-level pick resolves to storey 1, which does not match the drawn storey-0 cell",
    );
}
