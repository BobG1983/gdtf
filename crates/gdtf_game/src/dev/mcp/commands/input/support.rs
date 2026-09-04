//! Pieces the three focus commands are built from.

use bevy::{input_focus::InputFocus, prelude::*};
use cobalt_mcp_host::{command::McpCommand, dispatch::DeferredReplies};
use serde::{Deserialize, Serialize};

use crate::dev::mcp::wire::token::FocusTargetNet;

/// What the three focus commands answer: which widget holds focus once the frame settled.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct FocusReply {
    /// Focused widget, absent when nothing holds focus.
    focused: Option<FocusTargetNet>,
}

/// The token `ui.focus` names this widget by.
pub(super) const fn focus_token(entity: Entity) -> FocusTargetNet {
    FocusTargetNet::new(entity.to_bits())
}

/// Answer every focus call parked this frame with the focus the frame settled on.
pub(super) fn settle_focus<C>(focus: &InputFocus, deferred: &mut DeferredReplies<C>)
where
    C: McpCommand<Parked = (), Reply = FocusReply>,
{
    if deferred.is_empty() {
        return;
    }
    let delivered = deferred.answer_all(&FocusReply {
        focused: focus.get().map(focus_token),
    });
    debug!(
        command = C::NAME.as_str(),
        delivered = *delivered,
        "mcp: a focus command reported the focus its frame settled on"
    );
}
