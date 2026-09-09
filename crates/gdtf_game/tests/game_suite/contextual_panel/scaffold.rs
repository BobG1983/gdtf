use bevy::prelude::*;
use cobalt_test_utils::advance_until;
use gdtf_game::test_support::{
    BattleScapeState, BottomBarRoot, ContextualPanelRoot, ExecuteButton, MeleeButton,
    OpenDoorButton, StabilizeButton,
};

use super::harness::*;

#[test]
fn contextual_panel_spawns_hidden_in_battle() {
    let mut app = battle_running_app();

    let root = the_only::<ContextualPanelRoot>(
        &mut app,
        "exactly one contextual panel root is spawned in BattleRunning",
    );
    assert!(
        single_with::<ExecuteButton>(&mut app).is_some(),
        "the Execute button exists exactly once in BattleRunning",
    );
    assert!(
        single_with::<StabilizeButton>(&mut app).is_some(),
        "the Stabilize button exists exactly once in BattleRunning",
    );
    assert!(
        single_with::<MeleeButton>(&mut app).is_some(),
        "the Melee button exists exactly once in BattleRunning",
    );
    assert!(
        single_with::<OpenDoorButton>(&mut app).is_some(),
        "the Open Door button exists exactly once in BattleRunning",
    );

    let bar = the_only::<BottomBarRoot>(
        &mut app,
        "the bottom bar must exist so the contextual panel can parent under it",
    );
    assert_eq!(
        parent_of(&app, root),
        Some(bar),
        "the contextual panel root must be a CHILD of the bottom bar, not a free-floating top-level overlay",
    );

    assert_eq!(
        visibility::<ContextualPanelRoot>(&mut app),
        Some(Visibility::Hidden),
        "the contextual panel root spawns Visibility::Hidden (scaffold: no detection yet)",
    );
    assert_eq!(
        visibility::<ExecuteButton>(&mut app),
        Some(Visibility::Hidden),
        "the Execute button spawns Visibility::Hidden (scaffold: no detection yet)",
    );
    assert_eq!(
        visibility::<StabilizeButton>(&mut app),
        Some(Visibility::Hidden),
        "the Stabilize button spawns Visibility::Hidden (scaffold: no detection yet)",
    );
    assert_eq!(
        visibility::<MeleeButton>(&mut app),
        Some(Visibility::Hidden),
        "the Melee button spawns Visibility::Hidden (scaffold: no detection yet)",
    );
    assert_eq!(
        visibility::<OpenDoorButton>(&mut app),
        Some(Visibility::Hidden),
        "the Open Door button spawns Visibility::Hidden (deferred act)",
    );
}

#[test]
fn contextual_panel_despawns_outside_battle() {
    let mut app = battle_running_app();
    assert!(
        single_with::<ContextualPanelRoot>(&mut app).is_some(),
        "sanity: the contextual panel box is present in BattleRunning before we leave it",
    );

    app.world_mut()
        .insert_resource(gdtf_game::test_support::BattleRunningComplete);
    advance_until(&mut app, |app| {
        battlescape_state(app) != Some(BattleScapeState::BattleRunning)
    });

    assert!(
        single_with::<ContextualPanelRoot>(&mut app).is_none(),
        "the contextual panel box must be despawned once the battle leaves BattleRunning",
    );
    assert!(
        single_with::<ExecuteButton>(&mut app).is_none(),
        "the Execute button must be despawned with the panel",
    );
    assert!(
        single_with::<StabilizeButton>(&mut app).is_none(),
        "the Stabilize button must be despawned with the panel",
    );
    assert!(
        single_with::<MeleeButton>(&mut app).is_none(),
        "the Melee button must be despawned with the panel",
    );
    assert!(
        single_with::<OpenDoorButton>(&mut app).is_none(),
        "the Open Door button must be despawned with the panel",
    );
}
