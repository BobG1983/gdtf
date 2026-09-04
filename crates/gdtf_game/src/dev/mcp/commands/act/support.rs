//! Pieces every classic act command is built from.

use bevy::{ecs::system::SystemParam, prelude::*};
use cobalt_mcp_command::{command::McpCommand, dispatch::DeferredReplies};
use gdtf_battle_input::{ActIntent, PendingActIntent, SelectedShooter};
use gdtf_battle_sim::{
    act_log::{ActDeed, ActEntry, ActLog, ActSeq},
    acts::movement::WalkInProgress,
    prelude::{Faction, LifeState},
};
use serde::Deserialize;

use crate::dev::mcp::wire::{
    act::{ActCompleteNet, ActRefusalNet, ActReply, ActSeqNet, SelectReply},
    deed::MoveRejectionNet,
    token::GangerToken,
};

/// Argument shape of an act that takes nothing.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NoArgs {}

/// What one parked act call needs to report the act-log window it opened.
pub(crate) struct ActTicket {
    from:  ActSeqNet,
    actor: Option<Entity>,
}

impl ActTicket {
    /// Park with the log head at claim time and the actor whose walk to watch.
    pub(in crate::dev::mcp::commands) const fn new(from: ActSeqNet, actor: Option<Entity>) -> Self {
        Self { from, actor }
    }

    /// The act-log head as it stood when the call was claimed.
    pub(in crate::dev::mcp::commands) const fn claimed_at(&self) -> ActSeqNet {
        self.from
    }

    /// The actor whose walk the reply reports on, when the call had one.
    pub(in crate::dev::mcp::commands) const fn actor(&self) -> Option<Entity> {
        self.actor
    }
}

/// What an act command reads and writes as it claims its calls.
#[derive(SystemParam)]
pub(super) struct ActClaim<'w> {
    log:      Option<Res<'w, ActLog>>,
    pending:  ResMut<'w, PendingActIntent>,
    selected: Option<Res<'w, SelectedShooter>>,
}

impl ActClaim<'_> {
    /// The act-log head as it stands before the sim runs this frame.
    pub(super) fn head(&self) -> ActSeqNet {
        head_of(self.log.as_deref())
    }

    /// The selected shooter, when the game has one.
    pub(super) fn shooter(&self) -> Option<Entity> {
        self.selected.as_deref().and_then(|selected| **selected)
    }

    /// Queue an intent onto the same bus the keyboard and the pointer push onto.
    pub(super) fn push(&mut self, intent: ActIntent) {
        self.pending.push(intent);
    }
}

/// What an act command reads once the sim has recorded the frame.
#[derive(SystemParam)]
pub(in crate::dev::mcp::commands) struct ActSettle<'w, 's> {
    log:     Option<Res<'w, ActLog>>,
    walking: Query<'w, 's, (), With<WalkInProgress>>,
}

impl ActSettle<'_, '_> {
    /// The act-log head as it stands once the sim has recorded the frame.
    pub(in crate::dev::mcp::commands) fn head(&self) -> ActSeqNet {
        head_of(self.log.as_deref())
    }

    /// Whether the actor has finished the act rather than still walking it out.
    pub(in crate::dev::mcp::commands) fn completed(&self, actor: Option<Entity>) -> ActCompleteNet {
        ActCompleteNet::new(actor.is_none_or(|actor| self.walking.get(actor).is_err()))
    }

    /// The window one parked call opened, closed at this frame's log head.
    pub(in crate::dev::mcp::commands) fn window(
        &self,
        from_seq: ActSeqNet,
        actor: Option<Entity>,
    ) -> ActReply {
        ActReply::Accepted {
            from_seq,
            to_seq: self.head(),
            complete: self.completed(actor),
        }
    }

    /// The window one parked ticket opened, closed at this frame's log head.
    pub(in crate::dev::mcp::commands) fn window_of(&self, ticket: &ActTicket) -> ActReply {
        self.window(ticket.from, ticket.actor)
    }

    /// What the ticket's actor recorded from the call's own log head onwards.
    pub(in crate::dev::mcp::commands) fn deeds_of<'a>(
        &'a self,
        ticket: &'a ActTicket,
    ) -> impl Iterator<Item = &'a ActDeed> {
        let actor = ticket.actor;
        self.log
            .as_deref()
            .into_iter()
            .flat_map(move |log| log.since(ActSeq::new(*ticket.from)))
            .filter(move |entry| actor.is_none_or(|actor| entry.actor() == actor))
            .map(ActEntry::deed)
    }
}

/// The refusal the sim recorded for this move, or the window the call opened.
pub(in crate::dev::mcp::commands) fn move_reply(
    settle: &ActSettle,
    ticket: &ActTicket,
) -> ActReply {
    let refused = settle.deeds_of(ticket).find_map(|deed| {
        let ActDeed::MoveRefused { reason } = deed else {
            return None;
        };
        Some(MoveRejectionNet::from_sim(*reason))
    });
    match refused {
        Some(reason) => ActReply::MoveRefused { reason },
        None => settle.window_of(ticket),
    }
}

/// Answer every act call parked this frame with the window the sim just closed.
pub(in crate::dev::mcp::commands) fn settle_acts<C>(
    settle: &ActSettle,
    deferred: &mut DeferredReplies<C>,
) where
    C: McpCommand<Parked = ActTicket, Reply = ActReply>,
{
    settle_acts_with::<C>(deferred, |ticket| settle.window_of(ticket));
}

/// Answer every act call parked this frame with the reply `reply` reads out of the frame.
pub(in crate::dev::mcp::commands) fn settle_acts_with<C>(
    deferred: &mut DeferredReplies<C>,
    mut reply: impl FnMut(&ActTicket) -> ActReply,
) where
    C: McpCommand<Parked = ActTicket, Reply = ActReply>,
{
    if deferred.is_empty() {
        return;
    }
    let delivered = deferred.answer_resolved(|ticket| Some(reply(ticket)));
    debug!(
        command = C::NAME.as_str(),
        delivered = *delivered,
        "mcp: an act reported the log window it opened"
    );
}

/// Answer every selection call parked this frame with who is selected now.
pub(super) fn settle_selects<C>(
    selected: Option<&SelectedShooter>,
    deferred: &mut DeferredReplies<C>,
) where
    C: McpCommand<Parked = (), Reply = SelectReply>,
{
    if deferred.is_empty() {
        return;
    }
    let delivered = deferred.answer_all(&SelectReply::Selected {
        shooter: selected
            .and_then(|selected| **selected)
            .map(|entity| GangerToken::new(entity.to_bits())),
    });
    debug!(
        command = C::NAME.as_str(),
        delivered = *delivered,
        "mcp: a selection command reported the live selection"
    );
}

/// The refusal an act answers when it needs a selected shooter and there is none.
pub(super) const NO_SHOOTER: ActReply = ActReply::Refused {
    reason: ActRefusalNet::NoShooter,
};

/// The living ganger a token names, when the world still holds one under those bits.
pub(in crate::dev::mcp::commands) fn a_ganger(
    gangers: &Query<Option<&LifeState>, With<Faction>>,
    token: GangerToken,
) -> Option<Entity> {
    let entity = Entity::try_from_bits(*token)?;
    let life = gangers.get(entity).ok()?;
    life.is_none_or(|life| *life.is_active()).then_some(entity)
}

/// The act-log head, or zero on a host with no log yet.
pub(in crate::dev::mcp::commands) fn head_of(log: Option<&ActLog>) -> ActSeqNet {
    ActSeqNet::new(log.map_or(0, |log| *log.head()))
}
