use bevy::prelude::*;
use gdtf_battle_input::{SelectedFireMode, SelectedShooter, firing_weapon_of, mode_spec_for};
use gdtf_battle_sim::weapon::{FireMode, MeleeWeapon, MountedWeapon, WieldedBy, Wields};
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use serde::{Deserialize, Serialize};

use crate::dev::net_qa::{
    commands::read::availability::battle_is_live, facts::GameFacts, wire::misc::ModeKindNet,
};

/// The refusal a call gets when the fire-mode resources are not up.
const NO_MODEL: RefusalNote = RefusalNote::from_static(
    "the battle's selection and fire-mode resources are not up, so there is no mode to set",
);

/// The refusal a call gets with nobody selected to set a mode on.
const NO_SHOOTER: RefusalNote = RefusalNote::from_static(
    "the mode panel writes the selected shooter's mode, and no shooter is selected",
);

/// The refusal a call gets when the selected shooter has no gun.
const NO_WEAPON: RefusalNote = RefusalNote::from_static(
    "the selected shooter has no weapon to fire, so there is no fire mode to pick from",
);

/// The refusal a call gets when the gun does not offer the mode asked for.
const NO_SUCH_MODE: RefusalNote = RefusalNote::from_static(
    "the weapon the selected shooter fires does not offer that fire mode — read the modes it does \
     offer off its weapon spec",
);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BattleSetFireModeArgs {
    /// Fire mode to select, as `battle.selection` reports the live one.
    mode: ModeKindNet,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct BattleSetFireModeReply {
    /// Fire mode the shooter is now on.
    mode: ModeKindNet,
}

pub(crate) struct BattleSetFireMode;

impl QaCommand for BattleSetFireMode {
    type Args = BattleSetFireModeArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = BattleSetFireModeReply;

    const NAME: CommandName = CommandName::from_static("battle.set_fire_mode");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Select a fire mode on the weapon the selected shooter fires — the mounted gun while it \
         mans an emplacement, else the gun in its hands — writing the same resource the action \
         bar's mode panel writes through the same lookup. Needs a running battle with its sim \
         state loaded, a selected shooter, a weapon it fires, and that weapon to offer the mode \
         asked for; each missing piece is refused MissingModel with a note naming which.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        battle_is_live(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_battle_set_fire_mode.after(QaCommandSystems::Claim),
        );
    }
}

fn handle_battle_set_fire_mode(
    mut queue: ResMut<PendingQueue<CommandCall<BattleSetFireMode>>>,
    selected: Option<Res<SelectedShooter>>,
    fire_mode: Option<ResMut<SelectedFireMode>>,
    wields: Query<&Wields>,
    weapons: Query<&FireMode, With<WieldedBy>>,
    mounted: Query<(), With<MountedWeapon>>,
    melee: Query<(), With<MeleeWeapon>>,
) {
    if queue.is_empty() {
        return;
    }
    let (Some(selected), Some(mut fire_mode)) = (selected, fire_mode) else {
        for (_args, responder) in take_calls::<BattleSetFireMode>(&mut queue) {
            responder.unavailable(UnavailableCode::MissingModel, NO_MODEL);
        }
        return;
    };
    for (args, responder) in take_calls::<BattleSetFireMode>(&mut queue) {
        let Some(shooter) = **selected else {
            responder.unavailable(UnavailableCode::MissingModel, NO_SHOOTER);
            continue;
        };
        if firing_weapon_of(shooter, &wields, &mounted, &melee).is_none() {
            responder.unavailable(UnavailableCode::MissingModel, NO_WEAPON);
            continue;
        }
        let Some(spec) = mode_spec_for(
            *selected,
            &wields,
            &weapons,
            &mounted,
            &melee,
            args.mode.to_sim(),
        ) else {
            responder.unavailable(UnavailableCode::MissingModel, NO_SUCH_MODE);
            continue;
        };
        let next = SelectedFireMode::new(spec);
        if *fire_mode != next {
            *fire_mode = next;
        }
        responder.answer(&BattleSetFireModeReply {
            mode: ModeKindNet::from_sim(spec.kind),
        });
    }
}
