//! `editor.load_field` fills the Field draft from the field def registry.

use bevy::prelude::*;
use cobalt_mcp_command::{
    command::McpCommand,
    dispatch::{CommandCall, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_battle_sim::effects::fields::FieldDefRegistry;
use serde::{Deserialize, Serialize};

use super::super::families::{KeyLookup, load_field};
use crate::{
    field_form::FieldDraft,
    mcp::{
        commands::availability::only_on_the_tab,
        facts::EditorFacts,
        schedule::EditorMcpSystems,
        wire::{EditorKeyNet, EditorLoadOutcomeNet, EditorModeNet},
    },
};

const NO_DRAFT: RefusalNote = RefusalNote::from_static(
    "editor.load_field fills the Field draft, which the editor only creates on entering Editing",
);

const NOT_THE_FIELD_TAB: RefusalNote = RefusalNote::from_static(
    "the field form's load picker is drawn only on the Field tab, so this load needs that tab open",
);

const REGISTRY_GONE: RefusalNote =
    RefusalNote::from_static("the Field draft or the field registry is not in the world");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorLoadFieldArgs {
    key: EditorKeyNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::mcp) struct EditorLoadFieldReply {
    outcome: EditorLoadOutcomeNet,
}

pub(in crate::mcp) struct EditorLoadField;

impl McpCommand for EditorLoadField {
    type Args = EditorLoadFieldArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorLoadFieldReply;

    const NAME: CommandName = CommandName::from_static("editor.load_field");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Load a field def by key into the Field draft, through that draft's own `load_field` \
         method. A key the registry does not hold answers `NoSuchKey` with the keys it does hold, \
         and leaves the draft alone. Needs the Field tab open.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_tab(*facts, EditorModeNet::Field, NO_DRAFT, NOT_THE_FIELD_TAB)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_load_field
                .after(McpCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
        );
    }
}

fn handle_editor_load_field(
    draft: Option<ResMut<FieldDraft>>,
    registry: Option<Res<FieldDefRegistry>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorLoadField>>>,
) {
    if queue.is_empty() {
        return;
    }
    let (Some(mut draft), Some(registry)) = (draft, registry) else {
        for (_args, responder) in take_calls::<EditorLoadField>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, REGISTRY_GONE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorLoadField>(&mut queue) {
        let outcome = match load_field(&mut draft, &registry, &args.key) {
            KeyLookup::Loaded => EditorLoadOutcomeNet::Loaded { key: args.key },
            KeyLookup::NoSuchKey(known) => EditorLoadOutcomeNet::NoSuchKey {
                key: args.key,
                known,
            },
        };
        responder.answer(&EditorLoadFieldReply { outcome });
    }
}
