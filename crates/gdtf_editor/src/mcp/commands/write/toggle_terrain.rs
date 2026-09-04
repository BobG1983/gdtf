//! `editor.toggle_terrain`. Tick a terrain in the theme draft's terrain library.

use bevy::{asset::uuid::Uuid, prelude::*};
use cobalt_mcp_command::{
    command::McpCommand,
    dispatch::{CommandCall, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_battle_sim::terrain::def::{TerrainDefRegistry, TerrainUuid};
use serde::{Deserialize, Serialize};

use crate::{
    mcp::{
        commands::availability::only_on_the_theme_tab,
        facts::EditorFacts,
        schedule::EditorMcpSystems,
        wire::{TerrainKeyNet, TerrainToggleNet},
    },
    theme_form::ThemeDraft,
};

const NO_DRAFT: RefusalNote = RefusalNote::from_static(
    "editor.toggle_terrain writes the Theme draft, which the editor only creates on entering \
     Editing",
);

const NOT_THE_THEME_TAB: RefusalNote = RefusalNote::from_static(
    "the terrain library belongs to the Theme form, so this write needs the Theme tab open",
);

const DRAFT_GONE: RefusalNote =
    RefusalNote::from_static("the Theme draft or the terrain registry is not in the world");

const NOT_A_TERRAIN_KEY: RefusalNote = RefusalNote::from_static(
    "a terrain key is the hyphenated UUID text editor.families answers, and this one does not \
     parse as that",
);

const NO_SUCH_TERRAIN: RefusalNote = RefusalNote::from_static(
    "the terrain library only offers keys the terrain registry holds, and it holds none under \
     that key",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorToggleTerrainArgs {
    key: TerrainKeyNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::mcp) struct EditorToggleTerrainReply {
    toggle:        TerrainToggleNet,
    default_floor: Option<TerrainKeyNet>,
}

pub(in crate::mcp) struct EditorToggleTerrain;

impl McpCommand for EditorToggleTerrain {
    type Args = EditorToggleTerrainArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorToggleTerrainReply;

    const NAME: CommandName = CommandName::from_static("editor.toggle_terrain");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Add or remove one terrain in the Theme draft's terrain list, the way ticking its row in \
         the terrain library does. Removing the draft's default floor clears that floor, and the \
         reply says so. Needs the Theme tab open.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_theme_tab(*facts, NO_DRAFT, NOT_THE_THEME_TAB)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_toggle_terrain
                .after(McpCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
        );
    }
}

// The draft holds the floor as a registry key; a client reads it as that key's own text.
fn floor_key(draft: &ThemeDraft) -> Option<TerrainKeyNet> {
    draft
        .default_floor()
        .map(|floor| TerrainKeyNet::new((*floor).to_string()))
}

// Which way the tick will go, read before the draft is written.
fn direction(draft: &ThemeDraft, key: TerrainUuid) -> TerrainToggleNet {
    if draft.has_terrain(key) {
        TerrainToggleNet::Removed
    } else {
        TerrainToggleNet::Added
    }
}

fn handle_editor_toggle_terrain(
    draft: Option<ResMut<ThemeDraft>>,
    terrain: Option<Res<TerrainDefRegistry>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorToggleTerrain>>>,
) {
    if queue.is_empty() {
        return;
    }
    let (Some(mut draft), Some(terrain)) = (draft, terrain) else {
        for (_args, responder) in take_calls::<EditorToggleTerrain>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, DRAFT_GONE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorToggleTerrain>(&mut queue) {
        let Ok(parsed) = Uuid::parse_str(&args.key) else {
            responder.unavailable(UnavailableCode::MissingModel, NOT_A_TERRAIN_KEY);
            continue;
        };
        let key = TerrainUuid::new(parsed);
        if terrain.def(&key).is_none() {
            responder.unavailable(UnavailableCode::MissingModel, NO_SUCH_TERRAIN);
            continue;
        }
        let toggle = direction(&draft, key);
        draft.toggle_terrain(key);
        responder.answer(&EditorToggleTerrainReply {
            toggle,
            default_floor: floor_key(&draft),
        });
    }
}
