//! Shared decoding and argument shaping the raw-input socket cases share.

use gdtf_app::qa_wire::{
    cell::CellLevelNet,
    key::{KeyNet, KeyPressNet},
    token::{FocusTargetNet, GangerToken},
};
use gdtf_qa_protocol::{
    command::{CommandOutcome, UnavailableCode},
    message::QaResponse,
};
use serde::Deserialize;

use super::socket_support::TestError;

/// The body the three focus commands answer with.
#[derive(Debug, Deserialize)]
pub(crate) struct FocusBody {
    pub(crate) focused: Option<FocusTargetNet>,
}

/// The body `ui.focus` answers with, as far as these cases read it.
#[derive(Debug, Deserialize)]
pub(crate) struct UiFocusBody {
    pub(crate) focused:   Option<FocusTargetNet>,
    pub(crate) focusable: Vec<FocusTargetNet>,
}

/// The body `input.press_key` answers with.
#[derive(Debug, Deserialize)]
pub(crate) struct PressedKeyBody {
    pub(crate) key: KeyNet,
}

/// The part of `battle.selection` the click cases read.
#[derive(Debug, Deserialize)]
pub(crate) struct ClickedSelectionBody {
    pub(crate) shooter: Option<GangerToken>,
    pub(crate) pinned:  Option<CellLevelNet>,
}

/// Render a focus token as the compact RON body `input.set_focus` takes.
pub(crate) fn focus_argument(target: FocusTargetNet) -> String {
    format!("(target:{})", *target)
}

/// Render a key press as the compact RON body `input.press_key` takes.
pub(crate) fn key_argument(press: KeyPressNet) -> String {
    let Ok(text) = ron::ser::to_string(&press) else {
        unreachable!("a wire key press serializes to compact RON");
    };
    format!("(key:{text})")
}

/// The code and note a refusal carries, or a failure naming what came back instead.
pub(crate) fn refusal(
    name: &str,
    reply: QaResponse,
) -> Result<(UnavailableCode, String), TestError> {
    match reply {
        QaResponse::Outcome(CommandOutcome::Unavailable { code, note }) => {
            Ok((code, note.as_str().to_owned()))
        }
        other => Err(format!("`{name}` must refuse here, got {other:?}").into()),
    }
}
