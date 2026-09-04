//! The `editor.families` command itself: one row per family, each sorted by key.

use bevy::prelude::*;
use cobalt_mcp_command::{
    command::McpCommand,
    dispatch::{CommandCall, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote,
};
use cobalt_mcp_transport::PendingQueue;
use serde::{Deserialize, Serialize};

use super::{collect::entries_of, filter::families_to_answer, sort::sorted_by_key};
use crate::mcp::{
    commands::availability::only_while_editing,
    facts::EditorFacts,
    forms::EditorRegistries,
    schedule::EditorMcpSystems,
    wire::{EditorFamilyRowNet, EditorModeNet},
};

const NO_REGISTRIES: RefusalNote = RefusalNote::from_static(
    "editor.families reads the content registries, and the editor only holds every one of them \
     once the authoring scene is live",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorFamiliesArgs {
    #[serde(default)]
    family: Option<EditorModeNet>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::mcp) struct EditorFamiliesReply {
    families: Vec<EditorFamilyRowNet>,
}

pub(in crate::mcp) struct EditorFamilies;

impl McpCommand for EditorFamilies {
    type Args = EditorFamiliesArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorFamiliesReply;

    const NAME: CommandName = CommandName::from_static("editor.families");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Read the keys an author can pick from, family by family, each with the label the \
         editor's own picker shows. A `family` argument narrows the reply to that one family; \
         without it every family answers. `Prefab` names the map canvas rather than a family, \
         so it is refused. Entries come back sorted by key.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_while_editing(*facts, NO_REGISTRIES)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_families
                .after(McpCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
        );
    }
}

// One row per family the filter chose, each sorted by key.
fn rows(
    families: Vec<EditorModeNet>,
    registries: &EditorRegistries<'_>,
) -> Vec<EditorFamilyRowNet> {
    families
        .into_iter()
        .map(|family| {
            EditorFamilyRowNet::new(family, sorted_by_key(entries_of(family, registries)))
        })
        .collect()
}

fn handle_editor_families(
    registries: EditorRegistries,
    mut queue: ResMut<PendingQueue<CommandCall<EditorFamilies>>>,
) {
    if queue.is_empty() {
        return;
    }
    for (args, responder) in take_calls::<EditorFamilies>(&mut queue) {
        match families_to_answer(args.family) {
            Ok(families) => responder.answer(&EditorFamiliesReply {
                families: rows(families, &registries),
            }),
            Err(detail) => responder.bad_arguments(detail),
        }
    }
}
