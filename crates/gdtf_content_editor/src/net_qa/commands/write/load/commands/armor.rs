//! `editor.load_armor` fills the Armor draft from the armor registry.

use bevy::prelude::*;
use gdtf_battle_sim::armor::ArmorRegistry;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use serde::{Deserialize, Serialize};

use super::super::families::{KeyLookup, load_armor};
use crate::{
    armor_form::ArmorDraft,
    net_qa::{
        commands::availability::only_on_the_tab,
        facts::EditorFacts,
        schedule::EditorNetQaSystems,
        wire::{EditorKeyNet, EditorLoadOutcomeNet, EditorModeNet},
    },
};

const NO_DRAFT: RefusalNote = RefusalNote::from_static(
    "editor.load_armor fills the Armor draft, which the editor only creates on entering Editing",
);

const NOT_THE_ARMOR_TAB: RefusalNote = RefusalNote::from_static(
    "the armor form's load picker is drawn only on the Armor tab, so this load needs that tab \
     open",
);

const REGISTRY_GONE: RefusalNote =
    RefusalNote::from_static("the Armor draft or the armor registry is not in the world");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::net_qa) struct EditorLoadArmorArgs {
    key: EditorKeyNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::net_qa) struct EditorLoadArmorReply {
    outcome: EditorLoadOutcomeNet,
}

pub(in crate::net_qa) struct EditorLoadArmor;

impl QaCommand for EditorLoadArmor {
    type Args = EditorLoadArmorArgs;
    type Facts = EditorFacts;
    type Parked = ();
    type Reply = EditorLoadArmorReply;

    const NAME: CommandName = CommandName::from_static("editor.load_armor");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Load an armor by key into the Armor draft, through that draft's own `load_armor` method. \
         A key the registry does not hold answers `NoSuchKey` with the keys it does hold, and \
         leaves the draft alone. Needs the Armor tab open.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &EditorFacts) -> CommandAvailability {
        only_on_the_tab(*facts, EditorModeNet::Armor, NO_DRAFT, NOT_THE_ARMOR_TAB)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_editor_load_armor
                .after(QaCommandSystems::Claim)
                .in_set(EditorNetQaSystems::Gather),
        );
    }
}

fn handle_editor_load_armor(
    draft: Option<ResMut<ArmorDraft>>,
    registry: Option<Res<ArmorRegistry>>,
    mut queue: ResMut<PendingQueue<CommandCall<EditorLoadArmor>>>,
) {
    if queue.is_empty() {
        return;
    }
    let (Some(mut draft), Some(registry)) = (draft, registry) else {
        for (_args, responder) in take_calls::<EditorLoadArmor>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, REGISTRY_GONE);
        }
        return;
    };
    for (args, responder) in take_calls::<EditorLoadArmor>(&mut queue) {
        let outcome = match load_armor(&mut draft, &registry, &args.key) {
            KeyLookup::Loaded => EditorLoadOutcomeNet::Loaded { key: args.key },
            KeyLookup::NoSuchKey(known) => EditorLoadOutcomeNet::NoSuchKey {
                key: args.key,
                known,
            },
        };
        responder.answer(&EditorLoadArmorReply { outcome });
    }
}
