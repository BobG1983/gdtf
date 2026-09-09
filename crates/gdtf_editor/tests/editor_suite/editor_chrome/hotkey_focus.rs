//! Hotkey focus: digit and F keys are ignored while egui wants the keyboard.
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
use cobalt_test_utils::{UiTestAppBuilder, advance_until};
use gdtf_battle_presenter::ViewMode;
use gdtf_editor::{CurrentEditLevel, EditorMode, EditorState, MapEditorPlugin};

const PROBE_ID: &str = "egui-suppression-probe";

fn editor_app_with_focus_machinery() -> App {
    let mut app = UiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(MapEditorPlugin);
    app.init_resource::<EguiWantsInput>();
    app.add_systems(Update, write_egui_wants_input_system);
    app
}

fn advance_to_editing(app: &mut App) {
    advance_until(app, |app| {
        app.world()
            .get_resource::<State<EditorState>>()
            .is_some_and(|s| *s.get() == EditorState::Editing)
    });
    for _ in 0..4 {
        app.update();
    }
}

fn focus_egui_keyboard(app: &mut App) {
    let mut context = EguiContext::default();
    context
        .get_mut()
        .memory_mut(|memory| memory.request_focus(egui::Id::new(PROBE_ID)));
    app.world_mut().spawn(context);
}

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

fn mode(app: &App) -> Option<EditorMode> {
    app.world().get_resource::<EditorMode>().copied()
}

#[test]
fn digit_press_while_egui_focused_does_not_switch_mode() {
    let mut app = editor_app_with_focus_machinery();
    advance_to_editing(&mut app);
    assert_eq!(
        mode(&app),
        Some(EditorMode::Prefab),
        "the editor opens in the default PREFAB mode",
    );

    focus_egui_keyboard(&mut app);
    app.update();
    assert!(
        app.world()
            .get_resource::<EguiWantsInput>()
            .is_some_and(EguiWantsInput::wants_keyboard_input),
        "the focused egui context must make EguiWantsInput report keyboard input (the guard's \
         input)",
    );

    press_key(&mut app, KeyCode::Digit8);
    app.update();

    assert_eq!(
        mode(&app),
        Some(EditorMode::Prefab),
        "a digit press while an egui text field is focused must NOT switch EditorMode",
    );
}

#[test]
fn digit_press_switches_mode_when_unfocused() {
    let mut app = editor_app_with_focus_machinery();
    advance_to_editing(&mut app);
    assert!(
        !egui_wants_any_keyboard_input_now(&app),
        "with no focused egui context the guard reports no keyboard interest",
    );

    press_key(&mut app, KeyCode::Digit8);
    app.update();

    assert_eq!(
        mode(&app),
        Some(EditorMode::Attachment),
        "an unfocused digit `8` must switch to the ATTACHMENT tab",
    );
}

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
        "an unfocused `F` must flip the prefab viewport ViewMode",
    );
}

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
        "an unfocused `]` must step the CurrentEditLevel off the ground storey",
    );
}

fn egui_wants_any_keyboard_input_now(app: &App) -> bool {
    app.world()
        .get_resource::<EguiWantsInput>()
        .is_some_and(EguiWantsInput::wants_any_keyboard_input)
}
