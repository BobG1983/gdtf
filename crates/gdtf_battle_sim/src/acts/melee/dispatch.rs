//! Process melee requests against gangers or structure cells.

use bevy::prelude::MessageReader;

use super::{
    MeleeArms, MeleeCombatants, MeleeOutcomes, MeleeRngs, MeleeWorld,
    ganger::resolve_ganger_melee,
    snapshot::{AttackerSnapshot, MeleeStreams},
    structure::resolve_structure_melee,
};
use crate::{
    acts::request::{MeleeRequested, MeleeTarget},
    armor::WornArmor,
    ganger::Tu,
    melee::MeleeWeaponHit,
    weapon::DamageType,
};

/// Resolve each melee request against a ganger or structure.
pub fn dispatch_melee(
    mut requests: MessageReader<MeleeRequested>,
    mut combatants: MeleeCombatants,
    mut armor: WornArmor,
    arms: MeleeArms,
    mut world: MeleeWorld,
    rngs: MeleeRngs,
    mut outcomes: MeleeOutcomes,
) {
    let (Some(mut fight_rng), Some(mut shot_rng), Some(mut severity_rng), Some(mut injury_rng)) =
        (rngs.fight, rngs.shot, rngs.severity, rngs.injury)
    else {
        return;
    };
    for request in requests.read() {
        let Ok((&atk_pos, &atk_stance, &atk_facing, &atk_fight, &atk_faction, &atk_luck)) =
            combatants.geom.get(request.attacker)
        else {
            continue;
        };

        let Some(weapon_entity) = arms
            .wields
            .get(request.attacker)
            .ok()
            .and_then(|w| w.melee_weapon(|entity| arms.melee.get(entity).is_ok()))
        else {
            continue;
        };
        let Ok((damage, punch, shred, damage_type, fatal_bias, fight_mode, shove)) =
            arms.weapons.get(weapon_entity)
        else {
            continue;
        };
        let weapon = MeleeWeaponHit {
            damage,
            punch,
            shred,
            damage_type,
            fatal_bias,
        };
        let tu_cost = Tu::new(u8::try_from(*fight_mode.primary().tu_cost).unwrap_or(u8::MAX));
        let strike_damage_type: DamageType = *damage_type;

        let attacker = AttackerSnapshot {
            entity: request.attacker,
            position: atk_pos,
            stance: atk_stance,
            facing: atk_facing,
            fight: atk_fight,
            faction: atk_faction,
            luck: atk_luck,
            weapon,
            tu_cost,
            strike_damage_type,
            shove: *shove,
        };
        match request.target {
            MeleeTarget::Ganger(target_entity) => resolve_ganger_melee(
                &attacker,
                target_entity,
                &mut combatants,
                &mut armor,
                &mut world,
                MeleeStreams {
                    fight:    &mut fight_rng,
                    shot:     &mut shot_rng,
                    severity: &mut severity_rng,
                    injury:   &mut injury_rng,
                },
                &mut outcomes,
            ),

            MeleeTarget::Structure(at) => {
                resolve_structure_melee(&attacker, at, &mut combatants, &mut world, &mut outcomes);
            }
        }
    }
}
