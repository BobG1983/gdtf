//! `editor.load_melee_weapon` fills the `MeleeWeapon` draft from the melee weapon registry.

use bevy::prelude::*;
use cobalt_mcp_host::{
    PendingQueue,
    command::McpCommand,
    dispatch::{CommandCall, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use gdtf_battle_sim::weapon::MeleeWeaponRegistry;
use serde::{Deserialize, Serialize};

use super::super::families::{KeyLookup, load_melee_weapon};
use crate::{
    mcp::{
        commands::availability::only_on_the_tab,
        facts::EditorFacts,
        schedule::EditorMcpSystems,
        wire::{EditorKeyNet, EditorLoadOutcomeNet, EditorModeNet},
    },
    melee_weapon_form::MeleeWeaponDraft,
};

const NO_DRAFT: RefusalNote = RefusalNote::from_static(
    "editor.load_melee_weapon fills the MeleeWeapon draft, which the editor only creates on \
     entering Editing",
);

const NOT_THE_MELEE_WEAPON_TAB: RefusalNote = RefusalNote::from_static(
    "the melee weapon form's load picker is drawn only on the MeleeWeapon tab, so this load needs \
     that tab open",
);

const REGISTRY_GONE: RefusalNote = RefusalNote::from_static(
    "the MeleeWeapon draft or the melee weapon registry is not in the world",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::mcp) struct EditorLoadMeleeWeaponArgs {
    key: EditorKeyNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::mcp) struct EditorLoadMeleeWeaponReply {
    outcome: EditorLoadOutcomeNet,
}

pub(in crate::mcp) struct EditorLoadMeleeWeapon;

impl McpCommand for EditorLoadMeleeWeapon {
    type Args = EditorLoadMeleeWeaponArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorLoadMeleeWeaponReply;

    const NAME: CommandName = CommandName::from_static("editor.load_melee_weapon");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Load a melee weapon by key into the MeleeWeapon draft, through that draft's own \
         `load_melee_weapon` method. A key the registry does not hold answers `NoSuchKey` with \
         the keys it does hold, and leaves the draft alone. Needs the MeleeWeapon tab open.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_tab(
            *facts,
            EditorModeNet::MeleeWeapon,
            NO_DRAFT,
            NOT_THE_MELEE_WEAPON_TAB,
        )
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_load_melee_weapon
                .after(McpCommandSystems::Claim)
                .in_set(EditorMcpSystems::Gather),
        );
    }
}

fn handle_editor_load_melee_weapon(
    draft: Option<ResMut<MeleeWeaponDraft>>,
    registry: Option<Res<MeleeWeaponRegistry>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorLoadMeleeWeapon>>>,
) {
    if queue.is_empty() {
        return;
    }
    let (Some(mut draft), Some(registry)) = (draft, registry) else {
        for (_args, responder) in take_calls::<EditorLoadMeleeWeapon>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, REGISTRY_GONE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorLoadMeleeWeapon>(&mut queue) {
        let outcome = match load_melee_weapon(&mut draft, &registry, &args.key) {
            KeyLookup::Loaded => EditorLoadOutcomeNet::Loaded { key: args.key },
            KeyLookup::NoSuchKey(known) => EditorLoadOutcomeNet::NoSuchKey {
                key: args.key,
                known,
            },
        };
        responder.answer(&EditorLoadMeleeWeaponReply { outcome });
    }
}
