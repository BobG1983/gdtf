use bevy::{ecs::system::SystemParam, prelude::*};
use cobalt_mcp_command::{
    command::QaCommand,
    dispatch::{CommandCall, DeferredReplies, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_battle_input::{
    LeftClickOutcome, LeftClickReads, PendingActIntent, PointerSelection, ShooterArms,
    apply_left_click, apply_pin, battle_act_gate, decide_left_click, decide_pin, pick_hovered_cell,
};
use gdtf_battle_sim::{
    act_log::ActLog,
    prelude::{CellLevel, Faction, LifeState},
};
use serde::Deserialize;

use crate::dev::mcp::{
    commands::{
        act::{
            ActCommandSystems,
            support::{ActSettle, ActTicket, head_of, move_reply},
        },
        read::availability::running_and_caught,
    },
    facts::GameFacts,
    wire::{
        act::{ActReply, ActSeqNet},
        cell::CellLevelNet,
        click::{ClickDecisionNet, ClickReply},
    },
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InputClickCellArgs {
    /// Cell-level to click, as `battle.roster` and `battle.visible` report cells.
    at: CellLevelNet,
}

pub(crate) struct InputClickCell;

impl QaCommand for InputClickCell {
    type Args = InputClickCellArgs;
    type Facts = GameFacts;
    type Parked = ClickTicket;
    type Reply = ClickReply;

    const NAME: CommandName = CommandName::from_static("input.click_cell");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Left-click a cell, taking the same decide-then-apply path the mouse and the gamepad \
         take: the cell becomes the hovered inspect target, the click decision runs on it, and \
         both the selection and the inspect pin are updated from the result. So one click may \
         select a ganger, pin a move target, confirm the move, or fire — the decision is the \
         game's, not the caller's. The reply names that decision, and carries the act reply for \
         the two decisions that push an act; read the resulting selection and pin with \
         battle.selection.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        running_and_caught(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            (
                claim_input_click_cell
                    .in_set(ActCommandSystems::Claim)
                    .before(pick_hovered_cell)
                    .run_if(battle_act_gate()),
                settle_input_click_cell.in_set(ActCommandSystems::Settle),
            ),
        );
    }
}

/// What one click writes as it is claimed: the act bus, plus the log head it opened at.
#[derive(SystemParam)]
struct ClickClaim<'w> {
    log:     Option<Res<'w, ActLog>>,
    pending: ResMut<'w, PendingActIntent>,
}

impl ClickClaim<'_> {
    /// The act-log head as it stands before the sim runs this frame.
    fn head(&self) -> ActSeqNet {
        head_of(self.log.as_deref())
    }
}

/// The pointer state a left click reads and then updates.
#[derive(SystemParam)]
struct ClickPointer<'w, 's> {
    reads:     LeftClickReads<'w>,
    factions:  Query<'w, 's, &'static Faction>,
    lifes:     Query<'w, 's, &'static LifeState>,
    arms:      ShooterArms<'w, 's>,
    selection: PointerSelection<'w>,
}

/// What one clicked call needs to report: the decision, and the act it started.
pub(crate) struct ClickTicket {
    act:      ActTicket,
    decision: ClickDecisionNet,
}

impl ClickPointer<'_, '_> {
    /// Click `at`, answering what the game decided and whose walk the reply waits on.
    fn click(
        &mut self,
        at: CellLevel,
        pending: &mut ResMut<PendingActIntent>,
    ) -> (ClickDecisionNet, Option<Entity>) {
        if self.selection.inspect().hovered() != Some(at) {
            self.selection.inspect_mut().set_hovered(Some(at));
        }
        let outcome = decide_left_click(
            &self.reads,
            &self.selection,
            &self.factions,
            &self.lifes,
            &self.arms,
        );
        let decision = ClickDecisionNet::from_game(&outcome);
        let actor = actor_of(&outcome);
        let pin = decide_pin(&self.reads, self.selection.inspect(), &self.factions);
        apply_left_click(outcome, &mut self.selection, pending);
        apply_pin(pin, &mut self.selection);
        (decision, actor)
    }
}

/// The actor whose act the click started, when it started one.
const fn actor_of(outcome: &LeftClickOutcome) -> Option<Entity> {
    match outcome {
        LeftClickOutcome::Fire(request) => Some(request.shooter),
        LeftClickOutcome::Move(request) => Some(request.actor),
        LeftClickOutcome::Select(_)
        | LeftClickOutcome::SetMoveTarget(_)
        | LeftClickOutcome::NoOp
        | LeftClickOutcome::Clear => None,
    }
}

fn claim_input_click_cell(
    mut queue: ResMut<PendingQueue<CommandCall<InputClickCell>>>,
    mut deferred: ResMut<DeferredReplies<InputClickCell>>,
    mut claim: ClickClaim,
    mut pointer: ClickPointer,
) {
    if queue.is_empty() {
        return;
    }
    let from = claim.head();
    for (args, responder) in take_calls::<InputClickCell>(&mut queue) {
        let (decision, actor) = pointer.click(args.at.to_sim(), &mut claim.pending);
        deferred.park(
            responder,
            ClickTicket {
                act: ActTicket::new(from, actor),
                decision,
            },
        );
    }
}

fn settle_input_click_cell(
    settle: ActSettle,
    mut deferred: ResMut<DeferredReplies<InputClickCell>>,
) {
    if deferred.is_empty() {
        return;
    }
    let delivered = deferred.answer_resolved(|ticket| {
        Some(ClickReply {
            decision: ticket.decision,
            act:      click_act(&settle, ticket),
        })
    });
    debug!(
        command = InputClickCell::NAME.as_str(),
        delivered = *delivered,
        "mcp: a click reported what it decided"
    );
}

/// The act reply the click's own decision earned, absent when it pushed no act.
fn click_act(settle: &ActSettle, ticket: &ClickTicket) -> Option<ActReply> {
    match ticket.decision {
        ClickDecisionNet::Fire => Some(settle.window_of(&ticket.act)),
        ClickDecisionNet::Move => Some(move_reply(settle, &ticket.act)),
        ClickDecisionNet::Select
        | ClickDecisionNet::SetMoveTarget
        | ClickDecisionNet::NoOp
        | ClickDecisionNet::Clear => None,
    }
}
