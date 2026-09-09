use bevy::prelude::*;
use cobalt_test_utils::press_left;
use gdtf_battle_input::{ActIntent, GdtfBattleInputPlugin};
use gdtf_battle_presenter::{ActiveLevel, ViewMode};
use gdtf_battle_sim::prelude::{Cell, CellLevel, Level, OccupancyGrid};

use super::{harness::*, intent_seam::active_level};

#[test]
fn inert_without_battle_in_progress() {
    let level = Level::new(2);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(GdtfBattleInputPlugin);
    // NOTE: no BattleInProgress inserted.
    app.world_mut().insert_resource(ActiveLevel::new(level));
    app.world_mut().insert_resource(ViewMode::default());
    app.world_mut().insert_resource(OccupancyGrid::default());

    let ganger = mint_entity();
    let cell = CellLevel::new(Cell::new(3, 3), level);
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    set_hovered(&mut app, Some(cell));
    push_intent(&mut app, ActIntent::LevelUp);
    press_left(&mut app);

    for _ in 0..4 {
        app.update();
    }

    assert_eq!(
        active_level(&app),
        Some(level),
        "ActiveLevel must be UNCHANGED pre-battle (the drain did not run)",
    );
    assert_eq!(
        selected(&app),
        None,
        "no selection must happen pre-battle (left_click_act did not run)",
    );
    assert_eq!(
        highlight_count(&mut app),
        0,
        "no selection highlight must spawn pre-battle",
    );
}
