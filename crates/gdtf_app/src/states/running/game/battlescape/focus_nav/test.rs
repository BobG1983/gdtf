//! GTW-782: headless behavioral tests for the battlescape focus-navigation wiring — the
//! app-side bridge, the inter-panel topology rebuild, the Escape cancel, and the focus
//! outline — driven over the REAL system fns on a `MinimalPlugins` + `FocusNavPlugin` app.
//!
//! These exercise the actual wired path: the bridge writes `gdtf_ui`'s `NavigateRequest` /
//! `FocusCancelled`, the topology rebuild lays the Tab chain, and `gdtf_ui`'s
//! `apply_navigation` (installed by `FocusNavPlugin`) moves `InputFocus` — so a Tab that
//! moves focus proves the whole chain end-to-end. Buttons at the three panel nav-order
//! bands stand in for the action bar / weapon panel / contextual panel, so a walk across
//! them is the per-panel coverage.
//!
//! Every `app.world_mut()` mutation is in a TEST BODY (`bevy-traps.md` #7 carve-out (a)).

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

/// A test `Keybinds` — Tab = `select_next`/`select_prev`, Escape = `select_clear` (the
/// shipped chord).
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

/// The base focus-nav app: `MinimalPlugins` + `FocusNavPlugin` (which installs
/// `InputFocus`, the `DirectionalNavigationMap`, the nav messages, and `apply_navigation`),
/// plus the GTW-782 systems wired exactly as the scene-plugin wires them (rebuild + bridge
/// before `Apply`; cancel after the bridge; the outline). A seeded `ButtonInput` + a bound
/// `Keybinds`.
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

/// Spawns a SHOWN (non-`Hidden`) focus-navigable panel button at nav-order `order`.
fn shown_button(app: &mut App, order: u16) -> Entity {
    app.world_mut()
        .spawn((PanelNavOrder::new(order), Visibility::Inherited))
        .id()
}

/// Points `InputFocus` at `entity` (the panel-focus context).
fn focus_on(app: &mut App, entity: Entity) {
    app.world_mut()
        .insert_resource(InputFocus::from_entity(entity));
}

/// The currently-focused entity, if any.
fn focused(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

/// Presses `key`, runs one update, then clears the just-pressed edge (so the next press is
/// a fresh `just_pressed` — `MinimalPlugins` has no `keyboard_input_system` to auto-clear).
fn tap(app: &mut App, key: KeyCode) {
    press_key(app, key);
    app.update();
    clear_keys(app);
}

/// Tab walks focus forward (East) across all three panel bands, and Shift+Tab walks it
/// back (West) — the per-panel "focus-nav works via Tab" pin.
///
/// Buttons at the action-bar / weapon / contextual bands form the chain
/// `bar → weapon → ctx`. From the bar button, Tab must step to the weapon button, then the
/// contextual button; Shift+Tab must walk back. If the bridge stopped writing the navigate
/// request (or the topology stopped chaining the buttons), focus would not move and these
/// assertions fail.
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

    // Shift+Tab steps back West.
    press_key(&mut app, KeyCode::ShiftLeft);
    tap(&mut app, KeyCode::Tab);
    assert_eq!(
        focused(&app),
        Some(weapon),
        "Shift+Tab steps contextual → weapon (West)",
    );
}

/// The right / left arrows navigate like Tab / Shift+Tab (the horizontal arrow mirror).
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

/// Escape backs out of panel focus: the bridge raises `FocusCancelled` and
/// `apply_focus_cancel` clears `InputFocus`, so focus returns to nothing (the
/// map/selection context resumes).
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

/// A hidden or disabled button drops out of the Tab chain — the dynamic-visibility the
/// battlescape topology must handle (unlike the menu's static column).
///
/// Chain fixtures: a shown bar button, a HIDDEN weapon button (mid-band), and a shown
/// contextual button. Tab from the bar button must SKIP the hidden one straight to the
/// contextual button. A second run makes the middle button DISABLED instead, with the same
/// skip. If the rebuild stopped filtering `Hidden` / `DisabledButton`, focus would land on
/// the excluded button and the assertion fails.
#[test]
fn hidden_and_disabled_buttons_drop_out_of_the_chain() {
    // Hidden middle button.
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

    // Disabled middle button.
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

/// The focus outline marks exactly the focused button — the visual the QA screenshot reads.
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

    // Move focus (Tab East) → the ring follows.
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

/// The bridge is INERT when no panel holds focus: with focus on a NON-panel entity, Tab
/// raises no navigate request, so focus stays put (the map/selection context owns Tab).
#[test]
fn bridge_is_inert_without_panel_focus() {
    let mut app = focus_nav_app();
    let _a = shown_button(&mut app, ACTION_BAR_NAV_BASE);
    let _b = shown_button(&mut app, WEAPON_NAV_BASE);
    // Focus a stray, non-PanelNavOrder entity.
    let stray = app.world_mut().spawn_empty().id();
    focus_on(&mut app, stray);

    tap(&mut app, KeyCode::Tab);
    assert_eq!(
        focused(&app),
        Some(stray),
        "with no PANEL button focused, Tab drives no panel focus-nav (bridge inert)",
    );
}
