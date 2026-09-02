//! `editor.load_terrain` fills the Terrain draft from the terrain def registry.

use bevy::prelude::*;
use gdtf_battle_sim::terrain::def::TerrainDefRegistry;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use serde::{Deserialize, Serialize};

use super::super::families::{KeyLookup, load_terrain};
use crate::{
    net_qa::{
        commands::availability::only_on_the_tab,
        facts::EditorFacts,
        schedule::EditorNetQaSystems,
        wire::{EditorKeyNet, EditorLoadOutcomeNet, EditorModeNet},
    },
    terrain_form::TerrainDraft,
};

const NO_DRAFT: RefusalNote = RefusalNote::from_static(
    "editor.load_terrain fills the Terrain draft, which the editor only creates on entering \
     Editing",
);

const NOT_THE_TERRAIN_TAB: RefusalNote = RefusalNote::from_static(
    "the terrain form's load picker is drawn only on the Terrain tab, so this load needs that tab \
     open",
);

const REGISTRY_GONE: RefusalNote =
    RefusalNote::from_static("the Terrain draft or the terrain def registry is not in the world");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorLoadTerrainArgs {
    key: EditorKeyNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::net_qa) struct EditorLoadTerrainReply {
    outcome: EditorLoadOutcomeNet,
}

pub(in crate::net_qa) struct EditorLoadTerrain;

impl QaCommand for EditorLoadTerrain {
    type Args = EditorLoadTerrainArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorLoadTerrainReply;

    const NAME: CommandName = CommandName::from_static("editor.load_terrain");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Load a terrain def by key into the Terrain draft, through that draft's own \
         `load_from_def` method. The key is the hyphenated UUID text editor.families answers. A \
         key the registry does not hold answers `NoSuchKey` with the keys it does hold, and \
         leaves the draft alone. Needs the Terrain tab open.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_tab(
            *facts,
            EditorModeNet::Terrain,
            NO_DRAFT,
            NOT_THE_TERRAIN_TAB,
        )
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_load_terrain
                .after(QaCommandSystems::Claim)
                .in_set(EditorNetQaSystems::Gather),
        );
    }
}

fn handle_editor_load_terrain(
    draft: Option<ResMut<TerrainDraft>>,
    registry: Option<Res<TerrainDefRegistry>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorLoadTerrain>>>,
) {
    if queue.is_empty() {
        return;
    }
    let (Some(mut draft), Some(registry)) = (draft, registry) else {
        for (_args, responder) in take_calls::<EditorLoadTerrain>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, REGISTRY_GONE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorLoadTerrain>(&mut queue) {
        let outcome = match load_terrain(&mut draft, &registry, &args.key) {
            KeyLookup::Loaded => EditorLoadOutcomeNet::Loaded { key: args.key },
            KeyLookup::NoSuchKey(known) => EditorLoadOutcomeNet::NoSuchKey {
                key: args.key,
                known,
            },
        };
        responder.answer(&EditorLoadTerrainReply { outcome });
    }
}
