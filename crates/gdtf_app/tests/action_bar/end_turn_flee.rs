//! GTW-309 + GTW-240 battle-ending controls: the live end-turn button + the enabled flee button.

use bevy::{
    ecs::entity::Entity,
    prelude::*,
    ui::{Interaction, widget::Button},
};
use gdtf_app::test_support::{BattleRunningComplete, BattleScapeState, EndTurnButton, FleeButton};
use gdtf_battle_input::{ActIntent, PendingActIntent};
use gdtf_battle_sim::BattleInProgress;
use gdtf_test_utils::{advance_until, press_ui_button};
use gdtf_ui::DisabledButton;

use super::{harness::*, probes::*};

// ---------------------------------------------------------------------------------
// GTW-309 — the end-turn button is now LIVE: enabled, and a press pushes the fieldless
// GLOBAL ActIntent::EndTurn, which the ONE drain emits as a fieldless EndTurnRequested
// WITHOUT needing a selection. (GTW-275 removed the Reload deferred stub — reload is a LIVE
// weapon-panel button now — so there is no longer ANY deferred action-bar button.)
// ---------------------------------------------------------------------------------

/// GTW-309 — the end-turn button is ENABLED: it carries NO `DisabledButton`, so the
/// `Without<DisabledButton>` action filter now INCLUDES it (it is no longer a deferred
/// placeholder). The full press→intent→message wiring is exercised by
/// [`end_turn_button_emits_one_end_turn_requested_without_selection`].
#[test]
fn end_turn_button_is_enabled_not_disabled() {
    let mut app = battle_running_app();
    let Some(end_turn) = require_button::<EndTurnButton>(&mut app) else {
        return;
    };
    assert!(
        app.world().get::<DisabledButton>(end_turn).is_none(),
        "the end-turn button must be ENABLED — it must NOT carry DisabledButton (GTW-309)",
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

/// GTW-309 — pressing the end-turn button enqueues exactly one fieldless
/// `ActIntent::EndTurn`, and the ONE `dispatch_act_intents` drain emits exactly one fieldless
/// `EndTurnRequested` from it — with NO `SelectedShooter` set (the global turn signal needs no
/// selection, unlike a per-actor act). This is the load-bearing AC: it drives the REAL stack
/// (button press → 222a seam → the SAME drain the keyboard surface feeds) and asserts the
/// end-to-end message, byte-for-byte equal to the message the direct `ActIntent::EndTurn`
/// pushes (the `acts.rs` parity idiom).
///
/// Pin-discriminating: removing the new `EndTurnButton` arm in `action_bar_button_intents`
/// (or re-adding the `DisabledButton` marker) leaves the queue empty and emits zero messages,
/// failing the `len == 1` asserts.
#[test]
fn end_turn_button_emits_one_end_turn_requested_without_selection() {
    let mut app = battle_running_app();
    add_end_turn_probe(&mut app);
    // Deliberately NO arm_and_select / SelectedShooter — the end-turn intent is a fieldless
    // GLOBAL signal the drain emits unconditionally.

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

    // Byte-for-byte equal to the message the direct ActIntent::EndTurn (the keyboard surface)
    // pushes over the SAME seam — proving the button is a parallel surface, not a divergent
    // emission path.
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

// =================================================================================
// GTW-240 — the ENABLED Flee-battle button ends the persisting battle.
// =================================================================================

/// The caption of `button`'s `Text` child, if present (the `spawn_button` widget puts the
/// label on a `Text` child of the button root, not on the root itself).
fn button_label(app: &App, button: Entity) -> Option<String> {
    let children = app.world().get::<Children>(button)?;
    children
        .iter()
        .find_map(|child| app.world().get::<Text>(child).map(|text| text.0.clone()))
}

// ---------------------------------------------------------------------------------
// AC1 — the bar spawns exactly one ENABLED FleeButton (no DisabledButton) in BattleRunning,
// interactive, labelled "Flee" (D-D: shortened from "Flee battle").
// ---------------------------------------------------------------------------------

/// AC1 — in the live battle exactly one `FleeButton` is spawned; it carries `Button` +
/// `Interaction` (interactive), is ENABLED (NO `DisabledButton`, unlike the deferred
/// reload / end-turn buttons), and is labelled `"Flee"` (D-D: shortened from "Flee battle").
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
        "the flee button must be labelled \"Flee\" (D-D)",
    );
}

// ---------------------------------------------------------------------------------
// AC2 — a FleeButton press inserts BattleRunningComplete and the battle ends (the state
// advances to AnimateOut). This is the load-bearing AC.
// ---------------------------------------------------------------------------------

/// AC2 — pressing the flee button inserts the `BattleRunningComplete` end-signal marker and
/// the marker-gated `move_on` advances the machine out of `BattleRunning` to `AnimateOut`
/// (the explicit end requirement 5(b)).
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

    // Let `flee_button_pressed` insert the marker (Update) and `move_on` run (FixedUpdate),
    // then walk until the state leaves BattleRunning.
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

// ---------------------------------------------------------------------------------
// AC3 — a flee press is inert without BattleInProgress (the run_if gate holds).
// ---------------------------------------------------------------------------------

/// AC3 — with the `BattleInProgress` live-battle witness removed, a synthesized flee press
/// neither inserts `BattleRunningComplete` nor advances the state out of `BattleRunning` —
/// the `run_if(resource_exists::<BattleInProgress>)` gate holds (`bevy-traps.md` #1). The
/// REAL flee button entity is pressed (proving the system gate, not merely spawn timing).
#[test]
fn flee_button_inert_without_battle_in_progress() {
    let mut app = battle_running_app();
    let Some(flee) = require_button::<FleeButton>(&mut app) else {
        return;
    };

    // Remove the live-battle witness so the flee system's gate excludes it.
    app.world_mut().remove_resource::<BattleInProgress>();

    press_ui_button(&mut app, flee);
    // Several updates to prove no late insertion.
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

// ---------------------------------------------------------------------------------
// AC4 — FleeButton despawns OnExit(BattleRunning) with the bar.
// ---------------------------------------------------------------------------------

/// AC4 — after fleeing and leaving `BattleRunning`, the `FleeButton` is gone: it tore down
/// with the `ActionBarRoot` recursive despawn (`despawn_action_bar`), so it is battle-scoped
/// like every other action-bar button.
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

// ---------------------------------------------------------------------------------
// GTW-309 / GTW-240 — the end-turn and flee buttons are both ENABLED, distinct controls.
// ---------------------------------------------------------------------------------

/// GTW-309 / GTW-240 — the end-turn button (GTW-309 made it LIVE) and the flee button are
/// both ENABLED (neither carries `DisabledButton`) and are distinct entities: enabling the
/// end-turn button did not disturb the flee button, and vice versa. The full end-turn
/// press→emit wiring is `end_turn_button_emits_one_end_turn_requested_without_selection`;
/// the flee end-battle wiring is `flee_button_press_ends_battle`. (GTW-275: the Reload
/// deferred stub is GONE — reload is a LIVE weapon-panel button now.)
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
        "the end-turn button is ENABLED since GTW-309 — it must NOT carry DisabledButton",
    );
    assert!(
        app.world().get::<DisabledButton>(flee).is_none(),
        "the flee button is ENABLED — it must NOT carry DisabledButton",
    );
}
