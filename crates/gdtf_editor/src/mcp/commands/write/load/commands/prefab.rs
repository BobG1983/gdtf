//! `editor.load_prefab` opens an authored prefab onto the prefab canvas.

use bevy::{asset::uuid::Uuid, prelude::*};
use cobalt_mcp_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_battle_sim::level::{PrefabKey, PrefabRegistry, PrefabSpec, ThemeUuid, UuidThemeRegistry};
use serde::{Deserialize, Serialize};

use crate::{
    canvas::CurrentEditLevel,
    editor_map::EditorMap,
    mcp::{
        commands::availability::only_on_the_tab,
        facts::EditorFacts,
        schedule::EditorMcpSystems,
        wire::{
            EditorContentNameNet, EditorGridSizeNet, EditorLoadPrefabOutcomeNet, EditorModeNet,
            PrefabPlacementCountNet, SpawnRoleNet, ThemeKeyNet,
        },
    },
    open::open_prefab,
    session::MapEditorSession,
};

const NO_CANVAS: RefusalNote = RefusalNote::from_static(
    "editor.load_prefab writes the authoring session, the painted map and the edit storey, which \
     the editor only creates on entering Editing",
);

const NOT_THE_PREFAB_TAB: RefusalNote = RefusalNote::from_static(
    "the prefab canvas and its open picker are drawn only on the Prefab tab, so this load needs \
     that tab open",
);

const CANVAS_GONE: RefusalNote = RefusalNote::from_static(
    "the authoring session, the painted map or the edit storey is not in the world",
);

const PREFABS_GONE: RefusalNote = RefusalNote::from_static(
    "the prefab registry is not in the world yet, so there is no authored prefab to open",
);

const THEMES_GONE: RefusalNote = RefusalNote::from_static(
    "the theme registry is not in the world, so an open would leave the session on a theme \
     carrying no default floor",
);

/// The three models an open writes: the session, the painted map, and the edit storey.
#[derive(bevy::ecs::system::SystemParam)]
pub(in crate::mcp) struct PrefabCanvas<'w> {
    session:    Option<ResMut<'w, MapEditorSession>>,
    map:        Option<ResMut<'w, EditorMap>>,
    edit_level: Option<ResMut<'w, CurrentEditLevel>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorLoadPrefabArgs {
    name:  EditorContentNameNet,
    theme: ThemeKeyNet,
    size:  EditorGridSizeNet,
    role:  SpawnRoleNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::mcp) struct EditorLoadPrefabReply {
    outcome: EditorLoadPrefabOutcomeNet,
}

pub(in crate::mcp) struct EditorLoadPrefab;

impl QaCommand for EditorLoadPrefab {
    type Args = EditorLoadPrefabArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorLoadPrefabReply;

    const NAME: CommandName = CommandName::from_static("editor.load_prefab");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Open an authored prefab onto the prefab canvas, the way the Prefab tab's own picker \
         does: its grid extent and theme onto the session, its cells onto the painted map, and \
         the edit storey clamped into the loaded extent. The arguments are the prefab name and \
         its key: the theme's hyphenated UUID text, the grid size, and the spawn role. A name \
         and key no prefab sits under answers `NoSuchPrefab` and writes nothing. Needs the \
         Prefab tab open.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_tab(*facts, EditorModeNet::Prefab, NO_CANVAS, NOT_THE_PREFAB_TAB)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_load_prefab
                .after(QaCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
        );
    }
}

// The spec the asked-for name and key name, or `None` when no prefab sits under them.
fn asked_for<'a>(
    registry: &'a PrefabRegistry,
    args: &EditorLoadPrefabArgs,
) -> Option<&'a PrefabSpec> {
    let theme = ThemeUuid::new(Uuid::parse_str(&args.theme).ok()?);
    let key = PrefabKey::new(theme, args.size.size()?, args.role.to_role());
    registry
        .prefabs_for(&key)
        .iter()
        .find(|prefab| prefab.name().as_str() == args.name.as_str())
        .map(gdtf_battle_sim::level::Prefab::spec)
}

// The name and key the caller asked for, mirrored back so a miss says which call missed.
fn missed(args: &EditorLoadPrefabArgs) -> EditorLoadPrefabOutcomeNet {
    EditorLoadPrefabOutcomeNet::NoSuchPrefab {
        name:  args.name.clone(),
        theme: args.theme.clone(),
        size:  args.size,
        role:  args.role,
    }
}

// Answer every parked call with one refusal, writing nothing.
fn refuse(queue: &mut PendingQueue<CommandCall<EditorLoadPrefab>>, note: &RefusalNote) {
    for (_args, responder) in take_calls::<EditorLoadPrefab>(queue) {
        responder.unavailable(UnavailableCode::MissingModel, note.clone());
    }
}

fn handle_editor_load_prefab(
    canvas: PrefabCanvas,
    prefabs: Option<Res<PrefabRegistry>>,
    themes: Option<Res<UuidThemeRegistry>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorLoadPrefab>>>,
) {
    if queue.is_empty() {
        return;
    }
    let Some(prefabs) = prefabs else {
        refuse(&mut queue, &PREFABS_GONE);
        return;
    };
    let Some(themes) = themes else {
        refuse(&mut queue, &THEMES_GONE);
        return;
    };
    let PrefabCanvas {
        session,
        map,
        edit_level,
    } = canvas;
    let (Some(mut session), Some(mut map), Some(mut edit_level)) = (session, map, edit_level)
    else {
        refuse(&mut queue, &CANVAS_GONE);
        return;
    };
    for (args, responder) in take_calls::<EditorLoadPrefab>(&mut queue) {
        let outcome = match asked_for(&prefabs, &args).cloned() {
            Some(spec) => {
                open_prefab(&mut session, &mut map, &mut edit_level, &themes, &spec);
                EditorLoadPrefabOutcomeNet::Opened {
                    grid_size:  EditorGridSizeNet::from_size(spec.size),
                    placements: PrefabPlacementCountNet::new(spec.placements.len()),
                }
            }
            None => missed(&args),
        };
        responder.answer(&EditorLoadPrefabReply { outcome });
    }
}
