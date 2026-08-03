use bevy::prelude::*;
use gdtf_app::test_support::{
    BattleScapeState, BottomBarRoot, ContextualPanelRoot, ExecuteButton, MeleeButton,
    OpenDoorButton, StabilizeButton,
};
use gdtf_test_utils::advance_until;

use super::harness::*;


#[test]
fn contextual_panel_spawns_hidden_in_battle() {
    let mut app = battle_running_app();

    let root = single_with::<ContextualPanelRoot>(&mut app);
    assert!(
        root.is_some(),
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
        "the Melee button exists exactly once in BattleRunning (GTW-507)",
    );
    assert!(
        single_with::<OpenDoorButton>(&mut app).is_some(),
        "the Open Door button exists exactly once in BattleRunning",
    );

    let Some(root) = root else {
        return;
    };
    let bar = single_with::<BottomBarRoot>(&mut app);
    assert!(
        bar.is_some(),
        "the bottom bar must exist so the contextual panel can parent under it",
    );
    assert_eq!(
        parent_of(&app, root),
        bar,
        "the contextual panel root must be a CHILD of the bottom bar (GTW-726), not a \
         free-floating top-level overlay",
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
        "the Melee button spawns Visibility::Hidden (scaffold: no detection yet — GTW-507)",
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
        .insert_resource(gdtf_app::test_support::BattleRunningComplete);
    let left_battle_running = advance_until(
        &mut app,
        |app| battlescape_state(app) != Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        left_battle_running,
        "an explicit BattleRunningComplete insert must advance the machine out of BattleRunning \
         within {BUDGET} updates",
    );

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
        "the Melee button must be despawned with the panel (GTW-507)",
    );
    assert!(
        single_with::<OpenDoorButton>(&mut app).is_none(),
        "the Open Door button must be despawned with the panel",
    );
}
