//! Shared decoding the classic-act socket cases share.

use bevy::app::App;
use cobalt_mcp_protocol::{
    command::RunOptions,
    message::{McpRequest, McpResponse},
    ports::McpPort,
};
use gdtf_game::qa_wire::{
    WaitConditionNet,
    act::{ActCompleteNet, ActReply, ActSeqNet, SelectReply},
    deed::ActDeedKindNet,
    log::LogEntryNet,
    roster::GangerCardNet,
    token::GangerToken,
};
use serde::Deserialize;

use super::{
    battle_setup::let_the_screen_catch_up,
    command_exchange::{WAIT, ran_body, run},
    socket_support::{TestError, battle_app_listening},
};

/// An act closes the playback gate until the screen has replayed it, which is what a client waits on.
pub(crate) fn caught_up() -> McpRequest {
    run(WAIT, "(condition:CaughtUp)", RunOptions::default())
}

/// The part of a `wait` reply these cases read: the condition that came true.
#[derive(Debug, Deserialize)]
struct WaitedBody {
    condition: WaitConditionNet,
}

/// Fail unless the wait ran and named the condition it was held for, rather than giving up.
pub(crate) fn assert_waited_for(
    condition: WaitConditionNet,
    reply: McpResponse,
) -> Result<(), TestError> {
    let answered = decode::<WaitedBody>(WAIT, reply)?;
    if answered.condition == condition {
        return Ok(());
    }
    Err(format!(
        "`{WAIT}` answers with the condition that came true, and the commands after it are only \
         worth reading once that was `{condition:?}`: {answered:?}"
    )
    .into())
}

/// Fail unless the catch-up wait ran and named `CaughtUp`, rather than giving up.
pub(crate) fn assert_caught_up(reply: McpResponse) -> Result<(), TestError> {
    assert_waited_for(WaitConditionNet::CaughtUp, reply)
}

/// A running-battle socket fixture that also reports what `read` picked out of the live world.
pub(crate) fn battle_app_reporting<T>(
    read: impl FnOnce(&App) -> Option<T>,
    missing: &'static str,
) -> impl FnOnce() -> Result<(App, McpPort, T), TestError> {
    move || {
        let (app, port) = battle_app_listening()?;
        let chosen = read(&app).ok_or_else(|| TestError::from(missing))?;
        Ok((app, port, chosen))
    }
}

/// A running-battle socket fixture that writes the live world, then lets the screen catch up.
pub(crate) fn battle_app_prepared<T>(
    prepare: impl FnOnce(&mut App) -> Option<T>,
    missing: &'static str,
) -> impl FnOnce() -> Result<(App, McpPort, T), TestError> {
    move || {
        let (mut app, port) = battle_app_listening()?;
        let chosen = prepare(&mut app).ok_or_else(|| TestError::from(missing))?;
        let_the_screen_catch_up(&mut app)?;
        Ok((app, port, chosen))
    }
}

/// The body `battle.roster` answers with, as far as these cases read it.
#[derive(Debug, Deserialize)]
pub(crate) struct RosterBody {
    pub(crate) gangers: Vec<GangerCardNet>,
}

/// The part of `battle.selection` the act cases read: who the game has selected right now.
#[derive(Debug, Deserialize)]
pub(crate) struct SelectionBody {
    pub(crate) shooter: Option<GangerToken>,
}

/// The card the roster draws for one ganger.
pub(crate) fn card_of(roster: &RosterBody, token: GangerToken) -> Option<&GangerCardNet> {
    roster.gangers.iter().find(|card| card.token == token)
}

/// Wait until no ganger is part-way through a walk, which is what a client waits on after a move.
pub(crate) fn walk_complete() -> McpRequest {
    run(WAIT, "(condition:WalkComplete)", RunOptions::default())
}

/// The body `log.read` answers with, as far as these cases read it.
#[derive(Debug, Deserialize)]
pub(crate) struct LogBody {
    pub(crate) entries: Vec<LogEntryNet>,
}

impl LogBody {
    /// Every deed one actor logged inside the half-open window an act reply opened.
    pub(crate) fn deeds_by(
        &self,
        actor: GangerToken,
        window: (ActSeqNet, ActSeqNet),
    ) -> Vec<ActDeedKindNet> {
        let (from, to) = window;
        self.entries
            .iter()
            .filter(|entry| entry.actor == Some(actor) && entry.seq >= from && entry.seq < to)
            .map(|entry| entry.kind)
            .collect()
    }

    /// Every deed one actor logged, whichever window it fell in.
    pub(crate) fn deeds_from(&self, actor: GangerToken) -> Vec<ActDeedKindNet> {
        self.entries
            .iter()
            .filter(|entry| entry.actor == Some(actor))
            .map(|entry| entry.kind)
            .collect()
    }
}

/// The act window a reply carries, or a failure naming the refusal that came back instead.
pub(crate) fn accepted(name: &'static str, reply: McpResponse) -> Result<ActReply, TestError> {
    let decoded = decode::<ActReply>(name, reply)?;
    match decoded {
        ActReply::Accepted { .. } => Ok(decoded),
        ActReply::Refused { .. }
        | ActReply::FireRefused { .. }
        | ActReply::ReloadRefused { .. }
        | ActReply::MoveRefused { .. }
        | ActReply::StanceRefused { .. }
        | ActReply::FacingRefused { .. } => {
            Err(format!("`{name}` must be accepted here, it was refused: {decoded:?}").into())
        }
    }
}

/// The reply an act answered, whichever shape it took.
pub(crate) fn answered(name: &'static str, reply: McpResponse) -> Result<ActReply, TestError> {
    decode::<ActReply>(name, reply)
}

/// The `from_seq..to_seq` bounds of an accepted act.
pub(crate) const fn window(reply: ActReply) -> Option<(ActSeqNet, ActSeqNet)> {
    match reply {
        ActReply::Accepted {
            from_seq, to_seq, ..
        } => Some((from_seq, to_seq)),
        ActReply::Refused { .. }
        | ActReply::FireRefused { .. }
        | ActReply::ReloadRefused { .. }
        | ActReply::MoveRefused { .. }
        | ActReply::StanceRefused { .. }
        | ActReply::FacingRefused { .. } => None,
    }
}

/// Whether an accepted act finished inside its frame rather than still walking itself out.
pub(crate) const fn complete(reply: ActReply) -> Option<ActCompleteNet> {
    match reply {
        ActReply::Accepted { complete, .. } => Some(complete),
        ActReply::Refused { .. }
        | ActReply::FireRefused { .. }
        | ActReply::ReloadRefused { .. }
        | ActReply::MoveRefused { .. }
        | ActReply::StanceRefused { .. }
        | ActReply::FacingRefused { .. } => None,
    }
}

/// Who a selection command says is selected, or a failure naming the refusal.
pub(crate) fn selected(
    name: &'static str,
    reply: McpResponse,
) -> Result<Option<GangerToken>, TestError> {
    match decode::<SelectReply>(name, reply)? {
        SelectReply::Selected { shooter } => Ok(shooter),
        SelectReply::Refused { reason } => {
            Err(format!("`{name}` must be accepted here, it was refused: {reason:?}").into())
        }
    }
}

/// Decode a reply body into the shape the command publishes.
pub(crate) fn decode<T: serde::de::DeserializeOwned>(
    name: &'static str,
    reply: McpResponse,
) -> Result<T, TestError> {
    let body = ran_body(name, reply)?;
    ron::de::from_str::<T>(&body)
        .map_err(|fault| format!("`{name}`'s body must decode: {fault} — {body}").into())
}

/// Take the next reply out of an exchange, naming the command that owed it.
pub(crate) fn next(
    name: &'static str,
    replies: &mut impl Iterator<Item = McpResponse>,
) -> Result<McpResponse, TestError> {
    replies
        .next()
        .ok_or_else(|| format!("`{name}` produced no reply").into())
}
