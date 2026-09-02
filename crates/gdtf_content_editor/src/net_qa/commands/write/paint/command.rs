//! Lay the selected tile in a cell, the way a click on the canvas does.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::{
    metric::{Cell, CellLevel},
    terrain::def::TerrainDefRegistry,
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
    canvas::CurrentEditLevel,
    connector_pairing::apply_placement_with_pairing,
    editor_map::EditorMap,
    net_qa::{
        commands::availability::only_on_the_prefab_tab,
        facts::EditorFacts,
        schedule::EditorNetQaSystems,
        wire::{
            EditorCellXNet, EditorCellYNet, PaintRefusalNet, PairingOutcomeNet, PlacementVerdictNet,
        },
    },
    placement::{ProposedPlacement, evaluate_placement},
    session::MapEditorSession,
};

const NO_CANVAS: RefusalNote = RefusalNote::from_static(
    "editor.paint writes the painted map, which the editor only creates on entering Editing",
);

const NOT_THE_PREFAB_TAB: RefusalNote = RefusalNote::from_static(
    "the canvas belongs to the Prefab form, so this write needs the Prefab tab open",
);

const CANVAS_GONE: RefusalNote = RefusalNote::from_static(
    "the painted map, the authoring session, the edit storey or the terrain registry is not in \
     the world",
);

// The map a canvas click writes, and the three resources it reads the placement from.
#[derive(SystemParam)]
struct PaintSources<'w> {
    map:        Option<ResMut<'w, EditorMap>>,
    session:    Option<Res<'w, MapEditorSession>>,
    edit_level: Option<Res<'w, CurrentEditLevel>>,
    terrain:    Option<Res<'w, TerrainDefRegistry>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorPaintArgs {
    x: EditorCellXNet,
    y: EditorCellYNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::net_qa) struct EditorPaintReply {
    pub(super) refusal: Option<PaintRefusalNet>,
    pub(super) verdict: Option<PlacementVerdictNet>,
    pub(super) pairing: Option<PairingOutcomeNet>,
}

pub(in crate::net_qa) struct EditorPaint;

impl QaCommand for EditorPaint {
    type Args = EditorPaintArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorPaintReply;

    const NAME: CommandName = CommandName::from_static("editor.paint");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Paint the selected tile in one cell of the storey being edited, through the same \
         placement and connector pairing a click on the canvas runs. The reply carries the \
         rules' verdict for the slot, read before the write, and what the pairing pass did, \
         including the second tile it may place one storey up. An illegal slot names its \
         reason and writes nothing. Needs the Prefab tab open.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_prefab_tab(*facts, NO_CANVAS, NOT_THE_PREFAB_TAB)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_paint
                .after(QaCommandSystems::Claim)
                .in_set(EditorNetQaSystems::Gather),
        );
    }
}

// The verdict is read before the write.
pub(super) fn paint_one(
    map: &mut EditorMap,
    session: &MapEditorSession,
    terrain: &TerrainDefRegistry,
    placement: &ProposedPlacement,
) -> EditorPaintReply {
    let size = session.grid_size();
    let verdict = evaluate_placement(map, terrain, session.theme(), placement, size);
    let outcome = apply_placement_with_pairing(map, terrain, session.theme(), placement, size);
    EditorPaintReply {
        refusal: None,
        verdict: Some(PlacementVerdictNet::from_verdict(&verdict)),
        pairing: Some(PairingOutcomeNet::from_outcome(outcome)),
    }
}

// The storey, the tile and the facing come from where `paint_at_uv` reads them.
fn proposal(
    session: &MapEditorSession,
    edit_level: CurrentEditLevel,
    args: &EditorPaintArgs,
) -> Option<ProposedPlacement> {
    let slot = CellLevel::new(Cell::new(*args.x, *args.y), edit_level.level());
    session.paint_proposal(slot)
}

fn handle_editor_paint(
    mut sources: PaintSources,
    mut queue: ResMut<PendingQueue<CommandCall<EditorPaint>>>,
) {
    if queue.is_empty() {
        return;
    }
    let (Some(map), Some(session), Some(edit_level), Some(terrain)) = (
        sources.map.as_mut(),
        sources.session.as_deref(),
        sources.edit_level.as_deref(),
        sources.terrain.as_deref(),
    ) else {
        for (_args, responder) in take_calls::<EditorPaint>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, CANVAS_GONE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorPaint>(&mut queue) {
        let reply = match proposal(session, *edit_level, &args) {
            Some(placement) => paint_one(map, session, terrain, &placement),
            None => EditorPaintReply {
                refusal: Some(PaintRefusalNet::NoSelectedTile),
                verdict: None,
                pairing: None,
            },
        };
        responder.answer(&reply);
    }
}
