//! `editor.map`: every painted slot on one storey, exactly as `EditorMap` holds them.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::metric::Level;
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
    editor_map::EditorMap,
    net_qa::{
        commands::availability::only_on_the_prefab_tab,
        facts::EditorFacts,
        schedule::EditorNetQaSystems,
        wire::{EditorGridSizeNet, EditorLevelNet, PaintedMapNet, PaintedRowNet},
    },
    session::MapEditorSession,
};

const NO_CANVAS: RefusalNote = RefusalNote::from_static(
    "editor.map reads the painted map and the authoring session, which the editor only creates \
     on entering Editing",
);

const NOT_THE_PREFAB_TAB: RefusalNote = RefusalNote::from_static(
    "the painted map is the Prefab canvas's own model, so this read needs the Prefab tab open",
);

const CANVAS_GONE: RefusalNote =
    RefusalNote::from_static("the painted map or the authoring session is not in the world");

// The two resources the Prefab canvas paints into and reads its extent from.
#[derive(SystemParam)]
struct PaintedMapSources<'w> {
    map:     Option<Res<'w, EditorMap>>,
    session: Option<Res<'w, MapEditorSession>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorMapArgs {
    level: EditorLevelNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::net_qa) struct EditorMapReply {
    grid_size: EditorGridSizeNet,
    painted:   PaintedMapNet,
}

pub(in crate::net_qa) struct EditorPaintedMap;

impl QaCommand for EditorPaintedMap {
    type Args = EditorMapArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorMapReply;

    const NAME: CommandName = CommandName::from_static("editor.map");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Read every painted cell on one storey of the prefab canvas, each with its tile and \
         facing, plus the grid extent the session holds. Nothing is filtered out: a shrink \
         leaves slots outside the grid painted, and a save reports those as illegal cells. \
         Needs the Prefab tab open.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_prefab_tab(*facts, NO_CANVAS, NOT_THE_PREFAB_TAB)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_map
                .after(QaCommandSystems::Claim)
                .in_set(EditorNetQaSystems::Gather),
        );
    }
}

// Every slot on that storey, ordered by cell so two reads of one map answer alike.
fn rows_on(map: &EditorMap, level: Level) -> PaintedMapNet {
    let mut slots: Vec<_> = map
        .painted()
        .filter(|(slot, _)| slot.level() == level)
        .map(|(slot, piece)| (*slot, *piece))
        .collect();
    slots.sort_by_key(|(slot, _)| (slot.x, slot.y));
    PaintedMapNet::new(
        slots
            .into_iter()
            .map(|(slot, piece)| PaintedRowNet::from_piece(slot, piece))
            .collect(),
    )
}

fn handle_editor_map(
    sources: PaintedMapSources,
    mut queue: ResMut<PendingQueue<CommandCall<EditorPaintedMap>>>,
) {
    if queue.is_empty() {
        return;
    }
    let (Some(map), Some(session)) = (sources.map.as_deref(), sources.session.as_deref()) else {
        for (_args, responder) in take_calls::<EditorPaintedMap>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, CANVAS_GONE);
        }
        return;
    };
    let grid_size = EditorGridSizeNet::from_size(session.grid_size());
    for (args, responder) in take_calls::<EditorPaintedMap>(&mut queue) {
        responder.answer(&EditorMapReply {
            grid_size,
            painted: rows_on(map, Level::new(*args.level)),
        });
    }
}
