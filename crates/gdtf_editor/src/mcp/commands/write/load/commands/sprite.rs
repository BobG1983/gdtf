//! `editor.load_sprite` fills the Sprite draft from the sprite registry.

use bevy::prelude::*;
use cobalt_mcp_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_content_families::sprites::SpriteDefRegistry;
use serde::{Deserialize, Serialize};

use super::super::families::{KeyLookup, load_sprite};
use crate::{
    mcp::{
        commands::availability::only_on_the_tab,
        facts::EditorFacts,
        schedule::EditorMcpSystems,
        wire::{EditorKeyNet, EditorLoadOutcomeNet, EditorModeNet},
    },
    sprite_form::SpriteDraft,
};

const NO_DRAFT: RefusalNote = RefusalNote::from_static(
    "editor.load_sprite fills the Sprite draft, which the editor only creates on entering Editing",
);

const NOT_THE_SPRITE_TAB: RefusalNote = RefusalNote::from_static(
    "the sprite form's load picker is drawn only on the Sprite tab, so this load needs that tab \
     open",
);

const REGISTRY_GONE: RefusalNote =
    RefusalNote::from_static("the Sprite draft or the sprite registry is not in the world");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorLoadSpriteArgs {
    key: EditorKeyNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::mcp) struct EditorLoadSpriteReply {
    outcome: EditorLoadOutcomeNet,
}

pub(in crate::mcp) struct EditorLoadSprite;

impl QaCommand for EditorLoadSprite {
    type Args = EditorLoadSpriteArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorLoadSpriteReply;

    const NAME: CommandName = CommandName::from_static("editor.load_sprite");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Load a sprite def by key into the Sprite draft, through that draft's own `load_sprite` \
         method. A key the registry does not hold answers `NoSuchKey` with the keys it does hold, \
         and leaves the draft alone. Needs the Sprite tab open.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_tab(*facts, EditorModeNet::Sprite, NO_DRAFT, NOT_THE_SPRITE_TAB)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_load_sprite
                .after(QaCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
        );
    }
}

fn handle_editor_load_sprite(
    draft: Option<ResMut<SpriteDraft>>,
    registry: Option<Res<SpriteDefRegistry>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorLoadSprite>>>,
) {
    if queue.is_empty() {
        return;
    }
    let (Some(mut draft), Some(registry)) = (draft, registry) else {
        for (_args, responder) in take_calls::<EditorLoadSprite>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, REGISTRY_GONE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorLoadSprite>(&mut queue) {
        let outcome = match load_sprite(&mut draft, &registry, &args.key) {
            KeyLookup::Loaded => EditorLoadOutcomeNet::Loaded { key: args.key },
            KeyLookup::NoSuchKey(known) => EditorLoadOutcomeNet::NoSuchKey {
                key: args.key,
                known,
            },
        };
        responder.answer(&EditorLoadSpriteReply { outcome });
    }
}
