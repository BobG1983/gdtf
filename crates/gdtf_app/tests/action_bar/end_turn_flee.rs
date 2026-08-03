use bevy::{
    ecs::entity::Entity,
    prelude::*,
    ui::{Interaction, widget::Button},
};
use gdtf_app::test_support::{BattleRunningComplete, BattleScapeState, EndTurnButton, FleeButton};
use gdtf_battle_input::{ActIntent, PendingActIntent};
use gdtf_battle_sim::prelude::BattleInProgress;
use gdtf_test_utils::{advance_until, press_ui_button};
use gdtf_ui::DisabledButton;

use super::{harness::*, probes::*};


#[test]
fn end_turn_button_is_enabled_not_disabled() {
    let mut app = battle_running_app();
    let Some(end_turn) = require_button::<EndTurnButton>(&mut app) else {
        return;
    };
    assert!(
        app.world().get::<DisabledButton>(end_turn).is_none(),
        "the end-turn button must be ENABLED — it must NOT carry DisabledButton",
    );
    assert!(
        app.world().get::<Button>(end_turn).is_some(),
        "the end-turn button must carry Button (interactive)",
    );
    assert!(
        app.world().get::<Interaction>(end_turn).is_some(),
        "the end-turn button must carry Interaction (interactive)",
    );
}

#[test]
fn end_turn_button_emits_one_end_turn_requested_without_selection() {
    let mut app = battle_running_app();
    add_end_turn_probe(&mut app);

    let Some(end_turn) = require_button::<EndTurnButton>(&mut app) else {
        return;
    };
    press_ui_button(&mut app, end_turn);
    app.update();

    let via_button = end_turns(&app);
    assert_eq!(
        via_button.len(),
        1,
        "pressing the end-turn button must emit exactly one EndTurnRequested (no selection \
         needed)",
    );

    let mut app2 = battle_running_app();
    add_end_turn_probe(&mut app2);
    app2.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::EndTurn);
    app2.update();
    let via_intent = end_turns(&app2);
    assert_eq!(
        via_intent.len(),
        1,
        "one EndTurnRequested via the direct ActIntent::EndTurn"
    );
    assert_eq!(
        via_button[0], via_intent[0],
        "the button and the key/intent surface must produce byte-for-byte equal \
         EndTurnRequested",
    );
}


fn button_label(app: &App, button: Entity) -> Option<String> {
    let children = app.world().get::<Children>(button)?;
    children
        .iter()
        .find_map(|child| app.world().get::<Text>(child).map(|text| text.0.clone()))
}


#[test]
fn flee_button_spawns_enabled_in_battle() {
    let mut app = battle_running_app();

    let Some(flee) = require_button::<FleeButton>(&mut app) else {
        return;
    };
    assert!(
        app.world().get::<Button>(flee).is_some(),
        "the flee button must carry Button",
    );
    assert!(
        app.world().get::<Interaction>(flee).is_some(),
        "the flee button must carry Interaction (interactive)",
    );
    assert!(
        app.world().get::<DisabledButton>(flee).is_none(),
        "the flee button must be ENABLED — it must NOT carry DisabledButton",
    );
    assert_eq!(
        button_label(&app, flee).as_deref(),
        Some("Flee"),
        "the flee button must be labelled \"Flee\"",
    );
}


#[test]
fn flee_button_press_ends_battle() {
    let mut app = battle_running_app();
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::BattleRunning),
        "the harness must start in BattleRunning",
    );

    let Some(flee) = require_button::<FleeButton>(&mut app) else {
        return;
    };
    press_ui_button(&mut app, flee);

    let left_battle_running = advance_until(
        &mut app,
        |app| battlescape_state(app) != Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        left_battle_running,
        "a flee press must advance the machine out of BattleRunning within {BUDGET} updates; \
         last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::AnimateOut),
        "a flee press must end the battle by advancing BattleRunning -> AnimateOut",
    );
}


#[test]
fn flee_button_inert_without_battle_in_progress() {
    let mut app = battle_running_app();
    let Some(flee) = require_button::<FleeButton>(&mut app) else {
        return;
    };

    app.world_mut().remove_resource::<BattleInProgress>();

    press_ui_button(&mut app, flee);
    for _ in 0..3 {
        app.update();
    }

    assert!(
        app.world()
            .get_resource::<BattleRunningComplete>()
            .is_none(),
        "a flee press with no BattleInProgress must NOT insert BattleRunningComplete",
    );
    assert_eq!(
        battlescape_state(&app),
        Some(BattleScapeState::BattleRunning),
        "a flee press with no BattleInProgress must NOT advance the state out of BattleRunning",
    );
}


#[test]
fn flee_button_despawns_on_exit_battle_running() {
    let mut app = battle_running_app();
    let Some(flee) = require_button::<FleeButton>(&mut app) else {
        return;
    };
    press_ui_button(&mut app, flee);

    let left_battle_running = advance_until(
        &mut app,
        |app| battlescape_state(app) != Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        left_battle_running,
        "a flee press must advance the machine out of BattleRunning within {BUDGET} updates",
    );
    assert!(
        single_with::<FleeButton>(&mut app).is_none(),
        "the flee button must be despawned once the battle leaves BattleRunning (with the bar)",
    );
}


#[test]
fn end_turn_and_flee_buttons_are_both_enabled() {
    let mut app = battle_running_app();

    let Some(end_turn) = require_button::<EndTurnButton>(&mut app) else {
        return;
    };
    let Some(flee) = require_button::<FleeButton>(&mut app) else {
        return;
    };

    assert_ne!(
        end_turn, flee,
        "the end-turn and flee buttons must be distinct entities",
    );
    assert!(
        app.world().get::<DisabledButton>(end_turn).is_none(),
        "the end-turn button is ENABLED — it must NOT carry DisabledButton",
    );
    assert!(
        app.world().get::<DisabledButton>(flee).is_none(),
        "the flee button is ENABLED — it must NOT carry DisabledButton",
    );
}
