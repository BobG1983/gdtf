use bevy::prelude::*;
#[cfg(feature = "dev_tools")]
use gdtf_battle_sim::procgen::StagedProcgen;
#[cfg(feature = "dev_tools")]
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::command::QaCommand;
#[cfg(feature = "dev_tools")]
use gdtf_qa_command::dispatch::{CommandCall, QaCommandSystems, take_calls};
use gdtf_qa_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use serde::{Deserialize, Serialize};

use crate::dev::net_qa::{facts::GameFacts, wire::misc::ProcgenStageNet};
#[cfg(feature = "dev_tools")]
use crate::dev::{
    net_qa::{facts::StepperActivity, wire::BattleScapePhaseNet},
    procgen_stepper::{PendingStepCommand, StepCommand},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProcgenStepArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ProcgenStepReply {
    stage: ProcgenStageNet,
}

pub(crate) struct ProcgenStep;

impl QaCommand for ProcgenStep {
    type Args = ProcgenStepArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = ProcgenStepReply;

    const NAME: CommandName = CommandName::from_static("procgen.step");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Advance staged situation generation by one stage — the same request the stepper panel's \
         Next button makes. Needs the procgen stepper to own a situation that is still \
         generating; the reply names the stage the driver was on when the step was queued.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        stepper_availability(*facts)
    }

    fn register_handler(app: &mut App) {
        register_step_handler(app);
    }
}

#[cfg(feature = "dev_tools")]
const fn stepper_availability(facts: GameFacts) -> CommandAvailability {
    match (facts.stepper(), facts.phase().battlescape()) {
        (StepperActivity::Stepping, Some(BattleScapePhaseNet::Generation)) => {
            CommandAvailability::Available
        }
        (StepperActivity::Stepping | StepperActivity::NotStepping, _) => NOT_STEPPING,
    }
}

#[cfg(feature = "dev_tools")]
const NOT_STEPPING: CommandAvailability = CommandAvailability::Unavailable {
    code: UnavailableCode::WrongState,
    note: RefusalNote::from_static(
        "procgen.step needs the procgen stepper driving a situation that is still generating",
    ),
};

#[cfg(not(feature = "dev_tools"))]
const fn stepper_availability(_facts: GameFacts) -> CommandAvailability {
    CommandAvailability::Unavailable {
        code: UnavailableCode::NotBuilt,
        note: RefusalNote::from_static("this build has no procgen stepper"),
    }
}

#[cfg(feature = "dev_tools")]
fn register_step_handler(app: &mut App) {
    app.add_systems(Update, handle_procgen_step.after(QaCommandSystems::Claim));
}

#[cfg(not(feature = "dev_tools"))]
const fn register_step_handler(_app: &mut App) {}

#[cfg(feature = "dev_tools")]
fn handle_procgen_step(
    driver: Option<Res<StagedProcgen>>,
    mut pending: Option<ResMut<PendingStepCommand>>,
    mut queue: ResMut<PendingQueue<CommandCall<ProcgenStep>>>,
) {
    if queue.is_empty() {
        return;
    }
    let stage = driver.map(|driver| ProcgenStageNet::from_stage(driver.stage()));
    for (_args, responder) in take_calls::<ProcgenStep>(&mut queue) {
        match (pending.as_deref_mut(), stage) {
            (Some(queued), Some(stage)) => {
                queued.request(StepCommand::Next);
                responder.answer(&ProcgenStepReply { stage });
            }
            _ => responder.unavailable(
                UnavailableCode::WrongState,
                RefusalNote::from_static("the procgen stepper is not driving a situation"),
            ),
        }
    }
}
