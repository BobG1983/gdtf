//! Headless integration test for GTW-681 — the editor's `ButtonInput<KeyCode>` hotkeys must be
//! SUPPRESSED while an egui text field has keyboard focus.
//!
//! Drives the REAL [`MapEditorPlugin`] on the no-renderer `DefaultPlugins` UI harness (the same
//! harness the GTW-512 egui-shell test uses — a live [`AssetServer`] so the editor's actual `Load`
//! pass resolves the shipped theme + registries and its `Editing` scene inserts the model
//! resources), then exercises the SHIPPED `bevy_egui` focus machinery on the REAL hotkey systems:
//!
//! - the real [`write_egui_wants_input_system`] populates the real [`EguiWantsInput`] resource from
//!   a bare [`EguiContext`] whose egui memory we force keyboard focus on (via egui's own
//!   `request_focus`, so `Context::egui_wants_keyboard_input()` reports `true` with no window /
//!   render backend / widget draw), and
//! - the production `.run_if(not(egui_wants_any_keyboard_input))` guard the plugin attaches to the
//!   three `ButtonInput<KeyCode>` hotkey systems then decides whether they act.
//!
//! The headless UI harness ships NO windowed [`EguiPlugin`](bevy_egui::EguiPlugin), so this test
//! provisions [`EguiWantsInput`] + [`write_egui_wants_input_system`] itself (production gets both
//! from `EguiPlugin` — see `app.rs`). `assert!` + `let … else` keep the test panic-free per the
//! workspace lints (no `unwrap` / `expect` / `panic!`).

use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyCode, KeyboardInput},
    },
    prelude::*,
};
use bevy_egui::{
    EguiContext, egui,
    input::{EguiWantsInput, write_egui_wants_input_system},
};
use gdtf_battle_presenter::ViewMode;
use gdtf_content_editor::{CurrentEditLevel, EditorMode, EditorState, MapEditorPlugin};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};

/// A generous frame cap: the async asset loads under parallel `cargo` contention take a
/// non-deterministic number of frames, so this is a SAFETY NET (not a timing budget) — we poll the
/// `EditorState::Editing` SIGNAL, not a fixed count.
const MAX_UPDATES: u32 = 10_000;

/// The egui-widget id we force keyboard focus onto to stand in for a focused prefab width/height
/// `TextEdit` (the reported symptom) — any id makes `Context::egui_wants_keyboard_input()` true.
const PROBE_ID: &str = "gtw-681-suppression-probe";

/// Build the real editor app on the no-renderer `DefaultPlugins` UI harness, plus the SHIPPED
/// `bevy_egui` keyboard-focus machinery the headless harness omits (no windowed `EguiPlugin`): the
/// [`EguiWantsInput`] resource + the real [`write_egui_wants_input_system`] that computes it from
/// every [`EguiContext`]. Production gets both from `EguiPlugin::default()` (`app.rs`); the guard
/// under test consults exactly this resource.
fn editor_app_with_focus_machinery() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(MapEditorPlugin);
    app.init_resource::<EguiWantsInput>();
    app.add_systems(Update, write_egui_wants_input_system);
    app
}

/// Drive the app to `Editing`, then a few frames so the `OnEnter(Editing)` inserts apply.
fn advance_to_editing(app: &mut App) {
    let reached = advance_until(
        app,
        |app| {
            app.world()
                .get_resource::<State<EditorState>>()
                .is_some_and(|s| *s.get() == EditorState::Editing)
        },
        MAX_UPDATES,
    );
    assert!(
        reached,
        "the editor never reached EditorState::Editing — its Load pass did not resolve the theme + \
         registries",
    );
    for _ in 0..4 {
        app.update();
    }
}

/// Force egui keyboard focus by spawning a bare [`EguiContext`] whose egui memory has a focused
/// widget — `Context::egui_wants_keyboard_input()` is `memory(|m| m.focused().is_some())`, so this
/// alone flips it `true` (no `TextEdit`, no `Context::run`, no window). The next `app.update()` runs
/// [`write_egui_wants_input_system`], which reads this context and sets
/// [`EguiWantsInput::wants_keyboard_input`].
fn focus_egui_keyboard(app: &mut App) {
    let mut context = EguiContext::default();
    context
        .get_mut()
        .memory_mut(|memory| memory.request_focus(egui::Id::new(PROBE_ID)));
    app.world_mut().spawn(context);
}

/// Write a real [`KeyboardInput`] just-pressed message for `key_code`. The harness's real
/// `keyboard_input_system` (`PreUpdate`) turns it into a `just_pressed` edge on
/// `ButtonInput<KeyCode>` that survives into that frame's `Update`, so the hotkey systems observe
/// the press exactly as a live keypress would. The `logical_key` / `window` are irrelevant to the
/// hotkeys (they read the physical `key_code`).
fn press_key(app: &mut App, key_code: KeyCode) {
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key: Key::Character(" ".into()),
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
}

/// The current [`EditorMode`], or `None` if the resource is absent (state-scoped — bevy-traps #1).
fn mode(app: &App) -> Option<EditorMode> {
    app.world().get_resource::<EditorMode>().copied()
}

/// C1/C3 (the red-first core): a digit press while an egui text field has keyboard focus must NOT
/// switch the editor mode. `8` maps to the ATTACHMENT tab (the reported symptom: typing `8` into
/// the prefab width/height field jumped to ATTACHMENT); with the focus guard it stays PREFAB.
#[test]
fn digit_press_while_egui_focused_does_not_switch_mode() {
    let mut app = editor_app_with_focus_machinery();
    advance_to_editing(&mut app);
    assert_eq!(
        mode(&app),
        Some(EditorMode::Prefab),
        "the editor opens in the default PREFAB mode",
    );

    // Force keyboard focus, then settle a frame so `write_egui_wants_input_system` reports it.
    focus_egui_keyboard(&mut app);
    app.update();
    assert!(
        app.world()
            .get_resource::<EguiWantsInput>()
            .is_some_and(EguiWantsInput::wants_keyboard_input),
        "the focused egui context must make EguiWantsInput report keyboard input (the guard's \
         input)",
    );

    // Type `8` — with a focused text field, egui owns this keypress; the hotkey must not fire.
    press_key(&mut app, KeyCode::Digit8);
    app.update();

    assert_eq!(
        mode(&app),
        Some(EditorMode::Prefab),
        "a digit press while an egui text field is focused must NOT switch EditorMode (GTW-681 C1)",
    );
}

/// C2 (regression pin): with NO egui text field focused, a digit press still switches the mode —
/// `8` → ATTACHMENT — proving the guard is not a permanent no-op.
#[test]
fn digit_press_switches_mode_when_unfocused() {
    let mut app = editor_app_with_focus_machinery();
    advance_to_editing(&mut app);
    // No focused context: EguiWantsInput stays at its `false` default (the write system's query is
    // empty), so `not(egui_wants_any_keyboard_input)` allows the hotkey.
    assert!(
        !egui_wants_any_keyboard_input_now(&app),
        "with no focused egui context the guard reports no keyboard interest",
    );

    press_key(&mut app, KeyCode::Digit8);
    app.update();

    assert_eq!(
        mode(&app),
        Some(EditorMode::Attachment),
        "an unfocused digit `8` must switch to the ATTACHMENT tab (GTW-681 C2)",
    );
}

/// C2 (regression pin): with nothing focused, `F` still flips the prefab viewport [`ViewMode`] (the
/// editor opens in PREFAB, which `view_mode_hotkey` requires).
#[test]
fn f_flips_view_mode_when_unfocused() {
    let mut app = editor_app_with_focus_machinery();
    advance_to_editing(&mut app);

    let before = app.world().get_resource::<ViewMode>().copied();
    assert!(
        before.is_some(),
        "the ViewMode resource is inserted in Editing"
    );

    press_key(&mut app, KeyCode::KeyF);
    app.update();

    let after = app.world().get_resource::<ViewMode>().copied();
    assert_ne!(
        after, before,
        "an unfocused `F` must flip the prefab viewport ViewMode (GTW-681 C2)",
    );
}

/// C2 (regression pin): with nothing focused, `]` still steps the [`CurrentEditLevel`] up a storey
/// (the default `60×60×8` grid has room above the ground storey).
#[test]
fn bracket_steps_edit_level_when_unfocused() {
    let mut app = editor_app_with_focus_machinery();
    advance_to_editing(&mut app);

    let before = app.world().get_resource::<CurrentEditLevel>().copied();
    assert_eq!(
        before,
        Some(CurrentEditLevel::ground()),
        "the editor opens on the ground storey",
    );

    press_key(&mut app, KeyCode::BracketRight);
    app.update();

    let after = app.world().get_resource::<CurrentEditLevel>().copied();
    assert_ne!(
        after, before,
        "an unfocused `]` must step the CurrentEditLevel off the ground storey (GTW-681 C2)",
    );
}

/// The current value of the shipped `egui_wants_any_keyboard_input` run condition — the exact guard
/// the plugin attaches — evaluated over the test world so the control test asserts on the same
/// predicate production uses.
fn egui_wants_any_keyboard_input_now(app: &App) -> bool {
    app.world()
        .get_resource::<EguiWantsInput>()
        .is_some_and(EguiWantsInput::wants_any_keyboard_input)
}
