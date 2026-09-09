use bevy::{
    camera::visibility::RenderLayers,
    prelude::*,
    ui::{Interaction, Node, widget::Button},
};
use cobalt_test_utils::{advance_until, press_ui_button};
use gdtf_battle_input::{ActIntent, PendingActIntent, SelectedShooter};
use gdtf_battle_presenter::WORLD_RENDER_LAYER;
use gdtf_battle_sim::prelude::{Direction, StanceKind};
use gdtf_game::test_support::{
    AimToggleButton, BattleRunningComplete, BattleScapeState, LevelDownButton, LevelUpButton,
    StanceKneelingButton, StanceProneButton, StanceStandingButton,
};

use super::{harness::*, probes::*};

#[test]
fn action_bar_spawns_in_battle_and_despawns_outside() {
    let mut app = battle_running_app();

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

    app.world_mut().insert_resource(BattleRunningComplete);
    advance_until(&mut app, |app| {
        battlescape_state(app) != Some(BattleScapeState::BattleRunning)
    });
    assert!(
        single_with::<StanceStandingButton>(&mut app).is_none(),
        "the action bar must be despawned once the battle leaves BattleRunning",
    );
}

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

#[test]
fn no_selection_makes_act_buttons_a_no_op() {
    let mut app = battle_running_app();
    add_probes(&mut app);
    app.world_mut().insert_resource(SelectedShooter::cleared());

    let act_buttons = [
        single_with::<StanceStandingButton>(&mut app),
        single_with::<StanceProneButton>(&mut app),
        single_with::<AimToggleButton>(&mut app),
    ];
    for button in act_buttons.into_iter().flatten() {
        press_ui_button(&mut app, button);
    }
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

#[test]
fn action_bar_root_fits_contents_and_is_top_centered() {
    let mut app = battle_running_app();

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
        "the centering wrapper stays anchored to the screen TOP ",
    );
}
