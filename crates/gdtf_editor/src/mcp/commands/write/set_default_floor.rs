//! `editor.set_default_floor`. Pick the Theme draft's default floor from the slabs it holds.

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
        commands::availability::only_on_the_theme_tab, facts::EditorFacts,
        schedule::EditorMcpSystems, wire::TerrainKeyNet,
    },
    theme_form::{ThemeDraft, slab_floor_candidates},
};

const NO_DRAFT: RefusalNote = RefusalNote::from_static(
    "editor.set_default_floor writes the Theme draft, which the editor only creates on entering \
     Editing",
);

const NOT_THE_THEME_TAB: RefusalNote = RefusalNote::from_static(
    "the default-floor picker belongs to the Theme form, so this write needs the Theme tab open",
);

const DRAFT_GONE: RefusalNote =
    RefusalNote::from_static("the Theme draft or the terrain registry is not in the world");

const NOT_A_TERRAIN_KEY: RefusalNote = RefusalNote::from_static(
    "a terrain key is the hyphenated UUID text editor.families answers, and this one does not \
     parse as that",
);

const NOT_A_FLOOR_CANDIDATE: RefusalNote = RefusalNote::from_static(
    "the default-floor picker offers only the slabs the draft already holds, and the draft would \
     ignore any other key",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorSetDefaultFloorArgs {
    key: TerrainKeyNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::mcp) struct EditorSetDefaultFloorReply {
    default_floor: Option<TerrainKeyNet>,
}

pub(in crate::mcp) struct EditorSetDefaultFloor;

impl McpCommand for EditorSetDefaultFloor {
    type Args = EditorSetDefaultFloorArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorSetDefaultFloorReply;

    const NAME: CommandName = CommandName::from_static("editor.set_default_floor");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Set the Theme draft's default floor, choosing from the same list the form's picker \
         offers: the slabs the draft already holds. Any other key is refused rather than \
         silently dropped. Needs the Theme tab open.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_theme_tab(*facts, NO_DRAFT, NOT_THE_THEME_TAB)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_set_default_floor
                .after(McpCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
        );
    }
}

// The exact list `floor_combo` offers, so no call can author a floor no author could pick.
fn offered_floor(
    draft: &ThemeDraft,
    terrain: &TerrainDefRegistry,
    key: TerrainUuid,
) -> Option<TerrainUuid> {
    slab_floor_candidates(draft, terrain)
        .into_iter()
        .find_map(|(candidate, _)| (candidate == key).then_some(candidate))
}

// The draft holds the floor as a registry key; a client reads it as that key's own text.
fn floor_key(draft: &ThemeDraft) -> Option<TerrainKeyNet> {
    draft
        .default_floor()
        .map(|floor| TerrainKeyNet::new((*floor).to_string()))
}

fn handle_editor_set_default_floor(
    draft: Option<ResMut<ThemeDraft>>,
    terrain: Option<Res<TerrainDefRegistry>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorSetDefaultFloor>>>,
) {
    if queue.is_empty() {
        return;
    }
    let (Some(mut draft), Some(terrain)) = (draft, terrain) else {
        for (_args, responder) in take_calls::<EditorSetDefaultFloor>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, DRAFT_GONE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorSetDefaultFloor>(&mut queue) {
        let Ok(parsed) = Uuid::parse_str(&args.key) else {
            responder.unavailable(UnavailableCode::MissingModel, NOT_A_TERRAIN_KEY);
            continue;
        };
        let Some(key) = offered_floor(&draft, &terrain, TerrainUuid::new(parsed)) else {
            responder.unavailable(UnavailableCode::WrongState, NOT_A_FLOOR_CANDIDATE);
            continue;
        };
        draft.set_default_floor(key);
        responder.answer(&EditorSetDefaultFloorReply {
            default_floor: floor_key(&draft),
        });
    }
}
