use bevy::{ecs::system::SystemParam, prelude::*};
use cobalt_mcp_command::{
    command::McpCommand,
    dispatch::{CommandCall, McpCommandSystems, take_calls},
};
use cobalt_mcp_protocol::command::{
    CommandAvailability, CommandName, CommandSummary, CommandTiming, RefusalNote, UnavailableCode,
};
use cobalt_mcp_transport::PendingQueue;
use gdtf_battle_input::{
    ChosenFireMode, FireModeSystems, SelectedShooter, firing_weapon_of, mode_spec_for,
    reset_move_target_on_fire_mode_change, set_chosen_mode,
};
use gdtf_battle_sim::weapon::{FireMode, MeleeWeapon, MountedWeapon, WieldedBy, Wields};
use serde::{Deserialize, Serialize};

use crate::dev::mcp::{
    commands::read::availability::battle_is_live, facts::GameFacts, wire::misc::ModeKindNet,
};

/// The refusal a call gets when the battle's selection resource is not up.
const NO_MODEL: RefusalNote = RefusalNote::from_static(
    "the battle's selection resource is not up, so there is no shooter to set a mode on",
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

/// The lookups this command resolves the shooter's fired weapon against.
#[derive(SystemParam)]
struct SetModeGuns<'w, 's> {
    /// Weapons each shooter holds.
    wields:  Query<'w, 's, &'static Wields>,
    /// Fire modes of a wielded weapon.
    weapons: Query<'w, 's, &'static FireMode, With<WieldedBy>>,
    /// Mounted probe used to prefer a manned mount over the carried gun.
    mounted: Query<'w, 's, (), With<MountedWeapon>>,
    /// Melee probe used to skip melee weapons when picking the ranged one.
    melee:   Query<'w, 's, (), With<MeleeWeapon>>,
}

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

impl McpCommand for BattleSetFireMode {
    type Args = BattleSetFireModeArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = BattleSetFireModeReply;

    const NAME: CommandName = CommandName::from_static("battle.set_fire_mode");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Select a fire mode on the weapon the selected shooter fires — the mounted gun while it \
         mans an emplacement, else the gun in its hands — setting it on that weapon the way the \
         action bar's mode panel does, through the same lookup. Needs a running battle with its sim \
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
            handle_battle_set_fire_mode
                .in_set(FireModeSystems::Command)
                .after(McpCommandSystems::Claim)
                .before(reset_move_target_on_fire_mode_change),
        );
    }
}

fn handle_battle_set_fire_mode(
    mut queue: ResMut<PendingQueue<CommandCall<BattleSetFireMode>>>,
    selected: Option<Res<SelectedShooter>>,
    guns: SetModeGuns,
    mut chosen: Query<&mut ChosenFireMode>,
    mut commands: Commands,
) {
    if queue.is_empty() {
        return;
    }
    let Some(selected) = selected else {
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
        let Some(weapon) = firing_weapon_of(shooter, &guns.wields, &guns.mounted, &guns.melee)
        else {
            responder.unavailable(UnavailableCode::MissingModel, NO_WEAPON);
            continue;
        };
        let Some(spec) = mode_spec_for(
            *selected,
            &guns.wields,
            &guns.weapons,
            &guns.mounted,
            &guns.melee,
            args.mode.to_sim(),
        ) else {
            responder.unavailable(UnavailableCode::MissingModel, NO_SUCH_MODE);
            continue;
        };
        set_chosen_mode(weapon, spec.kind, &guns.weapons, &mut chosen, &mut commands);
        responder.answer(&BattleSetFireModeReply {
            mode: ModeKindNet::from_sim(spec.kind),
        });
    }
}
