//! `editor.select_tile`. Pick the paint tile from the same rows the palette draws.

use bevy::{asset::uuid::Uuid, prelude::*};
use gdtf_battle_sim::{
    level::UuidThemeRegistry,
    terrain::def::{TerrainDefRegistry, TerrainUuid},
};
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use serde::{Deserialize, Serialize};

use crate::{
    net_qa::{
        commands::availability::only_on_the_prefab_tab,
        facts::EditorFacts,
        schedule::EditorNetQaSystems,
        wire::{SelectTileRefusalNet, TerrainKeyNet},
    },
    session::MapEditorSession,
};

const NO_CANVAS: RefusalNote = RefusalNote::from_static(
    "editor.select_tile writes the authoring session, which the editor only creates on entering \
     Editing",
);

const NOT_THE_PREFAB_TAB: RefusalNote = RefusalNote::from_static(
    "the palette belongs to the Prefab form, so this write needs the Prefab tab open",
);

const PALETTE_GONE: RefusalNote = RefusalNote::from_static(
    "the authoring session, the theme registry or the terrain registry is not in the world",
);

const NOT_A_TERRAIN_KEY: RefusalNote = RefusalNote::from_static(
    "a terrain key is the hyphenated UUID text editor.families answers, and this one does not \
     parse as that",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorSelectTileArgs {
    key: TerrainKeyNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::net_qa) struct EditorSelectTileReply {
    refusal:       Option<SelectTileRefusalNet>,
    selected_tile: Option<TerrainKeyNet>,
}

pub(in crate::net_qa) struct EditorSelectTile;

impl QaCommand for EditorSelectTile {
    type Args = EditorSelectTileArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorSelectTileReply;

    const NAME: CommandName = CommandName::from_static("editor.select_tile");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Select the tile the canvas paints, choosing from the same rows the palette draws: the \
         terrain the session's theme lists that the terrain registry also holds a def for. A key \
         that fails either condition is a typed refusal naming which one, and the selected tile \
         stands. Needs the Prefab tab open.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_prefab_tab(*facts, NO_CANVAS, NOT_THE_PREFAB_TAB)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_select_tile
                .after(QaCommandSystems::Claim)
                .in_set(EditorNetQaSystems::Gather),
        );
    }
}

// The session holds the tile as a registry key; a client reads it as that key's own text.
fn selected_key(session: &MapEditorSession) -> Option<TerrainKeyNet> {
    session
        .selected_tile()
        .map(|tile| TerrainKeyNet::new((*tile).to_string()))
}

// The two conditions `palette_panel` applies before it draws a row for a key.
fn offered_tile(
    session: &MapEditorSession,
    themes: &UuidThemeRegistry,
    terrain: &TerrainDefRegistry,
    key: TerrainUuid,
) -> Result<TerrainUuid, SelectTileRefusalNet> {
    if terrain.def(&key).is_none() {
        return Err(SelectTileRefusalNet::NoTerrainDef);
    }
    let listed = themes
        .terrain(&session.theme())
        .is_some_and(|keys| keys.contains(&key));
    if listed {
        Ok(key)
    } else {
        Err(SelectTileRefusalNet::NotInTheThemePalette)
    }
}

fn handle_editor_select_tile(
    session: Option<ResMut<MapEditorSession>>,
    themes: Option<Res<UuidThemeRegistry>>,
    terrain: Option<Res<TerrainDefRegistry>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorSelectTile>>>,
) {
    if queue.is_empty() {
        return;
    }
    let (Some(mut session), Some(themes), Some(terrain)) = (session, themes, terrain) else {
        for (_args, responder) in take_calls::<EditorSelectTile>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, PALETTE_GONE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorSelectTile>(&mut queue) {
        let Ok(parsed) = Uuid::parse_str(&args.key) else {
            responder.unavailable(UnavailableCode::MissingModel, NOT_A_TERRAIN_KEY);
            continue;
        };
        let refusal = match offered_tile(&session, &themes, &terrain, TerrainUuid::new(parsed)) {
            Ok(key) => {
                session.select_tile(key);
                None
            }
            Err(refusal) => Some(refusal),
        };
        responder.answer(&EditorSelectTileReply {
            refusal,
            selected_tile: selected_key(&session),
        });
    }
}
