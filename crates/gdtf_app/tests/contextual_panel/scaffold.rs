//! Panel scaffold AC: spawns hidden in battle, despawns outside.

use bevy::prelude::*;
use gdtf_app::test_support::{
    BattleScapeState, BottomBarRoot, ContextualPanelRoot, ExecuteButton, MeleeButton,
    OpenDoorButton, StabilizeButton,
};
use gdtf_test_utils::advance_until;

use super::harness::*;

// ---------------------------------------------------------------------------------
// Scaffold AC — the panel root + the per-act buttons spawn in BattleRunning, each with
// its marker and `Visibility::Hidden`; they despawn outside BattleRunning.
// ---------------------------------------------------------------------------------

/// In the live battle the contextual panel has spawned the panel-box root + exactly one
/// button per contextual act (Execute / Stabilize / Open Door). The box + buttons spawn
/// `Visibility::Hidden` (no detection reveals them in this state). The root is a CHILD of the
/// bottom bar (GTW-726) — laid out INSIDE the bottom panel, not floating over the map — and so
/// needs no `GlobalZIndex` of its own to draw above the bar (a child renders in the bar's own
/// stacking context).
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

    // GTW-726: the panel-box root is a CHILD of the BottomBarRoot container (the stance-panel
    // precedent), so it is laid out INSIDE the bottom panel rather than floating over the map. The
    // `is_some` assert above already failed loudly if the root is missing; bind without a panic
    // (restriction lints deny `panic!` even in tests).
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

    // SCAFFOLD: the panel box AND every button are hidden by default — no detection reveals
    // them in this state.
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

/// Once the battle leaves `BattleRunning` the contextual panel subtree — the panel box and its
/// three buttons — is despawned (battle-scoped `OnExit` cleanup over the ROOT marker, mirroring
/// the action bar / bottom bar).
#[test]
fn contextual_panel_despawns_outside_battle() {
    let mut app = battle_running_app();
    assert!(
        single_with::<ContextualPanelRoot>(&mut app).is_some(),
        "sanity: the contextual panel box is present in BattleRunning before we leave it",
    );

    // Leave BattleRunning: the battlescape PERSISTS (GTW-236), so the test inserts the explicit
    // `BattleRunningComplete` end-signal marker to trip `move_on` and advance the machine out of
    // BattleRunning, where `OnExit` despawns the panel. The marker is reached through the same
    // `test_support` surface the action-bar / weapon-panel tests use.
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
