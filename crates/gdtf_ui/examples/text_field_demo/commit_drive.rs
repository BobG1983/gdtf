//! Commit-evidence drive: the phased Enter injection through the production
//! keyboard/focus path, the `info!`-logged commit confirmations, and the exit gate.

use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    window::PrimaryWindow,
};
use gdtf_ui::{NumericFieldCommitted, TextFieldCommitted};

use crate::{DemoNumericField, DemoTextField, FrameCounter, capture::ShotWritten};

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
pub(crate) struct CommitDrive {
    /// How far the Enter-drive has progressed.
    phase:             CommitPhase,
    /// A [`TextFieldCommitted`] was observed (the commit log line fired).
    text_confirmed:    bool,
    /// A [`NumericFieldCommitted`] was observed (the commit log line fired).
    numeric_confirmed: bool,
}

/// Hard cap on total Update frames so the demo ALWAYS exits, even if a step (PNG readback, a
/// dispatched commit) never completes (at 60 fps ~10s — generous but finite, no hang).
const FRAME_CAP: u32 = 600;

/// Logs every text / numeric commit so QA can confirm a commit end-to-end (the values land in
/// the typed messages on Enter / blur), and flips the [`CommitDrive`] confirmation flags so the
/// exit only fires once both lines have printed.
pub(crate) fn log_commits(
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

/// AFTER the PNG has landed, drives the two commits through the REAL keyboard/focus path: focus
/// the text field and write an Enter `KeyboardInput` (the production `dispatch_focused_input`
/// triggers `handle_text_field_key` → `TextFieldCommitted` next frame), then — once the text
/// commit is confirmed — do the same for the numeric field. No commit message or buffer is
/// poked directly; this is the same input path the running app uses.
pub(crate) fn drive_commits(
    written: Res<ShotWritten>,
    mut drive: ResMut<CommitDrive>,
    mut focus: ResMut<InputFocus>,
    text_field: Query<Entity, With<DemoTextField>>,
    numeric_field: Query<Entity, With<DemoNumericField>>,
    window: Query<Entity, With<PrimaryWindow>>,
    mut keys: MessageWriter<KeyboardInput>,
) {
    if !written.is_written() {
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
pub(crate) fn maybe_exit(
    written: Res<ShotWritten>,
    drive: Res<CommitDrive>,
    frames: Res<FrameCounter>,
    mut exit: MessageWriter<AppExit>,
) {
    if written.is_written() && drive.text_confirmed && drive.numeric_confirmed {
        info!("text_field_demo: PNG written and both commits confirmed, exiting.");
        exit.write(AppExit::Success);
        return;
    }
    if frames.count() >= FRAME_CAP {
        warn!(
            "text_field_demo: giving up after {} frames (png_written={}, text_committed={}, \
             numeric_committed={}). Was GDTF_TEXTFIELD_SHOT a writable path, and did the commit \
             dispatch run?",
            FRAME_CAP,
            written.is_written(),
            drive.text_confirmed,
            drive.numeric_confirmed,
        );
        exit.write(AppExit::Success);
    }
}
