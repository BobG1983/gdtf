//! `editor.load_gang` fills the Gang draft from the gang registry.

use bevy::prelude::*;
use cobalt_mcp_host::{
    PendingQueue,
    command::McpCommand,
    dispatch::{CommandCall, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use gdtf_battle_sim::ganger::GangRegistry;
use serde::{Deserialize, Serialize};

use super::super::families::{KeyLookup, load_gang};
use crate::{
    gang_form::GangDraft,
    mcp::{
        commands::availability::only_on_the_tab,
        facts::EditorFacts,
        schedule::EditorMcpSystems,
        wire::{EditorKeyNet, EditorLoadOutcomeNet, EditorModeNet},
    },
};

const NO_DRAFT: RefusalNote = RefusalNote::from_static(
    "editor.load_gang fills the Gang draft, which the editor only creates on entering Editing",
);

const NOT_THE_GANG_TAB: RefusalNote = RefusalNote::from_static(
    "the gang form's load picker is drawn only on the Gang tab, so this load needs that tab open",
);

const REGISTRY_GONE: RefusalNote =
    RefusalNote::from_static("the Gang draft or the gang registry is not in the world");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorLoadGangArgs {
    key: EditorKeyNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::mcp) struct EditorLoadGangReply {
    outcome: EditorLoadOutcomeNet,
}

pub(in crate::mcp) struct EditorLoadGang;

impl McpCommand for EditorLoadGang {
    type Args = EditorLoadGangArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorLoadGangReply;

    const NAME: CommandName = CommandName::from_static("editor.load_gang");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Load a gang roster by key into the Gang draft, through that draft's own `load_gang` \
         method. A key the registry does not hold answers `NoSuchKey` with the keys it does hold, \
         and leaves the draft alone. Needs the Gang tab open.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_tab(*facts, EditorModeNet::Gang, NO_DRAFT, NOT_THE_GANG_TAB)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_load_gang
                .after(McpCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
        );
    }
}

fn handle_editor_load_gang(
    draft: Option<ResMut<GangDraft>>,
    registry: Option<Res<GangRegistry>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorLoadGang>>>,
) {
    if queue.is_empty() {
        return;
    }
    let (Some(mut draft), Some(registry)) = (draft, registry) else {
        for (_args, responder) in take_calls::<EditorLoadGang>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, REGISTRY_GONE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorLoadGang>(&mut queue) {
        let outcome = match load_gang(&mut draft, &registry, &args.key) {
            KeyLookup::Loaded => EditorLoadOutcomeNet::Loaded { key: args.key },
            KeyLookup::NoSuchKey(known) => EditorLoadOutcomeNet::NoSuchKey {
                key: args.key,
                known,
            },
        };
        responder.answer(&EditorLoadGangReply { outcome });
    }
}
