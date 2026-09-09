use bevy::{image::TextureAtlas, prelude::*, ui::widget::ImageNode};
use cobalt_test_utils::{LoadTestAppBuilder, advance_until};
use gdtf_battle_sim::ganger::GangerName;
use gdtf_game::test_support::{
    AppState, BattleScapeState, RunningState, StatPortrait, portrait_index_for_name,
};

use super::harness::*;

fn load_battle_running_app() -> App {
    let mut app =
        LoadTestAppBuilder::new(gdtf_game::test_support::register_scenes_with_default_plugins)
            .starting_in(AppState::Load)
            .build();

    app.insert_resource(bevy::time::TimeUpdateStrategy::FixedTimesteps(1));
    advance_until(&mut app, |app| {
        running_state(app) == Some(RunningState::Menu)
    });
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::BattleRunning)
    });
    app
}

fn portrait_index(app: &mut App) -> Option<usize> {
    let portrait = single_with::<StatPortrait>(app)?;
    app.world()
        .get::<ImageNode>(portrait)
        .and_then(|n| n.texture_atlas.as_ref().map(|a: &TextureAtlas| a.index))
}

#[test]
fn portrait_index_is_deterministic_for_the_selected_ganger() {
    let mut app = load_battle_running_app();

    let vex_name = GangerName::new("Vex Harker".to_owned());
    let expected_vex = portrait_index_for_name(Some(&vex_name));
    spawn_and_select(&mut app, default_setup());
    app.update();

    let portrait_node = single_with::<StatPortrait>(&mut app);
    assert!(
        portrait_node.is_some(),
        "the stat block carries a portrait node"
    );

    let index = portrait_index(&mut app);
    assert!(
        index.is_some(),
        "the portrait node must carry a TextureAtlas (the portraits sheet loaded)",
    );
    let Some(index) = index else { return };
    assert_eq!(
        index, expected_vex,
        "the portrait index is the name's deterministic face"
    );

    let mut alex = default_setup();
    let alex_name = GangerName::new("Alex Mercer".to_owned());
    alex.name = alex_name.clone();
    let expected_alex = portrait_index_for_name(Some(&alex_name));
    assert_ne!(
        expected_alex, expected_vex,
        "the two test names must map to distinct portrait faces for the mutation to be visible",
    );
    spawn_and_select(&mut app, alex);
    app.update();
    assert_eq!(
        single_with::<StatPortrait>(&mut app),
        portrait_node,
        "the portrait node entity is stable (mutate, no respawn)",
    );
    assert_eq!(
        portrait_index(&mut app),
        Some(expected_alex),
        "the portrait index mutated to the new name's deterministic face",
    );
}
