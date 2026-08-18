//! `editor.list_op` — edit one list-valued field of the active mode's draft.

use bevy::prelude::*;
use gdtf_battle_sim::terrain::facing::TerrainFacing;
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
use crate::{
    net_qa::{
        facts::EditorFacts,
        forms::EditorForms,
        schedule::EditorNetQaSystems,
        wire::{
            EditorListMemberNet, EditorListNet, EditorListOpNet, EditorModeNet, TerrainFacingNet,
        },
    },
    terrain_form::{TerrainDraft, TerrainKindChoice},
};

const NO_DRAFTS: RefusalNote = RefusalNote::from_static(
    "editor.list_op edits a form's draft, and every draft is a resource the editor only creates \
     on entering Editing",
);

const NOT_THE_TERRAIN_TAB: RefusalNote = RefusalNote::from_static(
    "every list editor.list_op offers belongs to the Terrain draft, so it needs the Terrain tab \
     open",
);

const DRAFT_GONE: RefusalNote =
    RefusalNote::from_static("the Terrain draft resource is not in the world");

const NOT_AN_EMPLACEMENT: RefusalNote = RefusalNote::from_static(
    "the Terrain draft commits entry sides only while its kind is Emplacement, so this write \
     would silently do nothing",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorListOpArgs {
    list: EditorListNet,
    op:   EditorListOpNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::net_qa) struct EditorListOpReply {
    mode:    EditorModeNet,
    list:    EditorListNet,
    members: Vec<EditorListMemberNet>,
}

pub(in crate::net_qa) struct EditorListOp;

impl QaCommand for EditorListOp {
    type Args = EditorListOpArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorListOpReply;

    const NAME: CommandName = CommandName::from_static("editor.list_op");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Edit one list-valued field of the active mode's draft through that form's own setter. \
         Today that is the Terrain draft's emplacement entry sides, so it needs the Terrain tab.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_terrain_tab(*facts, NO_DRAFTS, NOT_THE_TERRAIN_TAB)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_list_op
                .after(QaCommandSystems::Claim)
                .in_set(EditorNetQaSystems::Gather),
        );
    }
}

// The sides the draft holds after `side` is added if absent, removed if present.
fn toggled(draft: &TerrainDraft, side: TerrainFacingNet) -> Vec<TerrainFacing> {
    let wanted = side.to_facing();
    let mut sides = draft.entry_sides().to_vec();
    if let Some(at) = sides.iter().position(|held| *held == wanted) {
        sides.remove(at);
    } else {
        sides.push(wanted);
    }
    sides
}

fn members_of(draft: &TerrainDraft) -> Vec<EditorListMemberNet> {
    draft
        .entry_sides()
        .iter()
        .map(|side| EditorListMemberNet::from_facing(TerrainFacingNet::from_facing(*side)))
        .collect()
}

fn handle_editor_list_op(
    mut forms: EditorForms,
    mut queue: ResMut<PendingQueue<CommandCall<EditorListOp>>>,
) {
    if queue.is_empty() {
        return;
    }
    let Some(terrain) = forms.terrain.as_mut() else {
        for (_args, responder) in take_calls::<EditorListOp>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, DRAFT_GONE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorListOp>(&mut queue) {
        let EditorListNet::EntrySides = args.list;
        if terrain.kind() != TerrainKindChoice::Emplacement {
            responder.unavailable(UnavailableCode::WrongState, NOT_AN_EMPLACEMENT);
            continue;
        }
        let EditorListOpNet::Toggle(side) = args.op;
        let sides = toggled(terrain, side);
        terrain.set_entry_sides(sides);
        responder.answer(&EditorListOpReply {
            mode:    EditorModeNet::Terrain,
            list:    args.list,
            members: members_of(terrain),
        });
    }
}
