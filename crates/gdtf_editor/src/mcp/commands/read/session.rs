//! `editor.session`: theme, grid, level and view, as the authoring scene holds them.

use bevy::{ecs::system::SystemParam, prelude::*};
use cobalt_mcp_host::{
    PendingQueue,
    command::McpCommand,
    dispatch::{CommandCall, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use gdtf_battle_presenter::{IsolateView, ViewMode};
use gdtf_battle_sim::{level::ThemeUuid, terrain::def::TerrainUuid};
use serde::{Deserialize, Serialize};

use crate::{
    canvas::{CanvasZoom, CurrentEditLevel},
    mcp::{
        commands::availability::only_while_editing,
        facts::EditorFacts,
        schedule::EditorMcpSystems,
        wire::{
            EditorGridSizeNet, EditorIsolateViewNet, EditorKeyNet, EditorLevelNet, EditorPanNet,
            EditorViewModeNet, EditorViewNet, EditorZoomNet, TerrainFacingNet,
        },
    },
    preview::view::PreviewPan,
    session::MapEditorSession,
};

const NO_SESSION: RefusalNote = RefusalNote::from_static(
    "every resource editor.session reads is scoped to Editing, so there is no session to read \
     while the editor is still loading its registries",
);

const SESSION_GONE: RefusalNote = RefusalNote::from_static(
    "a resource the authoring session is made of left the world between the availability check \
     and the handler",
);

// The six resources the authoring scene holds for the canvas the author is looking at.
#[derive(SystemParam)]
struct EditorSessionSources<'w> {
    session: Option<Res<'w, MapEditorSession>>,
    level:   Option<Res<'w, CurrentEditLevel>>,
    view:    Option<Res<'w, ViewMode>>,
    isolate: Option<Res<'w, IsolateView>>,
    zoom:    Option<Res<'w, CanvasZoom>>,
    pan:     Option<Res<'w, PreviewPan>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorSessionArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::mcp) struct EditorSessionReply {
    theme:         EditorKeyNet,
    default_floor: Option<EditorKeyNet>,
    grid_size:     EditorGridSizeNet,
    selected_tile: Option<EditorKeyNet>,
    facing:        TerrainFacingNet,
    level:         EditorLevelNet,
    view:          EditorViewNet,
}

pub(in crate::mcp) struct EditorSession;

impl McpCommand for EditorSession {
    type Args = EditorSessionArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorSessionReply;

    const NAME: CommandName = CommandName::from_static("editor.session");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Read the authoring session the canvas draws from: the theme and its default floor, \
         the grid extent, the selected paint tile, the facing a paint turns it to, the storey \
         being edited, and the view. The view is the draw mode, the onion isolation, the zoom \
         and the pan. Needs the authoring scene.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_while_editing(*facts, NO_SESSION)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_session
                .after(McpCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
        );
    }
}

// Registry UUIDs go on the wire as their own hyphenated text, the key a load takes.
fn theme_key(theme: ThemeUuid) -> EditorKeyNet {
    EditorKeyNet::new((*theme).to_string())
}

fn terrain_key(terrain: TerrainUuid) -> EditorKeyNet {
    EditorKeyNet::new((*terrain).to_string())
}

fn reply_from(
    session: &MapEditorSession,
    level: CurrentEditLevel,
    view: EditorViewNet,
) -> EditorSessionReply {
    EditorSessionReply {
        theme: theme_key(session.theme()),
        default_floor: session.default_floor().map(terrain_key),
        grid_size: EditorGridSizeNet::from_size(session.grid_size()),
        selected_tile: session.selected_tile().map(terrain_key),
        facing: TerrainFacingNet::from_facing(session.facing()),
        level: EditorLevelNet::new(*level.level()),
        view,
    }
}

impl EditorSessionSources<'_> {
    // Absent together: every one is scoped to the authoring scene, so all six or none.
    fn read(&self) -> Option<EditorSessionReply> {
        let (Some(session), Some(level), Some(view), Some(isolate), Some(zoom), Some(pan)) = (
            self.session.as_deref(),
            self.level.as_deref(),
            self.view.as_deref(),
            self.isolate.as_deref(),
            self.zoom.as_deref(),
            self.pan.as_deref(),
        ) else {
            return None;
        };
        Some(reply_from(
            session,
            *level,
            EditorViewNet::new(
                EditorViewModeNet::from_mode(*view),
                EditorIsolateViewNet::from_isolate(*isolate),
                EditorZoomNet::new(**zoom),
                EditorPanNet::from_pan(*pan),
            ),
        ))
    }
}

fn handle_editor_session(
    sources: EditorSessionSources,
    mut queue: ResMut<PendingQueue<CommandCall<EditorSession>>>,
) {
    if queue.is_empty() {
        return;
    }
    let Some(reply) = sources.read() else {
        for (_args, responder) in take_calls::<EditorSession>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, SESSION_GONE);
        }
        return;
    };
    for (_args, responder) in take_calls::<EditorSession>(&mut queue) {
        responder.answer(&reply);
    }
}
