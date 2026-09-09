use bevy::{app::App, prelude::Visibility, transform::components::Transform};
use cobalt_test_utils::advance_until_mut;
use gdtf_battle_presenter::{ActiveLevel, TerrainSprite};
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Direction, Level, Position},
    test_support::SituationBuilder,
};

use super::{harness::*, probes::*};

fn terrain_z_at(app: &mut App, at: CellLevel) -> Option<f32> {
    let mut q = app.world_mut().query::<(&TerrainSprite, &Transform)>();
    q.iter(app.world())
        .find(|(marker, _)| marker.at == at)
        .map(|(_, transform)| transform.translation.z)
}

fn settle_terrain_z_at(app: &mut App, at: CellLevel) -> f32 {
    advance_until_mut(app, |app| terrain_z_at(app, at).is_some());
    let Some(z) = terrain_z_at(app, at) else {
        unreachable!("the wait only returns once the terrain sprite at {at:?} carries a z")
    };
    z
}

#[test]
fn active_level_change_shows_drawn_band_hides_above() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0_at = CellLevel::new(Cell::new(5, 6), Level::new(0));
    let l1_at = CellLevel::new(Cell::new(7, 8), Level::new(1));
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(l0_at, 0, Direction::East))
        .with_ganger(ganger_at(l1_at, 1, Direction::West))
        .slab_at(l0_at)
        .slab_at(l1_at)
        .build();
    drive_setup(&mut app, situation);
    band_only_fog(&mut app);
    app.update();

    let l0_sim = sim_entity_at(&mut app, l0_at);
    let l1_sim = sim_entity_at(&mut app, l1_at);
    assert!(
        l0_sim.is_some() && l1_sim.is_some(),
        "both gangers must have spawned",
    );

    assert_eq!(
        visibility_of_sim(&mut app, l0_sim),
        Some(Visibility::Inherited),
        "the level-0 ganger sprite is visible at active level 0 (in the band 0..=0)",
    );
    assert_eq!(
        visibility_of_sim(&mut app, l1_sim),
        Some(Visibility::Hidden),
        "the level-1 ganger sprite is hidden at active level 0 (strictly above the band)",
    );

    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(Level::new(1));
    app.update();

    assert_eq!(
        visibility_of_sim(&mut app, l1_sim),
        Some(Visibility::Inherited),
        "after the change, the level-1 (active) ganger sprite is visible",
    );
    assert_eq!(
        visibility_of_sim(&mut app, l0_sim),
        Some(Visibility::Inherited),
        "after the change, the level-0 ganger sprite is shown — lower storey within band 0..=1",
    );
}

#[test]
fn ganger_draws_above_its_own_floor_at_spawn_and_after_move() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let start = CellLevel::new(Cell::new(5, 6), Level::new(0));
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(start, 0, Direction::East))
        .build();
    drive_setup(&mut app, situation);

    let floor_z = settle_terrain_z_at(&mut app, start);

    let drawn = drawn_gangers(&mut app);
    assert_eq!(drawn.len(), 1, "one ganger sprite at spawn");
    let ganger_z = drawn[0].translation.z;

    assert!(
        ganger_z > floor_z,
        "the ganger sprite z ({ganger_z}) must be strictly greater than its own floor z ({floor_z})",
    );
    assert!(
        ganger_z > 0.0 && ganger_z < 1.0,
        "the level-0 ganger z ({ganger_z}) must satisfy 0.0 < z < 1.0",
    );

    let sim = sim_entity_at(&mut app, start);
    assert!(sim.is_some(), "the spawned ganger sim entity must exist");
    let Some(sim) = sim else { return };
    let dest = CellLevel::new(Cell::new(7, 8), Level::new(0));
    let mut pos_q = app.world_mut().query::<&mut Position>();
    if let Ok(mut pos) = pos_q.get_mut(app.world_mut(), sim) {
        *pos = Position::new(dest);
    }
    app.update();

    let drawn_after = drawn_gangers(&mut app);
    assert_eq!(
        drawn_after.len(),
        1,
        "still one ganger sprite after the move"
    );
    let moved_z = drawn_after[0].translation.z;
    let dest_floor_z = settle_terrain_z_at(&mut app, dest);
    assert!(
        moved_z > dest_floor_z,
        "after the move the ganger z ({moved_z}) must still be strictly greater than the floor z at its new cell ({dest_floor_z})",
    );
}
