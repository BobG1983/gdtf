use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_input::{
    LeftClickOutcome, LeftClickReads, PendingActIntent, PointerSelection, ShooterArms,
    apply_left_click, apply_pin, battle_act_gate, decide_left_click, decide_pin, pick_hovered_cell,
};
use gdtf_battle_sim::{
    act_log::ActLog,
    prelude::{CellLevel, Faction, LifeState},
};
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, DeferredReplies, take_calls},
};
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};
use serde::Deserialize;

use crate::dev::net_qa::{
    commands::{
        act::{
            ActCommandSystems,
            support::{ActSettle, ActTicket, head_of, settle_acts},
        },
        read::availability::running_and_caught,
    },
    facts::GameFacts,
    wire::{
        act::{ActReply, ActSeqNet},
        cell::CellLevelNet,
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
    type Parked = ActTicket;
    type Reply = ActReply;

    const NAME: CommandName = CommandName::from_static("input.click_cell");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Left-click a cell, taking the same decide-then-apply path the mouse and the gamepad \
         take: the cell becomes the hovered inspect target, the click decision runs on it, and \
         both the selection and the inspect pin are updated from the result. So one click may \
         select a ganger, pin a move target, confirm the move, or fire — the decision is the \
         game's, not the caller's. The reply brackets the act log the way every act.* command \
         does; read the resulting selection and pin with battle.selection.",
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

impl ClickPointer<'_, '_> {
    /// Click `at`, answering the actor whose walk the reply must wait on.
    fn click(&mut self, at: CellLevel, pending: &mut ResMut<PendingActIntent>) -> Option<Entity> {
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
        let walker = match &outcome {
            LeftClickOutcome::Move(request) => Some(request.actor),
            _ => None,
        };
        let pin = decide_pin(&self.reads, self.selection.inspect(), &self.factions);
        apply_left_click(outcome, &mut self.selection, pending);
        apply_pin(pin, &mut self.selection);
        walker
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
        let walker = pointer.click(args.at.to_sim(), &mut claim.pending);
        deferred.park(responder, ActTicket::new(from, walker));
    }
}

fn settle_input_click_cell(
    settle: ActSettle,
    mut deferred: ResMut<DeferredReplies<InputClickCell>>,
) {
    settle_acts::<InputClickCell>(&settle, &mut deferred);
}
