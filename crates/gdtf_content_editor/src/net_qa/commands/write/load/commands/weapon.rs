//! `editor.load_weapon` fills the Weapon draft from the ranged weapon registry.

use bevy::prelude::*;
use gdtf_battle_sim::weapon::WeaponRegistry;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use serde::{Deserialize, Serialize};

use super::super::families::{KeyLookup, load_weapon};
use crate::{
    net_qa::{
        commands::availability::only_on_the_tab,
        facts::EditorFacts,
        schedule::EditorNetQaSystems,
        wire::{EditorKeyNet, EditorLoadOutcomeNet, EditorModeNet},
    },
    weapon_form::WeaponDraft,
};

const NO_DRAFT: RefusalNote = RefusalNote::from_static(
    "editor.load_weapon fills the Weapon draft, which the editor only creates on entering Editing",
);

const NOT_THE_WEAPON_TAB: RefusalNote = RefusalNote::from_static(
    "the weapon form's load picker is drawn only on the Weapon tab, so this load needs that tab \
     open",
);

const REGISTRY_GONE: RefusalNote =
    RefusalNote::from_static("the Weapon draft or the weapon registry is not in the world");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorLoadWeaponArgs {
    key: EditorKeyNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::net_qa) struct EditorLoadWeaponReply {
    outcome: EditorLoadOutcomeNet,
}

pub(in crate::net_qa) struct EditorLoadWeapon;

impl QaCommand for EditorLoadWeapon {
    type Args = EditorLoadWeaponArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorLoadWeaponReply;

    const NAME: CommandName = CommandName::from_static("editor.load_weapon");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Load a ranged weapon by key into the Weapon draft, through that draft's own \
         `load_weapon` method. A key the registry does not hold answers `NoSuchKey` with the keys \
         it does hold, and leaves the draft alone. Needs the Weapon tab open.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_tab(*facts, EditorModeNet::Weapon, NO_DRAFT, NOT_THE_WEAPON_TAB)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_load_weapon
                .after(QaCommandSystems::Claim)
                .in_set(EditorNetQaSystems::Gather),
        );
    }
}

fn handle_editor_load_weapon(
    draft: Option<ResMut<WeaponDraft>>,
    registry: Option<Res<WeaponRegistry>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorLoadWeapon>>>,
) {
    if queue.is_empty() {
        return;
    }
    let (Some(mut draft), Some(registry)) = (draft, registry) else {
        for (_args, responder) in take_calls::<EditorLoadWeapon>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, REGISTRY_GONE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorLoadWeapon>(&mut queue) {
        let outcome = match load_weapon(&mut draft, &registry, &args.key) {
            KeyLookup::Loaded => EditorLoadOutcomeNet::Loaded { key: args.key },
            KeyLookup::NoSuchKey(known) => EditorLoadOutcomeNet::NoSuchKey {
                key: args.key,
                known,
            },
        };
        responder.answer(&EditorLoadWeaponReply { outcome });
    }
}
