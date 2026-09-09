//! Handle [`SetupBattleRequested`] and signal [`BattleReady`].

use bevy::prelude::{Commands, MessageReader, MessageWriter, error};

use super::{content::BattleContent, runtime_seed::insert_battle_runtime};
use crate::{
    battle::messages::{BattleReady, SetupBattleRequested},
    situation::{BattleRegistries, setup_battle},
    tuning::{CombatTuning, GangerStatTuning},
};

/// Spawn a battle from the requested situation once registries are loaded.
pub fn setup_battle_on_request(
    mut requests: MessageReader<SetupBattleRequested>,
    mut ready: MessageWriter<BattleReady>,
    content: BattleContent,
    mut commands: Commands,
) {
    for request in requests.read() {
        let Some(gangs) = content.gangs.as_deref() else {
            error!(
                "battle setup requested but no GangRegistry is loaded; no BattleReady will be \
                 signalled (the gangs folder must load before a battle starts)"
            );
            continue;
        };
        let Some(weapons) = content.weapons.as_deref() else {
            error!(
                "battle setup requested but no WeaponRegistry is loaded; no BattleReady will be \
                 signalled (the weapons folder must load before a battle starts)"
            );
            continue;
        };
        let Some(melee_weapons) = content.melee_weapons.as_deref() else {
            error!(
                "battle setup requested but no MeleeWeaponRegistry is loaded; no BattleReady will \
                 be signalled (the melee weapons folder must load before a battle starts)"
            );
            continue;
        };
        let Some(armor) = content.armor.as_deref() else {
            error!(
                "battle setup requested but no ArmorRegistry is loaded; no BattleReady will be \
                 signalled (the armor folder must load before a battle starts)"
            );
            continue;
        };
        let default_stat_tuning = GangerStatTuning::default();
        let stat_tuning = content
            .stat_tuning
            .as_deref()
            .unwrap_or(&default_stat_tuning);

        let default_combat_tuning = CombatTuning::default();
        let fallback_floor_cost = content
            .combat_tuning
            .as_deref()
            .map_or(default_combat_tuning.move_costs.open, |ct| {
                ct.move_costs.open
            });

        let mut battle_registries = BattleRegistries::new(
            gangs,
            weapons,
            melee_weapons,
            armor,
            stat_tuning,
            content.terrain.as_deref(),
        );
        if let Some(field_defs) = content.field_defs.as_deref() {
            battle_registries = battle_registries.with_field_defs(field_defs);
        }
        if let Some(attachments) = content.attachments.as_deref() {
            battle_registries = battle_registries.with_attachments(attachments);
        }
        match setup_battle(
            &request.situation.map,
            &request.placements,
            battle_registries,
            fallback_floor_cost,
            &mut commands,
        ) {
            Ok(_setup) => {
                insert_battle_runtime(&mut commands, request);
                ready.write(BattleReady);
            }
            Err(e) => {
                error!(
                    "battle setup failed: {e} (an invalid vertical link, an unresolved \
                     weapon/armor/terrain key, a floor cost below the A* admissibility \
                     floor, or two gangers stacked on one spawn cell); no BattleReady \
                     will be signalled"
                );
            }
        }
    }
}
