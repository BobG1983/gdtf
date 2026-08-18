//! `editor.set_field` — write one single-value field of the active mode's draft.

use bevy::prelude::*;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use serde::{Deserialize, Serialize};

use super::availability::only_on_the_terrain_tab;
use crate::net_qa::{
    facts::EditorFacts,
    forms::EditorForms,
    schedule::EditorNetQaSystems,
    wire::{EditorFieldNet, EditorModeNet, TerrainKindNet},
};

const NO_DRAFTS: RefusalNote = RefusalNote::from_static(
    "editor.set_field writes a form's draft, and every draft is a resource the editor only \
     creates on entering Editing",
);

const NOT_THE_TERRAIN_TAB: RefusalNote = RefusalNote::from_static(
    "every field editor.set_field offers belongs to the Terrain draft, so it needs the Terrain \
     tab open",
);

const DRAFT_GONE: RefusalNote =
    RefusalNote::from_static("the Terrain draft resource is not in the world");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorSetFieldArgs {
    field: EditorFieldNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::net_qa) struct EditorSetFieldReply {
    mode:  EditorModeNet,
    field: EditorFieldNet,
}

pub(in crate::net_qa) struct EditorSetField;

impl QaCommand for EditorSetField {
    type Args = EditorSetFieldArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorSetFieldReply;

    const NAME: CommandName = CommandName::from_static("editor.set_field");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Write one single-value field of the active mode's draft, the way the form's own widget \
         writes it. Today that is the Terrain draft's kind, so it needs the Terrain tab open.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_terrain_tab(*facts, NO_DRAFTS, NOT_THE_TERRAIN_TAB)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_set_field
                .after(QaCommandSystems::Claim)
                .in_set(EditorNetQaSystems::Gather),
        );
    }
}

fn handle_editor_set_field(
    mut forms: EditorForms,
    mut queue: ResMut<PendingQueue<CommandCall<EditorSetField>>>,
) {
    if queue.is_empty() {
        return;
    }
    let Some(terrain) = forms.terrain.as_mut() else {
        for (_args, responder) in take_calls::<EditorSetField>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, DRAFT_GONE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorSetField>(&mut queue) {
        let EditorFieldNet::Kind(kind) = args.field;
        terrain.set_kind(kind.to_choice());
        responder.answer(&EditorSetFieldReply {
            mode:  EditorModeNet::Terrain,
            field: EditorFieldNet::Kind(TerrainKindNet::from_choice(terrain.kind())),
        });
    }
}
