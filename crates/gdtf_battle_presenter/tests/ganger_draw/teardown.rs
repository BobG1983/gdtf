use bevy::app::{App, Update};
use gdtf_battle_presenter::GangerSprites;
use gdtf_battle_sim::{
    battle::{TeardownBattleRequested, teardown_battle_on_request},
    prelude::{Cell, CellLevel, Direction, Level},
    test_support::SituationBuilder,
};
use gdtf_test_utils::advance_until;

use super::{harness::*, probes::*};

fn sprite_map_is_empty(app: &App) -> bool {
    app.world()
        .get_resource::<GangerSprites>()
        .is_some_and(GangerSprites::is_empty)
}

#[test]
fn teardown_despawns_every_ganger_sprite_and_clears_the_map() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.add_message::<TeardownBattleRequested>()
        .add_systems(Update, teardown_battle_on_request);

    let g0_at = CellLevel::new(Cell::new(5, 6), Level::new(0));
    let g1_at = CellLevel::new(Cell::new(12, 9), Level::new(0));
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(g0_at, 0, Direction::East))
        .with_ganger(ganger_at(g1_at, 1, Direction::North))
        .build();
    assert!(
        drive_setup(&mut app, situation),
        "setup_battle must complete"
    );

    assert_eq!(
        drawn_gangers(&mut app).len(),
        2,
        "both gangers must be drawn before teardown, or this case proves nothing",
    );

    app.world_mut().write_message(TeardownBattleRequested);
    let cleared = advance_until(&mut app, sprite_map_is_empty, MAX_UPDATES);

    let left = drawn_gangers(&mut app);
    assert!(
        left.is_empty(),
        "leaving a battle must despawn every ganger sprite, but {} survived",
        left.len(),
    );
    assert!(
        cleared && sprite_map_is_empty(&app),
        "leaving a battle must clear every GangerSprites entry",
    );
}
