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
//! is taken (at [`SHOT_FRAME`]) BEFORE the commit drive moves focus. With the GTW-454
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
//! [`TextFieldCommitted`] / [`NumericFieldCommitted`]. The `info!`-log-on-commit system then
//! prints a `TextFieldCommitted` line (with the committed text) and a `NumericFieldCommitted`
//! line (with the committed clamped number). The app exits ONLY after the PNG is confirmed
//! written AND both commit log lines have fired — with a frame cap so it always exits.

use std::{env, path::PathBuf};

use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::{FocusCause, InputFocus},
    log::LogPlugin,
    prelude::*,
    render::view::window::screenshot::{Screenshot, save_to_disk},
    ui::{GlobalZIndex, Node, PositionType, Val},
    window::PrimaryWindow,
};
use gdtf_ui::{
    CommittedTextValue, FieldColors, NumericFieldCommitted, NumericRange, TextFieldCommitted,
    UiPlugin, register_numeric_field, register_text_field, spawn_numeric_field, spawn_panel,
    spawn_text_field, theme::default_theme,
};

/// Marker on the demo text field so the auto-focus / commit-drive systems find exactly it.
#[derive(Component)]
struct DemoTextField;

/// Marker on the demo numeric field so the commit-drive system can focus + Enter it.
#[derive(Component)]
struct DemoNumericField;

/// Resource holding the optional self-screenshot output path (from `GDTF_TEXTFIELD_SHOT`).
#[derive(Resource)]
struct ShotPath(Option<PathBuf>);

/// Counts elapsed update frames so the demo captures after the UI has had time to lay out.
#[derive(Resource, Default)]
struct FrameCounter(u32);

/// Tracks whether the screenshot was already requested this run.
#[derive(Resource, Default)]
struct ShotRequested(bool);

/// Tracks whether the PNG has landed on disk (the screenshot's async GPU readback completed).
#[derive(Resource, Default)]
struct ShotWritten(bool);

/// How far the post-screenshot commit drive has progressed (which Enter has been written to
/// the keyboard stream through the production path).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum CommitPhase {
    /// Nothing kicked yet (waiting on the PNG).
    #[default]
    Idle,
    /// The text-field Enter has been written; waiting for its commit to be confirmed.
    TextKicked,
    /// The numeric-field Enter has been written; waiting for its commit to be confirmed.
    NumericKicked,
}

/// Drives the post-screenshot commit sequence: the [`CommitPhase`] (which Enter has been KICKED
/// through the production path) plus which commits have been CONFIRMED (their log lines fired).
#[derive(Resource, Default)]
struct CommitDrive {
    /// How far the Enter-drive has progressed.
    phase:             CommitPhase,
    /// A [`TextFieldCommitted`] was observed (the commit log line fired).
    text_confirmed:    bool,
    /// A [`NumericFieldCommitted`] was observed (the commit log line fired).
    numeric_confirmed: bool,
}

/// The frame the demo requests the screenshot (after a layout settle; the GPU readback is
/// ASYNC, so the commit drive + exit happen only after the file lands).
const SHOT_FRAME: u32 = 16;

/// Hard cap on total Update frames so the demo ALWAYS exits, even if a step (PNG readback, a
/// dispatched commit) never completes (at 60 fps ~10s — generous but finite, no hang).
const FRAME_CAP: u32 = 600;

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
        .insert_resource(ShotPath(shot))
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

/// Logs every text / numeric commit so QA can confirm a commit end-to-end (the values land in
/// the typed messages on Enter / blur), and flips the [`CommitDrive`] confirmation flags so the
/// exit only fires once both lines have printed.
fn log_commits(
    mut text_commits: MessageReader<TextFieldCommitted>,
    mut numeric_commits: MessageReader<NumericFieldCommitted<i64>>,
    mut drive: ResMut<CommitDrive>,
) {
    for commit in text_commits.read() {
        info!(
            "text_field_demo: TextFieldCommitted {{ field: {:?}, value: {:?} }}",
            commit.field(),
            commit.value().value(),
        );
        drive.text_confirmed = true;
    }
    for commit in numeric_commits.read() {
        info!(
            "text_field_demo: NumericFieldCommitted {{ field: {:?}, value: {} }}",
            commit.field(),
            commit.value().value(),
        );
        drive.numeric_confirmed = true;
    }
}

/// At [`SHOT_FRAME`] (if `GDTF_TEXTFIELD_SHOT` was set), requests a screenshot via the observer
/// API; the demo does NOT exit here — [`poll_for_png`] polls until the file appears, and only
/// THEN does [`drive_commits`] kick the commits.
fn maybe_capture(
    frames: Res<FrameCounter>,
    shot: Res<ShotPath>,
    mut requested: ResMut<ShotRequested>,
    mut commands: Commands,
) {
    if requested.0 {
        return;
    }
    let Some(path) = shot.0.clone() else {
        return;
    };
    if frames.0 != SHOT_FRAME {
        return;
    }
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
    requested.0 = true;
}

/// Polls until the PNG appears (written by the `save_to_disk` observer after GPU readback) and
/// sets [`ShotWritten`]. With no shot path set, marks it written immediately so the live /
/// no-screenshot run still proceeds to the commit drive.
fn poll_for_png(
    shot: Res<ShotPath>,
    requested: Res<ShotRequested>,
    mut written: ResMut<ShotWritten>,
) {
    if written.0 {
        return;
    }
    let Some(path) = shot.0.as_ref() else {
        // No screenshot requested (live run): treat the render evidence as "done" so the
        // commit drive still runs.
        written.0 = true;
        return;
    };
    if !requested.0 {
        return;
    }
    if std::path::Path::new(path).exists() {
        info!(
            "text_field_demo: PNG written to {}, driving commits.",
            path.display()
        );
        written.0 = true;
    }
}

/// AFTER the PNG has landed, drives the two commits through the REAL keyboard/focus path: focus
/// the text field and write an Enter `KeyboardInput` (the production `dispatch_focused_input`
/// triggers `handle_text_field_key` → `TextFieldCommitted` next frame), then — once the text
/// commit is confirmed — do the same for the numeric field. No commit message or buffer is
/// poked directly; this is the same input path the running app uses.
fn drive_commits(
    written: Res<ShotWritten>,
    mut drive: ResMut<CommitDrive>,
    mut focus: ResMut<InputFocus>,
    text_field: Query<Entity, With<DemoTextField>>,
    numeric_field: Query<Entity, With<DemoNumericField>>,
    window: Query<Entity, With<PrimaryWindow>>,
    mut keys: MessageWriter<KeyboardInput>,
) {
    if !written.0 {
        return;
    }
    let Ok(window) = window.single() else {
        return;
    };
    match drive.phase {
        // Step 1: commit the text field (already focused from `auto_focus`, but set it again so
        // the drive is self-contained). Write a single Enter through the real keyboard stream.
        CommitPhase::Idle => {
            if let Ok(field) = text_field.single() {
                focus.set(field, FocusCause::Navigated);
                keys.write(enter_key(window));
                drive.phase = CommitPhase::TextKicked;
            }
        }
        // Step 2: once the text commit has been observed, focus + Enter the numeric field.
        CommitPhase::TextKicked if drive.text_confirmed => {
            if let Ok(field) = numeric_field.single() {
                focus.set(field, FocusCause::Navigated);
                keys.write(enter_key(window));
                drive.phase = CommitPhase::NumericKicked;
            }
        }
        CommitPhase::TextKicked | CommitPhase::NumericKicked => {}
    }
}

/// Builds a real Enter `KeyboardInput` message (the same shape the OS keyboard device produces)
/// originating from `window`, so `dispatch_focused_input::<KeyboardInput>` routes it to the
/// focused field's observer.
const fn enter_key(window: Entity) -> KeyboardInput {
    KeyboardInput {
        key_code: KeyCode::Enter,
        logical_key: Key::Enter,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window,
    }
}

/// Exits cleanly ONLY after BOTH the PNG is written AND both commit log lines have fired; a
/// [`FRAME_CAP`] safety cap guarantees the demo never hangs if a step does not occur.
fn maybe_exit(
    written: Res<ShotWritten>,
    drive: Res<CommitDrive>,
    frames: Res<FrameCounter>,
    mut exit: MessageWriter<AppExit>,
) {
    if written.0 && drive.text_confirmed && drive.numeric_confirmed {
        info!("text_field_demo: PNG written and both commits confirmed, exiting.");
        exit.write(AppExit::Success);
        return;
    }
    if frames.0 >= FRAME_CAP {
        warn!(
            "text_field_demo: giving up after {} frames (png_written={}, text_committed={}, \
             numeric_committed={}). Was GDTF_TEXTFIELD_SHOT a writable path, and did the commit \
             dispatch run?",
            FRAME_CAP, written.0, drive.text_confirmed, drive.numeric_confirmed,
        );
        exit.write(AppExit::Success);
    }
}
