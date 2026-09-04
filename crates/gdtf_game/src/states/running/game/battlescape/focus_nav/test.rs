use bevy::{input::ButtonInput, input_focus::InputFocus, prelude::*};
use gdtf_battle_input::{BoundKey, Keybinds, PanelNavOrder};
use gdtf_test_utils::{clear_keys, press_key};
use gdtf_ui::{
    DisabledButton,
    focus_nav::{FocusNavPlugin, FocusNavSystems},
};

use super::{
    bridge::{apply_focus_cancel, bridge_panel_focus_nav},
    outline::paint_focus_outline,
    topology::{
        ACTION_BAR_NAV_BASE, CONTEXTUAL_NAV_BASE, WEAPON_NAV_BASE, rebuild_panel_nav_topology,
    },
};

const fn test_keybinds() -> Keybinds {
    Keybinds {
        select_clear:     BoundKey::KeyEscape,
        level_up:         BoundKey::KeyPageUp,
        level_down:       BoundKey::KeyPageDown,
        toggle_full_view: BoundKey::KeyV,
        stance_cycle:     BoundKey::KeyC,
        aim_toggle:       BoundKey::KeyF,
        facing_cycle:     BoundKey::KeyR,
        select_next:      BoundKey::KeyTab,
        select_prev:      BoundKey::KeyTab,
    }
}

fn focus_nav_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_plugins(FocusNavPlugin);
    app.insert_resource(test_keybinds());
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.add_systems(
        Update,
        (
            rebuild_panel_nav_topology.before(FocusNavSystems::Apply),
            bridge_panel_focus_nav.before(FocusNavSystems::Apply),
            apply_focus_cancel.after(bridge_panel_focus_nav),
            paint_focus_outline,
        ),
    );
    app
}

fn shown_button(app: &mut App, order: u16) -> Entity {
    app.world_mut()
        .spawn((PanelNavOrder::new(order), Visibility::Inherited))
        .id()
}

fn focus_on(app: &mut App, entity: Entity) {
    app.world_mut()
        .insert_resource(InputFocus::from_entity(entity));
}

fn focused(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

fn tap(app: &mut App, key: KeyCode) {
    press_key(app, key);
    app.update();
    clear_keys(app);
}

#[test]
fn tab_walks_focus_across_the_three_panels() {
    let mut app = focus_nav_app();
    let bar = shown_button(&mut app, ACTION_BAR_NAV_BASE);
    let weapon = shown_button(&mut app, WEAPON_NAV_BASE);
    let ctx = shown_button(&mut app, CONTEXTUAL_NAV_BASE);
    focus_on(&mut app, bar);

    tap(&mut app, KeyCode::Tab);
    assert_eq!(focused(&app), Some(weapon), "Tab steps bar → weapon (East)");

    tap(&mut app, KeyCode::Tab);
    assert_eq!(
        focused(&app),
        Some(ctx),
        "Tab steps weapon → contextual (East)",
    );

    press_key(&mut app, KeyCode::ShiftLeft);
    tap(&mut app, KeyCode::Tab);
    assert_eq!(
        focused(&app),
        Some(weapon),
        "Shift+Tab steps contextual → weapon (West)",
    );
}

#[test]
fn arrows_navigate_like_tab() {
    let mut app = focus_nav_app();
    let a = shown_button(&mut app, ACTION_BAR_NAV_BASE);
    let b = shown_button(&mut app, WEAPON_NAV_BASE);
    focus_on(&mut app, a);

    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(focused(&app), Some(b), "ArrowRight steps East (like Tab)");

    tap(&mut app, KeyCode::ArrowLeft);
    assert_eq!(
        focused(&app),
        Some(a),
        "ArrowLeft steps West (like Shift+Tab)"
    );
}

#[test]
fn escape_cancels_panel_focus() {
    let mut app = focus_nav_app();
    let a = shown_button(&mut app, ACTION_BAR_NAV_BASE);
    let _b = shown_button(&mut app, WEAPON_NAV_BASE);
    focus_on(&mut app, a);

    tap(&mut app, KeyCode::Escape);
    assert_eq!(
        focused(&app),
        None,
        "Escape (FocusCancelled → InputFocus::clear) drops keyboard focus",
    );
}

#[test]
fn hidden_and_disabled_buttons_drop_out_of_the_chain() {
    let mut app = focus_nav_app();
    let bar = shown_button(&mut app, ACTION_BAR_NAV_BASE);
    let hidden = app
        .world_mut()
        .spawn((PanelNavOrder::new(WEAPON_NAV_BASE), Visibility::Hidden))
        .id();
    let ctx = shown_button(&mut app, CONTEXTUAL_NAV_BASE);
    focus_on(&mut app, bar);

    tap(&mut app, KeyCode::Tab);
    assert_eq!(
        focused(&app),
        Some(ctx),
        "Tab skips the HIDDEN middle button straight to the contextual button",
    );
    assert_ne!(
        focused(&app),
        Some(hidden),
        "focus never lands on a hidden button"
    );

    let mut app = focus_nav_app();
    let bar = shown_button(&mut app, ACTION_BAR_NAV_BASE);
    let disabled = app
        .world_mut()
        .spawn((
            PanelNavOrder::new(WEAPON_NAV_BASE),
            Visibility::Inherited,
            DisabledButton,
        ))
        .id();
    let ctx = shown_button(&mut app, CONTEXTUAL_NAV_BASE);
    focus_on(&mut app, bar);

    tap(&mut app, KeyCode::Tab);
    assert_eq!(
        focused(&app),
        Some(ctx),
        "Tab skips the DISABLED middle button straight to the contextual button",
    );
    assert_ne!(
        focused(&app),
        Some(disabled),
        "focus never lands on a disabled button",
    );
}

#[test]
fn focus_outline_marks_only_the_focused_button() {
    let mut app = focus_nav_app();
    let a = shown_button(&mut app, ACTION_BAR_NAV_BASE);
    let b = shown_button(&mut app, WEAPON_NAV_BASE);
    focus_on(&mut app, a);

    app.update();
    assert!(
        app.world().get::<Outline>(a).is_some(),
        "the focused button gains the focus-ring Outline",
    );
    assert!(
        app.world().get::<Outline>(b).is_none(),
        "an unfocused button has no focus ring",
    );

    tap(&mut app, KeyCode::Tab);
    app.update();
    assert!(
        app.world().get::<Outline>(b).is_some(),
        "the ring moves to the newly-focused button",
    );
    assert!(
        app.world().get::<Outline>(a).is_none(),
        "the previously-focused button loses its ring",
    );
}

#[test]
fn bridge_is_inert_without_panel_focus() {
    let mut app = focus_nav_app();
    let _a = shown_button(&mut app, ACTION_BAR_NAV_BASE);
    let _b = shown_button(&mut app, WEAPON_NAV_BASE);
    let stray = app.world_mut().spawn_empty().id();
    focus_on(&mut app, stray);

    tap(&mut app, KeyCode::Tab);
    assert_eq!(
        focused(&app),
        Some(stray),
        "with no PANEL button focused, Tab drives no panel focus-nav (bridge inert)",
    );
}
