//! Shared decoding the classic-act socket cases share.

use bevy::app::App;
use gdtf_app::qa_wire::{
    act::{ActCompleteNet, ActReply, ActSeqNet, SelectReply},
    deed::ActDeedKindNet,
    log::LogEntryNet,
    roster::GangerCardNet,
    token::GangerToken,
};
use gdtf_qa_protocol::{
    command::RunOptions,
    message::{QaRequest, QaResponse},
    ports::NetQaPort,
};
use serde::Deserialize;

use super::{
    command_exchange::{WAIT, ran_body, run},
    socket_support::{TestError, battle_app_listening},
};

/// An act closes the playback gate until the screen has replayed it, which is what a client waits on.
pub(crate) fn caught_up() -> QaRequest {
    run(WAIT, "(condition:CaughtUp)", RunOptions::default())
}

/// A running-battle socket fixture that also reports what `read` picked out of the live world.
pub(crate) fn battle_app_reporting<T>(
    read: impl FnOnce(&App) -> Option<T>,
    missing: &'static str,
) -> impl FnOnce() -> Result<(App, NetQaPort, T), TestError> {
    move || {
        let (app, port) = battle_app_listening()?;
        let chosen = read(&app).ok_or_else(|| TestError::from(missing))?;
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
pub(crate) fn walk_complete() -> QaRequest {
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
            .filter(|entry| entry.actor == actor && entry.seq >= from && entry.seq < to)
            .map(|entry| entry.kind)
            .collect()
    }
}

/// The act window a reply carries, or a failure naming the refusal that came back instead.
pub(crate) fn accepted(name: &'static str, reply: QaResponse) -> Result<ActReply, TestError> {
    let decoded = decode::<ActReply>(name, reply)?;
    match decoded {
        ActReply::Accepted { .. } => Ok(decoded),
        ActReply::Refused { reason } => {
            Err(format!("`{name}` must be accepted here, it was refused: {reason:?}").into())
        }
    }
}

/// The `from_seq..to_seq` bounds of an accepted act.
pub(crate) const fn window(reply: ActReply) -> Option<(ActSeqNet, ActSeqNet)> {
    match reply {
        ActReply::Accepted {
            from_seq, to_seq, ..
        } => Some((from_seq, to_seq)),
        ActReply::Refused { .. } => None,
    }
}

/// Whether an accepted act finished inside its frame rather than still walking itself out.
pub(crate) const fn complete(reply: ActReply) -> Option<ActCompleteNet> {
    match reply {
        ActReply::Accepted { complete, .. } => Some(complete),
        ActReply::Refused { .. } => None,
    }
}

/// Who a selection command says is selected, or a failure naming the refusal.
pub(crate) fn selected(
    name: &'static str,
    reply: QaResponse,
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
    reply: QaResponse,
) -> Result<T, TestError> {
    let body = ran_body(name, reply)?;
    ron::de::from_str::<T>(&body)
        .map_err(|fault| format!("`{name}`'s body must decode: {fault} — {body}").into())
}

/// Take the next reply out of an exchange, naming the command that owed it.
pub(crate) fn next(
    name: &'static str,
    replies: &mut impl Iterator<Item = QaResponse>,
) -> Result<QaResponse, TestError> {
    replies
        .next()
        .ok_or_else(|| format!("`{name}` produced no reply").into())
}
