//! Process melee requests against gangers or structure cells.

use bevy::prelude::{MessageReader, MessageWriter, Query, With};

use super::{
    MeleeFacts, MeleeRngs, MeleeWorld,
    ganger::resolve_ganger_melee,
    queries::{MeleeGeomQuery, MeleeTargetQuery, MeleeWeaponQuery},
    snapshot::{AttackerSnapshot, MeleeStreams},
    structure::resolve_structure_melee,
};
use crate::{
    acts::request::{MeleeRequested, MeleeResolved, MeleeTarget, ShoveRequested},
    armor::{PieceArmorMut, Wears, WornBy},
    fire::{MeleeQuery, WieldsQuery},
    ganger::Tu,
    melee::MeleeWeaponHit,
    occupancy_sync::CoverDestroyed,
    weapon::DamageType,
};

#[expect(
    clippy::too_many_arguments,
    reason = "the melee dispatch needs the request reader, the disjoint geometry / attacker-Tu \
              / target-surfaces ganger queries, the two armor relationship queries, the wields \
              + melee-marker + melee-weapon-stat relationship queries, the grouped world reads \
              (MeleeWorld: grids + tuning + injury content) + draw streams (MeleeRngs) \
              bundles, and the MeleeResolved + MeleeStruck + InjuryInflicted + CoverDestroyed \
              + ShoveRequested writers — each a distinct Bevy SystemParam (the dispatch_fire \
              argument-count carve-out)"
)]
/// Resolve each melee request against a ganger or structure.
pub fn dispatch_melee(
    mut requests: MessageReader<MeleeRequested>,
    geom: MeleeGeomQuery,
    mut tu_q: Query<&mut Tu>,
    mut targets: MeleeTargetQuery,
    wears: Query<&Wears>,
    mut pieces: Query<PieceArmorMut, With<WornBy>>,
    wields: WieldsQuery,
    melee: MeleeQuery,
    weapons: MeleeWeaponQuery,
    mut world: MeleeWorld,
    rngs: MeleeRngs,
    mut resolved: MessageWriter<MeleeResolved>,
    mut facts: MeleeFacts,
    mut cover_destroyed: MessageWriter<CoverDestroyed>,
    mut shoves: MessageWriter<ShoveRequested>,
    mut deaths: MessageWriter<crate::effects::on_death::OnDeathOccurred>,
) {
    let (Some(mut fight_rng), Some(mut shot_rng), Some(mut severity_rng), Some(mut injury_rng)) =
        (rngs.fight, rngs.shot, rngs.severity, rngs.injury)
    else {
        return;
    };
    for request in requests.read() {
        let Ok((&atk_pos, &atk_stance, &atk_facing, &atk_fight, &atk_faction, &atk_luck)) =
            geom.get(request.attacker)
        else {
            continue;
        };

        let Some(weapon_entity) = wields
            .get(request.attacker)
            .ok()
            .and_then(|w| w.melee_weapon(|entity| melee.get(entity).is_ok()))
        else {
            continue;
        };
        let Ok((damage, punch, shred, damage_type, fatal_bias, fight_mode, shove)) =
            weapons.get(weapon_entity)
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
                &geom,
                &mut tu_q,
                &mut targets,
                &wears,
                &mut pieces,
                &mut world,
                MeleeStreams {
                    fight:    &mut fight_rng,
                    shot:     &mut shot_rng,
                    severity: &mut severity_rng,
                    injury:   &mut injury_rng,
                },
                &mut resolved,
                &mut facts,
                &mut shoves,
                &mut deaths,
            ),

            MeleeTarget::Structure(at) => resolve_structure_melee(
                &attacker,
                at,
                &mut tu_q,
                &mut world,
                &mut resolved,
                &mut cover_destroyed,
                &mut deaths,
            ),
        }
    }
}
