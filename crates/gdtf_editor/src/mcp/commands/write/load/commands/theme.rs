//! `editor.load_theme` selects a theme in the session and fills the Theme draft.

use bevy::prelude::*;
use cobalt_mcp_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_assets::ContentSourcePaths;
use gdtf_battle_sim::level::UuidThemeRegistry;
use gdtf_content_families::ThemeDefsFamily;
use serde::{Deserialize, Serialize};

use super::super::{families::KeyLookup, theme::load_theme};
use crate::{
    mcp::{
        commands::availability::only_on_the_tab,
        facts::EditorFacts,
        schedule::EditorMcpSystems,
        wire::{EditorKeyNet, EditorLoadOutcomeNet, EditorModeNet},
    },
    session::MapEditorSession,
    theme_form::ThemeDraft,
};

const NO_DRAFT: RefusalNote = RefusalNote::from_static(
    "editor.load_theme fills the Theme draft and writes the authoring session, both of which the \
     editor only creates on entering Editing",
);

const NOT_THE_THEME_TAB: RefusalNote = RefusalNote::from_static(
    "the theme form's load picker is drawn only on the Theme tab, so this load needs that tab open",
);

const REGISTRY_GONE: RefusalNote = RefusalNote::from_static(
    "the Theme draft, the authoring session or the theme registry is not in the world",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorLoadThemeArgs {
    key: EditorKeyNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::mcp) struct EditorLoadThemeReply {
    outcome: EditorLoadOutcomeNet,
}

pub(in crate::mcp) struct EditorLoadTheme;

impl QaCommand for EditorLoadTheme {
    type Args = EditorLoadThemeArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorLoadThemeReply;

    const NAME: CommandName = CommandName::from_static("editor.load_theme");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Select a theme in the authoring session and load its def into the Theme draft, the way \
         the theme form's own load does. A key the registry does not hold answers `NoSuchKey` \
         with the keys it does hold, and leaves both alone. Needs the Theme tab open.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_tab(*facts, EditorModeNet::Theme, NO_DRAFT, NOT_THE_THEME_TAB)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_load_theme
                .after(QaCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
        );
    }
}

fn handle_editor_load_theme(
    draft: Option<ResMut<ThemeDraft>>,
    session: Option<ResMut<MapEditorSession>>,
    registry: Option<Res<UuidThemeRegistry>>,
    sources: Option<Res<ContentSourcePaths<ThemeDefsFamily>>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorLoadTheme>>>,
) {
    if queue.is_empty() {
        return;
    }
    let (Some(mut draft), Some(mut session), Some(registry)) = (draft, session, registry) else {
        for (_args, responder) in take_calls::<EditorLoadTheme>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, REGISTRY_GONE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorLoadTheme>(&mut queue) {
        let outcome = match load_theme(
            &mut draft,
            &mut session,
            &registry,
            sources.as_deref(),
            &args.key,
        ) {
            KeyLookup::Loaded => EditorLoadOutcomeNet::Loaded { key: args.key },
            KeyLookup::NoSuchKey(known) => EditorLoadOutcomeNet::NoSuchKey {
                key: args.key,
                known,
            },
        };
        responder.answer(&EditorLoadThemeReply { outcome });
    }
}
