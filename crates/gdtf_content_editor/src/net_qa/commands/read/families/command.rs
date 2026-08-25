//! The `editor.families` command itself: one row per family, each sorted by key.

use bevy::prelude::*;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote,
};
use serde::{Deserialize, Serialize};

use super::{collect::entries_of, sort::sorted_by_key};
use crate::net_qa::{
    commands::availability::only_while_editing,
    facts::EditorFacts,
    forms::EditorRegistries,
    schedule::EditorNetQaSystems,
    wire::{EditorFamilyNet, EditorFamilyRowNet},
};

const NO_REGISTRIES: RefusalNote = RefusalNote::from_static(
    "editor.families reads the content registries, and the editor only holds every one of them \
     once the authoring scene is live",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorFamiliesArgs {
    #[serde(default)]
    family: Option<EditorFamilyNet>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::net_qa) struct EditorFamiliesReply {
    families: Vec<EditorFamilyRowNet>,
}

pub(in crate::net_qa) struct EditorFamilies;

impl QaCommand for EditorFamilies {
    type Args = EditorFamiliesArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorFamiliesReply;

    const NAME: CommandName = CommandName::from_static("editor.families");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Read the keys an author can pick from, family by family, each with the label the \
         editor's own picker shows. A `family` argument narrows the reply to that one family; \
         without it every family answers. Entries come back sorted by key.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_while_editing(*facts, NO_REGISTRIES)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_families
                .after(QaCommandSystems::Claim)
                .in_set(EditorNetQaSystems::Gather),
        );
    }
}

// A filter narrows the reply to one family; without one, every family answers in `ALL` order.
fn rows(
    only: Option<EditorFamilyNet>,
    registries: &EditorRegistries<'_>,
) -> Vec<EditorFamilyRowNet> {
    EditorFamilyNet::ALL
        .into_iter()
        .filter(|family| only.is_none_or(|wanted| wanted == *family))
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
        responder.answer(&EditorFamiliesReply {
            families: rows(args.family, &registries),
        });
    }
}
