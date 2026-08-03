use bevy::prelude::{Commands, MessageReader, MessageWriter, Res, error};

use super::runtime_seed::insert_battle_runtime;
use crate::{
    armor::ArmorRegistry,
    battle::messages::{BattleReady, SetupBattleRequested},
    equipment::attachments::AttachmentRegistry,
    ganger::GangRegistry,
    situation::{BattleRegistries, setup_battle},
    terrain::def::TerrainDefRegistry,
    tuning::{CombatTuning, GangerStatTuning},
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};

#[expect(
    clippy::too_many_arguments,
    reason = "the params are the message reader + writer, the gang / weapon / MELEE-weapon \
              (GTW-505) / armor / terrain / area-damage-field (GTW-545) / attachment \
              (GTW-549) registries, and the stat + combat tuning — each a distinct Bevy \
              SystemParam (Option<Res<_>> for the Load-state registries); the injection model \
              cannot be refactored to fewer without a wrapper resource that changes the API \
              surface"
)]
pub fn setup_battle_on_request(
    mut requests: MessageReader<SetupBattleRequested>,
    mut ready: MessageWriter<BattleReady>,
    gangs: Option<Res<GangRegistry>>,
    weapons: Option<Res<WeaponRegistry>>,
    melee_weapons: Option<Res<MeleeWeaponRegistry>>,
    armor: Option<Res<ArmorRegistry>>,
    terrain: Option<Res<TerrainDefRegistry>>,
    stat_tuning: Option<Res<GangerStatTuning>>,
    combat_tuning: Option<Res<CombatTuning>>,
    field_defs: Option<Res<crate::effects::fields::FieldDefRegistry>>,
    attachments: Option<Res<AttachmentRegistry>>,
    mut commands: Commands,
) {
    for request in requests.read() {
        let Some(gangs) = gangs.as_deref() else {
            error!(
                "battle setup requested but no GangRegistry is loaded; no BattleReady will be \
                 signalled (the gangs folder must load before a battle starts)"
            );
            continue;
        };
        let Some(weapons) = weapons.as_deref() else {
            error!(
                "battle setup requested but no WeaponRegistry is loaded; no BattleReady will be \
                 signalled (the weapons folder must load before a battle starts)"
            );
            continue;
        };
        let Some(melee_weapons) = melee_weapons.as_deref() else {
            error!(
                "battle setup requested but no MeleeWeaponRegistry is loaded; no BattleReady will \
                 be signalled (the melee weapons folder must load before a battle starts)"
            );
            continue;
        };
        let Some(armor) = armor.as_deref() else {
            error!(
                "battle setup requested but no ArmorRegistry is loaded; no BattleReady will be \
                 signalled (the armor folder must load before a battle starts)"
            );
            continue;
        };
        let default_stat_tuning = GangerStatTuning::default();
        let stat_tuning = stat_tuning.as_deref().unwrap_or(&default_stat_tuning);

        let default_combat_tuning = CombatTuning::default();
        let fallback_floor_cost = combat_tuning
            .as_deref()
            .map_or(default_combat_tuning.move_costs.open, |ct| {
                ct.move_costs.open
            });

        let terrain_ref = terrain.as_deref();

        let field_defs_ref = field_defs.as_deref();

        let attachments_ref = attachments.as_deref();

        let mut battle_registries = BattleRegistries::new(
            gangs,
            weapons,
            melee_weapons,
            armor,
            stat_tuning,
            terrain_ref,
        );
        if let Some(field_defs) = field_defs_ref {
            battle_registries = battle_registries.with_field_defs(field_defs);
        }
        if let Some(attachments) = attachments_ref {
            battle_registries = battle_registries.with_attachments(attachments);
        }
        match setup_battle(
            &request.situation,
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
