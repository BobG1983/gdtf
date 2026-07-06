//! Deterministic portrait atlas index for the selected ganger, on the real asset stack.

use bevy::{image::TextureAtlas, prelude::*, ui::widget::ImageNode};
use gdtf_app::test_support::{
    AppState, BattleScapeState, RunningState, StatPortrait, portrait_index_for_name,
};
use gdtf_battle_sim::ganger::GangerName;
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until};

use super::harness::*;

/// A larger budget for the real `DefaultPlugins` async asset loads + the full state descent
/// (the `real_battle_panel.rs` precedent) when the portrait atlas must actually load.
const LOAD_BUDGET: u32 = 512;

/// Drives the REAL `DefaultPlugins` asset stack (`GdtfLoadTestAppBuilder`, a live
/// `AssetServer` rooted at the workspace `assets/`) from `Load` down to `BattleRunning`,
/// asserting the descent. UNLIKE [`battle_running_app`] (`MinimalPlugins`, no `AssetServer`),
/// here `TopDownRendererPlugin`'s `Startup` `load_topdown_atlases` runs for real, so the
/// `OnEnter(BattleRunning)` panel spawn reads a present `TopDownAtlases` and the portrait
/// node carries a `TextureAtlas` over the portraits sheet — the harness the portrait
/// node-wiring assertions need (the `atlas_load.rs` / `real_battle_panel.rs` pattern).
fn load_battle_running_app() -> App {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    let at_menu = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        LOAD_BUDGET,
    );
    assert!(
        at_menu,
        "the real Load + descent must reach RunningState::Menu within {LOAD_BUDGET} updates",
    );
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    let at_battle = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        LOAD_BUDGET,
    );
    assert!(
        at_battle,
        "the real battle must reach BattleScapeState::BattleRunning within {LOAD_BUDGET} \
         updates; last BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    app
}

/// The portrait node's current atlas index (the `TextureAtlas.index` on the single
/// `StatPortrait` `ImageNode`). `None` if the portrait has no atlas (sheet unloaded).
fn portrait_index(app: &mut App) -> Option<usize> {
    let portrait = single_with::<StatPortrait>(app)?;
    app.world()
        .get::<ImageNode>(portrait)
        .and_then(|n| n.texture_atlas.as_ref().map(|a: &TextureAtlas| a.index))
}

// ---------------------------------------------------------------------------------
// Portrait — deterministic atlas index from the ganger name.
// ---------------------------------------------------------------------------------

/// The portrait node carries a `TextureAtlas` pointing at the portraits sheet, at the
/// DETERMINISTIC index for the ganger's name (computed in-test from the same rule), and
/// selecting a different-named ganger MUTATES the index on the SAME node (no respawn).
///
/// Driven on the REAL `DefaultPlugins` asset stack ([`load_battle_running_app`]) so
/// `load_topdown_atlases` actually runs and the `OnEnter(BattleRunning)` panel spawn gives
/// the portrait node a `TextureAtlas` over the portraits sheet — the assertions run
/// UNCONDITIONALLY (no `if let Some` guard), so a reverted `update_portrait` (or a missing
/// atlas) would FAIL this test rather than silently skip it.
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

    // The node carries a TextureAtlas over the portraits sheet (the real atlas loaded).
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

    // A different-named ganger MUTATES the index on the SAME node.
    let mut alex = default_setup();
    let alex_name = GangerName::new("Alex Mercer".to_owned());
    alex.name = alex_name.clone();
    let expected_alex = portrait_index_for_name(Some(&alex_name));
    // The two authored names must derive DISTINCT faces, else the mutation is unobservable
    // (the derivation determinism itself is unit-tested in `stat_block/test.rs`).
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
