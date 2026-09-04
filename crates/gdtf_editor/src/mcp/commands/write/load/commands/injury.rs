//! `editor.load_injury` fills the Injury draft from the injury registry.

use bevy::prelude::*;
use cobalt_mcp_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_battle_sim::injuries::InjuryRegistry;
use serde::{Deserialize, Serialize};

use super::super::families::{KeyLookup, load_injury};
use crate::{
    injury_form::InjuryDraft,
    mcp::{
        commands::availability::only_on_the_tab,
        facts::EditorFacts,
        schedule::EditorMcpSystems,
        wire::{EditorKeyNet, EditorLoadOutcomeNet, EditorModeNet},
    },
};

const NO_DRAFT: RefusalNote = RefusalNote::from_static(
    "editor.load_injury fills the Injury draft, which the editor only creates on entering Editing",
);

const NOT_THE_INJURY_TAB: RefusalNote = RefusalNote::from_static(
    "the injury form's load picker is drawn only on the Injury tab, so this load needs that tab \
     open",
);

const REGISTRY_GONE: RefusalNote =
    RefusalNote::from_static("the Injury draft or the injury registry is not in the world");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorLoadInjuryArgs {
    key: EditorKeyNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::mcp) struct EditorLoadInjuryReply {
    outcome: EditorLoadOutcomeNet,
}

pub(in crate::mcp) struct EditorLoadInjury;

impl QaCommand for EditorLoadInjury {
    type Args = EditorLoadInjuryArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorLoadInjuryReply;

    const NAME: CommandName = CommandName::from_static("editor.load_injury");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Load an injury by key into the Injury draft, through that draft's own `load_injury` \
         method. A key the registry does not hold answers `NoSuchKey` with the keys it does hold, \
         and leaves the draft alone. Needs the Injury tab open.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_tab(*facts, EditorModeNet::Injury, NO_DRAFT, NOT_THE_INJURY_TAB)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_load_injury
                .after(QaCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
        );
    }
}

fn handle_editor_load_injury(
    draft: Option<ResMut<InjuryDraft>>,
    registry: Option<Res<InjuryRegistry>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorLoadInjury>>>,
) {
    if queue.is_empty() {
        return;
    }
    let (Some(mut draft), Some(registry)) = (draft, registry) else {
        for (_args, responder) in take_calls::<EditorLoadInjury>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, REGISTRY_GONE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorLoadInjury>(&mut queue) {
        let outcome = match load_injury(&mut draft, &registry, &args.key) {
            KeyLookup::Loaded => EditorLoadOutcomeNet::Loaded { key: args.key },
            KeyLookup::NoSuchKey(known) => EditorLoadOutcomeNet::NoSuchKey {
                key: args.key,
                known,
            },
        };
        responder.answer(&EditorLoadInjuryReply { outcome });
    }
}
