//! GTW-411 in-engine QA demo for the [`TextField`](gdtf_ui::TextField) /
//! [`NumericField`](gdtf_ui::NumericField) widgets (AC4).
//!
//! A tiny `DefaultPlugins` app that spawns a themed PANEL and, over it, a
//! [`TextField`](gdtf_ui::TextField) (seeded with edited content so the caret + text are
//! visible) and a [`NumericField`](gdtf_ui::NumericField). The text field is auto-focused so
//! QA sees the rendered caret. The demo proves the full AC4 loop end-to-end in the RUNNING app:
//! it captures the rendered caret/edit state to a PNG, THEN drives a REAL commit through the
//! production keyboard/focus path (it does NOT poke the commit messages or buffers directly) so
//! the `info!`-logged commit lines confirm the VALUE COMMIT in-engine too.
//!
//! ## How QA runs it
//!
//! Capture a PNG and watch the commit log lines, then it exits on its own (no manual
//! interaction needed):
//!
//! ```text
//! GDTF_TEXTFIELD_SHOT=/abs/out.png cargo run -p gdtf_ui --example text_field_demo
//! ```
//!
//! Drop the `GDTF_TEXTFIELD_SHOT` env var to just watch it live (it stays open so you can type
//! into the focused text field and watch the buffer grow, then press Enter to commit). `Read`
//! the PNG to confirm: the text field shows its seeded edited content with a visible caret at
//! the text end, and the numeric field shows its (clamped) value. The capture path is purely a
//! dev affordance — it is NOT compiled into any shipped binary.
//!
//! ## GTW-454 single-caret evidence (C4)
//!
//! The demo has TWO fields on screen but auto-focuses only the text field, and the screenshot
//! is taken (at [`SHOT_FRAME`](capture::SHOT_FRAME)) BEFORE the commit drive moves focus. With
//! the GTW-454
//! focus-gate (`gate_caret_visibility`, registered by `register_text_field`), the PNG therefore
//! shows EXACTLY ONE caret — the focused text field's; the unfocused numeric field's caret is
//! hidden. That is the in-engine single-caret confirmation: two fields, one caret.
//!
//! ## The AC4 commit drive (the in-engine VALUE-COMMIT evidence)
//!
//! The screenshot is captured FIRST (render evidence of the edited/seeded state). THEN, on a
//! later frame, the demo drives a genuine commit through the SAME path the real app uses: it
//! sets [`InputFocus`](bevy::input_focus::InputFocus) to a field and WRITES a real
//! [`KeyboardInput`](bevy::input::keyboard::KeyboardInput) Enter message — so
//! `bevy_input_focus`'s `dispatch_focused_input` triggers the production
//! [`handle_text_field_key`](gdtf_ui::handle_text_field_key) observer, which emits
//! [`TextFieldCommitted`](gdtf_ui::TextFieldCommitted) /
//! [`NumericFieldCommitted`](gdtf_ui::NumericFieldCommitted). The `info!`-log-on-commit system
//! then prints a `TextFieldCommitted` line (with the committed text) and a
//! `NumericFieldCommitted` line (with the committed clamped number). The app exits ONLY after
//! the PNG is confirmed written AND both commit log lines have fired — with a frame cap so it
//! always exits.

mod capture;
mod commit_drive;

use std::{env, path::PathBuf};

use bevy::{
    input_focus::{FocusCause, InputFocus},
    log::LogPlugin,
    prelude::*,
    ui::{GlobalZIndex, Node, PositionType, Val},
};
use gdtf_ui::{
    CommittedTextValue, FieldColors, NumericRange, UiPlugin, register_numeric_field,
    register_text_field, spawn_numeric_field, spawn_panel, spawn_text_field, theme::default_theme,
};

use crate::{
    capture::{ShotPath, ShotRequested, ShotWritten, maybe_capture, poll_for_png},
    commit_drive::{CommitDrive, drive_commits, log_commits, maybe_exit},
};

/// Marker on the demo text field so the auto-focus / commit-drive systems find exactly it.
#[derive(Component)]
pub(crate) struct DemoTextField;

/// Marker on the demo numeric field so the commit-drive system can focus + Enter it.
#[derive(Component)]
pub(crate) struct DemoNumericField;

/// Counts elapsed update frames so the demo captures after the UI has had time to lay out.
#[derive(Resource, Default)]
pub(crate) struct FrameCounter(u32);

impl FrameCounter {
    /// The number of Update frames elapsed so far this run.
    pub(crate) const fn count(&self) -> u32 {
        self.0
    }
}

/// The inclusive numeric range the demo's numeric field clamps into (a ganger-attribute-like
/// 1..=10 band, standing in for a real GTW-403 attribute).
const ATTRIBUTE_MIN: i64 = 1;
/// The inclusive maximum of the demo numeric field's range.
const ATTRIBUTE_MAX: i64 = 10;

fn main() {
    let shot = env::var("GDTF_TEXTFIELD_SHOT").ok().map(PathBuf::from);
    let mut app = App::new();
    // `DefaultPlugins` includes `LogPlugin`, so the `info!` commit lines reach stderr — the
    // AC4 VALUE-COMMIT evidence. (Asserting it explicitly here documents the dependency.)
    app.add_plugins(DefaultPlugins.set(LogPlugin::default()))
        .add_plugins(UiPlugin)
        .insert_resource(ShotPath::new(shot))
        .init_resource::<FrameCounter>()
        .init_resource::<ShotRequested>()
        .init_resource::<ShotWritten>()
        .init_resource::<CommitDrive>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                tick_frames,
                auto_focus,
                log_commits,
                maybe_capture,
                poll_for_png,
                drive_commits,
                maybe_exit,
            )
                .chain(),
        );
    // Register the type-agnostic field pieces (the keyboard / blur observers + the commit
    // message + the per-frame sync/focus systems) and the concrete `i64` numeric handler —
    // this MUST happen at build time (`&mut App`), not inside a system (an unregistered
    // observer/handler is a dead feature — gate 4b).
    register_text_field(&mut app);
    register_numeric_field::<i64>(&mut app);
    app.run();
}

/// Spawns the camera, the theme, a panel, and the two fields over the panel.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    let theme = default_theme();
    commands.insert_resource(theme.clone());

    // A panel anchored top-left, sized so the fields visibly sit over it.
    let panel = spawn_panel(&mut commands, &theme);
    commands.entity(panel).insert((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Vw(8.0),
            top: Val::Vh(12.0),
            width: Val::Vw(36.0),
            height: Val::Vh(36.0),
            ..default()
        },
        GlobalZIndex(20),
    ));

    let colors = FieldColors {
        background: Color::srgb(0.14, 0.14, 0.18),
        text:       Color::srgb(0.92, 0.92, 0.86),
        caret:      Color::srgb(0.95, 0.85, 0.30),
    };

    // The text field, seeded with edited content so the text + end-caret are visible at rest.
    let text_field = spawn_text_field(
        &mut commands,
        CommittedTextValue::new("Goliath"),
        colors,
        DemoTextField,
    );
    commands.entity(text_field).insert((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Vw(10.0),
            top: Val::Vh(16.0),
            width: Val::Vw(26.0),
            ..default()
        },
        GlobalZIndex(21),
    ));

    // The numeric field, seeded ABOVE the range to show the clamp lands it at the max.
    let numeric_field = spawn_numeric_field::<i64>(
        &mut commands,
        99,
        NumericRange::new(ATTRIBUTE_MIN, ATTRIBUTE_MAX),
        colors,
        DemoNumericField,
    );
    commands.entity(numeric_field).insert((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Vw(10.0),
            top: Val::Vh(26.0),
            width: Val::Vw(10.0),
            ..default()
        },
        GlobalZIndex(21),
    ));
}

/// Advances the frame counter once per Update (runs first in the chain so every later system
/// reads a consistent frame number this tick).
fn tick_frames(mut frames: ResMut<FrameCounter>) {
    frames.0 += 1;
}

/// Auto-focuses the demo text field on the first frame so its caret renders without a click.
fn auto_focus(
    frames: Res<FrameCounter>,
    mut focus: ResMut<InputFocus>,
    field: Query<Entity, With<DemoTextField>>,
) {
    if frames.0 == 1
        && let Ok(entity) = field.single()
    {
        focus.set(entity, FocusCause::Navigated);
    }
}
