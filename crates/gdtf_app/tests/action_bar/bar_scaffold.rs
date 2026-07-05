//! The bar itself: spawn/despawn, press==intent parity, no-selection no-op, real `bevy_ui` nodes, and the root fit-content layout.

use bevy::{
    camera::visibility::RenderLayers,
    prelude::*,
    ui::{Interaction, Node, widget::Button},
};
use gdtf_app::test_support::{
    AimToggleButton, BattleRunningComplete, BattleScapeState, LevelDownButton, LevelUpButton,
    StanceKneelingButton, StanceProneButton, StanceStandingButton,
};
use gdtf_battle_input::{ActIntent, PendingActIntent, SelectedShooter};
use gdtf_battle_presenter::WORLD_RENDER_LAYER;
use gdtf_battle_sim::{Direction, StanceKind};
use gdtf_test_utils::{advance_until, press_ui_button};

use super::{harness::*, probes::*};

// ---------------------------------------------------------------------------------
// AC1 — the bar spawns N markered, interactive buttons in BattleRunning and is
// despawned / inert outside it.
// ---------------------------------------------------------------------------------

/// AC1 — in the live battle the action-bar has spawned one interactive `Button` per
/// STABLE control (three stance toggles, aim, and the two level buttons), each carrying
/// `Button` + `Interaction`, and once the battle leaves `BattleRunning` they are
/// despawned.
#[test]
fn action_bar_spawns_in_battle_and_despawns_outside() {
    let mut app = battle_running_app();

    // Each stable-control marker resolves to exactly one entity carrying Button +
    // Interaction (interactive).
    let buttons = [
        require_button::<StanceStandingButton>(&mut app),
        require_button::<StanceKneelingButton>(&mut app),
        require_button::<StanceProneButton>(&mut app),
        require_button::<AimToggleButton>(&mut app),
        require_button::<LevelUpButton>(&mut app),
        require_button::<LevelDownButton>(&mut app),
    ];
    let present = buttons.iter().filter(|b| b.is_some()).count();
    assert_eq!(
        present, STABLE_CONTROL_BUTTONS,
        "the bar spawns exactly one button per stable control",
    );
    for found in buttons {
        let Some(button) = found else { return };
        assert!(
            app.world().get::<Button>(button).is_some(),
            "an action button must carry Button",
        );
        assert!(
            app.world().get::<Interaction>(button).is_some(),
            "an action button must carry Interaction (interactive)",
        );
    }

    // Leave BattleRunning: the battlescape now PERSISTS (GTW-236, the placeholder budget
    // auto-exit is gone), so the test inserts the explicit `BattleRunningComplete`
    // end-signal marker (standing in for the not-yet-wired victory/flee) to trip `move_on`
    // and advance the machine out of BattleRunning, where `OnExit` despawns the bar.
    app.world_mut().insert_resource(BattleRunningComplete);
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
        single_with::<StanceStandingButton>(&mut app).is_none(),
        "the action bar must be despawned once the battle leaves BattleRunning",
    );
}

// ---------------------------------------------------------------------------------
// AC3 — a button press writes the SAME *Requested the equivalent intent does,
// byte-for-byte (over the REAL 222a drain).
// ---------------------------------------------------------------------------------

/// AC3 — pressing the aim button emits one `SetAimingRequested` toggling the actor's aim,
/// byte-for-byte EQUAL to the direct `AimToggle` intent's message.
#[test]
fn aim_button_toggles_and_matches_direct_intent() {
    let mut app = battle_running_app();
    add_probes(&mut app);
    let ganger = arm_and_select(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    let Some(button) = require_button::<AimToggleButton>(&mut app) else {
        return;
    };
    press_ui_button(&mut app, button);
    app.update();

    let via_button = aims(&app);
    assert_eq!(
        via_button.len(),
        1,
        "one SetAimingRequested via the aim button"
    );
    assert_eq!(via_button[0].actor, ganger, "actor = *SelectedShooter");

    let mut app2 = battle_running_app();
    add_probes(&mut app2);
    let _ganger2 = arm_and_select(
        &mut app2,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    app2.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::AimToggle);
    app2.update();
    let via_intent = aims(&app2);
    assert_eq!(via_intent.len(), 1, "one SetAimingRequested via the intent");

    assert_eq!(
        via_button[0], via_intent[0],
        "the button and the intent (key) surface must produce byte-for-byte equal \
         SetAimingRequested",
    );
}

// ---------------------------------------------------------------------------------
// AC6 — with NO SelectedShooter, an act-button press is a no-op (no message, no panic).
// ---------------------------------------------------------------------------------

/// AC6 — with the selection cleared, pressing each act button across several updates
/// emits ZERO `*Requested` and does not panic (the drain finds nothing to act on).
#[test]
fn no_selection_makes_act_buttons_a_no_op() {
    let mut app = battle_running_app();
    add_probes(&mut app);
    app.world_mut().insert_resource(SelectedShooter::cleared());

    // Press the stance toggles + aim button (the *Requested-emitting acts) with no
    // selection.
    let act_buttons = [
        single_with::<StanceStandingButton>(&mut app),
        single_with::<StanceProneButton>(&mut app),
        single_with::<AimToggleButton>(&mut app),
    ];
    for button in act_buttons.into_iter().flatten() {
        press_ui_button(&mut app, button);
    }
    // Several updates to prove no deferred panic / late emission.
    for _ in 0..3 {
        app.update();
    }

    assert!(
        stances(&app).is_empty(),
        "no SetStanceRequested without a selection"
    );
    assert!(
        aims(&app).is_empty(),
        "no SetAimingRequested without a selection"
    );
}

// ---------------------------------------------------------------------------------
// AC7 — the bar renders on the UI camera (a bevy_ui Button/Node tree), NOT a
// WORLD_RENDER_LAYER sprite.
// ---------------------------------------------------------------------------------

/// AC7 — each action button carries `Button` + `Node` and does NOT carry
/// `RenderLayers::layer(WORLD_RENDER_LAYER)` (`bevy_ui` routes it to the highest-order =
/// UI camera per `bevy-traps.md` #6 — no extra `Pickable` / `IsDefaultUiCamera` / picking
/// plugin).
#[test]
fn buttons_are_ui_nodes_not_world_render_layer_sprites() {
    let mut app = battle_running_app();
    let world_layer = RenderLayers::layer(WORLD_RENDER_LAYER);

    let buttons = [
        require_button::<StanceStandingButton>(&mut app),
        require_button::<AimToggleButton>(&mut app),
        require_button::<LevelUpButton>(&mut app),
        require_button::<LevelDownButton>(&mut app),
    ];
    for found in buttons {
        let Some(button) = found else { return };
        assert!(
            app.world().get::<Button>(button).is_some(),
            "an action button must carry Button",
        );
        assert!(
            app.world().get::<Node>(button).is_some(),
            "an action button must carry Node (a bevy_ui node)",
        );
        let on_world_layer = app
            .world()
            .get::<RenderLayers>(button)
            .is_some_and(|layers| *layers == world_layer);
        assert!(
            !on_world_layer,
            "an action button must NOT be on the WORLD_RENDER_LAYER — it routes to the UI camera",
        );
    }
}

// ---------------------------------------------------------------------------------
// AC1 — the fit-content fix sets explicit Node fields on the bar root (height Auto; the bar
// grows upward from the bottom). Headless assert of the fields the fix sets, NOT a brittle
// pixel pin (the fit-content render itself is in-engine QA).
// ---------------------------------------------------------------------------------

/// GTW-272 AC1 / GTW-298 / D-C — the top action bar SHRINK-WRAPS its contents and sits at the
/// TOP-MIDDLE of the window. D-C (2026-06-18 screenshot review) split the bar into a TRANSPARENT
/// full-window-width centering WRAPPER (the `ActionBarRoot`) holding a compact, fit-content bordered
/// PANEL of the four buttons:
///
/// - the compact PANEL (the `LevelUp` button's direct parent) has `width: Auto` — it FITS its
///   contents rather than spanning the full window (the old `left:0 + right:0` stretch is gone) —
///   and `height: Auto` (fit-content vertically too);
/// - the WRAPPER (the panel's parent) is the full window WIDTH (`Vw(100)`) anchored to the screen
///   TOP (`top: 0`) and CENTRES the panel horizontally (`justify_content: Center`), anchoring it to
///   the top edge (`align_items: FlexStart`).
///
/// The fit-content / centred RENDER is an in-engine VISUAL check (verification.md #3); this pins
/// the explicit `Node` fields the layout sets (NO brittle pixel pin — only unit-kind asserts). A
/// revert to the full-width stretch (panel `width: Percent`/`right: 0`) fails the `width: Auto`
/// assert; a revert to a non-centred / non-full-width wrapper fails the wrapper asserts.
#[test]
fn action_bar_root_fits_contents_and_is_top_centered() {
    let mut app = battle_running_app();

    // The compact PANEL is the parent of the LevelUp button (a remaining action-bar control).
    let Some(level_up) = require_button::<LevelUpButton>(&mut app) else {
        return;
    };
    let Some(panel) = app.world().get::<ChildOf>(level_up).map(ChildOf::parent) else {
        return;
    };
    let Some(panel_node) = app.world().get::<Node>(panel) else {
        return;
    };
    assert_eq!(
        panel_node.width,
        Val::Auto,
        "the compact button panel must FIT its contents (width Auto, NOT a full-width stretch)",
    );
    assert_eq!(
        panel_node.height,
        Val::Auto,
        "the compact button panel must FIT its contents vertically too (height Auto)",
    );

    // The WRAPPER (the panel's parent, the `ActionBarRoot`) is the full-window-width centering row.
    let Some(wrapper) = app.world().get::<ChildOf>(panel).map(ChildOf::parent) else {
        return;
    };
    let Some(wrapper_node) = app.world().get::<Node>(wrapper) else {
        return;
    };
    assert_eq!(
        wrapper_node.width,
        Val::Vw(100.0),
        "the centering wrapper spans the full window WIDTH (Vw 100) so it can centre the panel",
    );
    assert_eq!(
        wrapper_node.justify_content,
        JustifyContent::Center,
        "the centering wrapper CENTRES the panel horizontally (top-middle) — D-C",
    );
    assert_eq!(
        wrapper_node.align_items,
        AlignItems::FlexStart,
        "the centering wrapper anchors the panel to its TOP edge (FlexStart)",
    );
    assert_eq!(
        wrapper_node.top,
        Val::Px(0.0),
        "the centering wrapper stays anchored to the screen TOP (GTW-298 / D-C)",
    );
}
