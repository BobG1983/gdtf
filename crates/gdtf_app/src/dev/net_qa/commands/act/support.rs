//! Pieces every classic act command is built from.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_input::{ActIntent, PendingActIntent, SelectedShooter};
use gdtf_battle_sim::{
    act_log::ActLog,
    acts::movement::WalkInProgress,
    prelude::{Faction, LifeState},
};
use gdtf_qa_command::{command::QaCommand, dispatch::DeferredReplies};
use serde::Deserialize;

use crate::dev::net_qa::wire::{
    act::{ActCompleteNet, ActRefusalNet, ActReply, ActSeqNet, SelectReply},
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
    pub(in crate::dev::net_qa::commands) const fn new(
        from: ActSeqNet,
        actor: Option<Entity>,
    ) -> Self {
        Self { from, actor }
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
pub(in crate::dev::net_qa::commands) struct ActSettle<'w, 's> {
    log:     Option<Res<'w, ActLog>>,
    walking: Query<'w, 's, (), With<WalkInProgress>>,
}

impl ActSettle<'_, '_> {
    /// The window one parked call opened, closed at this frame's log head.
    pub(in crate::dev::net_qa::commands) fn window(
        &self,
        from_seq: ActSeqNet,
        actor: Option<Entity>,
    ) -> ActReply {
        ActReply::Accepted {
            from_seq,
            to_seq: head_of(self.log.as_deref()),
            complete: ActCompleteNet::new(
                actor.is_none_or(|actor| self.walking.get(actor).is_err()),
            ),
        }
    }
}

/// Answer every act call parked this frame with the window the sim just closed.
pub(in crate::dev::net_qa::commands) fn settle_acts<C>(
    settle: &ActSettle,
    deferred: &mut DeferredReplies<C>,
) where
    C: QaCommand<Parked = ActTicket, Reply = ActReply>,
{
    if deferred.is_empty() {
        return;
    }
    let delivered =
        deferred.answer_resolved(|ticket| Some(settle.window(ticket.from, ticket.actor)));
    debug!(
        command = C::NAME.as_str(),
        delivered = *delivered,
        "net_qa: an act reported the log window it opened"
    );
}

/// Answer every selection call parked this frame with who is selected now.
pub(super) fn settle_selects<C>(
    selected: Option<&SelectedShooter>,
    deferred: &mut DeferredReplies<C>,
) where
    C: QaCommand<Parked = (), Reply = SelectReply>,
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
        "net_qa: a selection command reported the live selection"
    );
}

/// The refusal an act answers when it needs a selected shooter and there is none.
pub(super) const NO_SHOOTER: ActReply = ActReply::Refused {
    reason: ActRefusalNet::NoShooter,
};

/// The living ganger a token names, when the world still holds one under those bits.
pub(super) fn a_ganger(
    gangers: &Query<Option<&LifeState>, With<Faction>>,
    token: GangerToken,
) -> Option<Entity> {
    let entity = Entity::try_from_bits(*token)?;
    let life = gangers.get(entity).ok()?;
    life.is_none_or(|life| *life.is_active()).then_some(entity)
}

/// The act-log head, or zero on a host with no log yet.
pub(in crate::dev::net_qa::commands) fn head_of(log: Option<&ActLog>) -> ActSeqNet {
    ActSeqNet::new(log.map_or(0, |log| *log.head()))
}
