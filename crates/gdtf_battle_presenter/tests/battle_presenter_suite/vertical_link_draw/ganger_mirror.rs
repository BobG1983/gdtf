use std::time::Duration;

use bevy::{
    app::App,
    math::Vec3,
    prelude::{Entity, Visibility},
    time::TimeUpdateStrategy,
    transform::components::Transform,
};
use cobalt_test_utils::advance_until_mut;
use gdtf_battle_presenter::{DrawnPosition, GangerSprites, Layer, cell_to_world_layered};
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Direction, Level, Position},
    test_support::SituationBuilder,
};

use super::harness::*;

fn sim_entity_at(app: &mut App, at: CellLevel) -> Option<Entity> {
    let mut q = app.world_mut().query::<(Entity, &Position)>();
    q.iter(app.world())
        .find(|(_, pos)| ***pos == at)
        .map(|(e, _)| e)
}

fn visibility_of_sim(app: &mut App, sim: Option<Entity>) -> Option<Visibility> {
    let sim = sim?;
    let sprite = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim))?;
    let mut q = app.world_mut().query::<&Visibility>();
    q.get(app.world(), sprite).ok().copied()
}

fn translation_of_sim(app: &mut App, sim: Entity) -> Option<Vec3> {
    let sprite = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim))?;
    let mut q = app.world_mut().query::<&Transform>();
    q.get(app.world(), sprite).ok().map(|t| t.translation)
}

fn settle_sprite_mapped(app: &mut App, sim: Entity) {
    advance_until_mut(app, |app| translation_of_sim(app, sim).is_some());
}

fn settle_visibility(app: &mut App, sim: Entity, expected: Visibility) {
    advance_until_mut(app, |app| {
        visibility_of_sim(app, Some(sim)) == Some(expected)
    });
}

#[test]
fn ganger_visibility_flips_hidden_when_position_leaves_active_storey() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = CellLevel::new(Cell::new(5, 5), Level::new(0));
    let l1 = CellLevel::new(Cell::new(5, 5), Level::new(1));
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(l0, 0, Direction::East))
        .slab_at(l0)
        .slab_at(l1)
        .build();
    drive_setup(&mut app, situation);

    let sim = sim_entity_at(&mut app, l0);
    assert!(sim.is_some(), "the ganger sim entity must exist");
    let Some(sim) = sim else { return };

    settle_sprite_mapped(&mut app, sim);

    assert_eq!(
        visibility_of_sim(&mut app, Some(sim)),
        Some(Visibility::Inherited),
        "a ganger ON the active storey is Visibility::Inherited",
    );

    let mut pos_q = app.world_mut().query::<&mut Position>();
    if let Ok(mut pos) = pos_q.get_mut(app.world_mut(), sim) {
        *pos = Position::new(l1);
    }
    app.update();
    settle_visibility(&mut app, sim, Visibility::Hidden);
    assert_eq!(
        visibility_of_sim(&mut app, Some(sim)),
        Some(Visibility::Hidden),
        "after Position.z leaves the active storey the ganger sprite goes Hidden (the handoff)",
    );

    let mut pos_q = app.world_mut().query::<&mut Position>();
    if let Ok(mut pos) = pos_q.get_mut(app.world_mut(), sim) {
        *pos = Position::new(l0);
    }
    app.update();
    settle_visibility(&mut app, sim, Visibility::Inherited);
    assert_eq!(
        visibility_of_sim(&mut app, Some(sim)),
        Some(Visibility::Inherited),
        "back on the active storey the ganger sprite goes Inherited again",
    );
}

#[test]
fn moved_sprite_is_intermediate_mid_tween_not_snapped() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let start = CellLevel::new(Cell::new(5, 6), Level::new(0));
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(start, 0, Direction::East))
        .build();
    drive_setup(&mut app, situation);

    let sim = sim_entity_at(&mut app, start);
    assert!(sim.is_some(), "the ganger sim entity must exist");
    let Some(sim) = sim else { return };

    let start_world = cell_to_world_layered(Cell::new(5, 6), Level::new(0), Layer::Actor);
    let dest = CellLevel::new(Cell::new(12, 6), Level::new(0));
    let dest_world = cell_to_world_layered(Cell::new(12, 6), Level::new(0), Layer::Actor);

    settle_sprite_mapped(&mut app, sim);

    let before = translation_of_sim(&mut app, sim);
    assert!(
        before.is_some_and(|t| t.distance(start_world) < 1.0e-3),
        "the sprite begins at the start cell (got {before:?}, expected {start_world:?})",
    );

    app.world_mut()
        .entity_mut(sim)
        .insert(DrawnPosition::seeded(Position::new(dest)));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        30,
    )));
    app.update();

    let mid = translation_of_sim(&mut app, sim);
    assert!(mid.is_some(), "the sprite must still exist mid-tween");
    let Some(mid) = mid else { return };
    assert!(
        mid.x > start_world.x && mid.x < dest_world.x,
        "the sprite must be at an INTERMEDIATE x mid-tween (strictly between start {} and dest \
         {}, NOT snapped), got {}",
        start_world.x,
        dest_world.x,
        mid.x,
    );
    assert!(
        mid.distance(start_world) > 1.0e-3,
        "the sprite must have started gliding (moved off the start cell), got {mid:?}",
    );
    assert!(
        mid.distance(dest_world) > 1.0e-3,
        "the sprite must NOT have snapped to the dest in one small step, got {mid:?}",
    );
}
