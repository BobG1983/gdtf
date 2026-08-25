//! `editor.select_theme`. Pick the top bar's theme, writing the authoring session.

use bevy::{asset::uuid::Uuid, prelude::*};
use gdtf_battle_sim::level::{ThemeUuid, UuidThemeRegistry};
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
        commands::availability::only_while_editing,
        facts::EditorFacts,
        schedule::EditorNetQaSystems,
        wire::{TerrainKeyNet, ThemeKeyNet},
    },
    session::MapEditorSession,
};

const NO_SESSION: RefusalNote = RefusalNote::from_static(
    "editor.select_theme writes the authoring session, which the editor only creates on entering \
     Editing",
);

const SESSION_GONE: RefusalNote =
    RefusalNote::from_static("the authoring session or the theme registry is not in the world");

const NOT_A_THEME_KEY: RefusalNote = RefusalNote::from_static(
    "a theme key is the hyphenated UUID text editor.families answers, and this one does not parse \
     as that",
);

const NO_SUCH_THEME: RefusalNote = RefusalNote::from_static(
    "the theme registry holds no theme under that key, so the session keeps the theme it had",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorSelectThemeArgs {
    key: ThemeKeyNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::net_qa) struct EditorSelectThemeReply {
    theme:         ThemeKeyNet,
    default_floor: Option<TerrainKeyNet>,
}

pub(in crate::net_qa) struct EditorSelectTheme;

impl QaCommand for EditorSelectTheme {
    type Args = EditorSelectThemeArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorSelectThemeReply;

    const NAME: CommandName = CommandName::from_static("editor.select_theme");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Select a theme in the authoring session, the way the top bar's theme picker does: the \
         theme and the default floor that theme's registry entry names. The top bar is drawn on \
         every tab, so this needs no particular mode.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_while_editing(*facts, NO_SESSION)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_select_theme
                .after(QaCommandSystems::Claim)
                .in_set(EditorNetQaSystems::Gather),
        );
    }
}

// Read back out of the session, so the reply is what the write left rather than what it asked.
fn reply_from(session: &MapEditorSession) -> EditorSelectThemeReply {
    EditorSelectThemeReply {
        theme:         ThemeKeyNet::new((*session.theme()).to_string()),
        default_floor: session
            .default_floor()
            .map(|floor| TerrainKeyNet::new((*floor).to_string())),
    }
}

fn handle_editor_select_theme(
    session: Option<ResMut<MapEditorSession>>,
    themes: Option<Res<UuidThemeRegistry>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorSelectTheme>>>,
) {
    if queue.is_empty() {
        return;
    }
    let (Some(mut session), Some(themes)) = (session, themes) else {
        for (_args, responder) in take_calls::<EditorSelectTheme>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, SESSION_GONE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorSelectTheme>(&mut queue) {
        let Ok(parsed) = Uuid::parse_str(&args.key) else {
            responder.unavailable(UnavailableCode::MissingModel, NOT_A_THEME_KEY);
            continue;
        };
        let key = ThemeUuid::new(parsed);
        if themes.def(&key).is_none() {
            responder.unavailable(UnavailableCode::MissingModel, NO_SUCH_THEME);
            continue;
        }
        let default_floor = themes.default_floor(&key);
        session.select_theme(key, default_floor);
        responder.answer(&reply_from(&session));
    }
}
