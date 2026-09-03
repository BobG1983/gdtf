//! off the authored `at` + the synchronously-reserved `Entity`.

use bevy::{
    prelude::Commands,
    scene::{CommandsSceneExt, EntityCommandsSceneExt},
};

use super::{
    armor_scenes::worn_piece_scenes,
    ganger_scene::ganger_scene,
    resolve::ResolvedWeapon,
    weapon_scenes::{wielded_melee_weapon_scenes, wielded_weapon_scenes},
};
use crate::{
    armor::{ArmorSpec, Wears},
    clearance::silhouette_band,
    ganger::GangMember,
    occupancy::OccupantPlacement,
    situation::PlacedGanger,
    tuning::GangerStatTuning,
    weapon::{MeleeWeaponBundle, PendingAttachments, Wields},
};

pub(super) fn spawn_gangers(
    commands: &mut Commands,
    resolved: &[(&PlacedGanger, &GangMember)],
    weapon_bundles: Vec<Option<ResolvedWeapon>>,
    melee_bundles: Vec<(MeleeWeaponBundle, PendingAttachments)>,
    armor_specs: Vec<Option<ArmorSpec>>,
    stat_tuning: &GangerStatTuning,
) -> Vec<OccupantPlacement> {
    let mut occupants = Vec::with_capacity(resolved.len());
    for ((((placed, member), weapon), (melee_bundle, melee_pending)), armor_spec) in resolved
        .iter()
        .copied()
        .zip(weapon_bundles)
        .zip(melee_bundles)
        .zip(armor_specs)
    {
        let entity = commands
            .spawn_scene(ganger_scene(placed, member, stat_tuning))
            .id();
        if let Some(armor_spec) = armor_spec {
            commands
                .entity(entity)
                .queue_spawn_related_scenes::<Wears>(worn_piece_scenes(&armor_spec));
        }
        if let Some(ResolvedWeapon {
            bundle,
            siblings,
            pending,
        }) = weapon
        {
            commands
                .entity(entity)
                .queue_spawn_related_scenes::<Wields>(wielded_weapon_scenes(
                    &bundle, siblings, pending,
                ));
        }
        commands
            .entity(entity)
            .queue_spawn_related_scenes::<Wields>(wielded_melee_weapon_scenes(
                &melee_bundle,
                melee_pending,
            ));
        occupants.push(OccupantPlacement::new(
            placed.at,
            entity,
            silhouette_band(*placed.stance),
        ));
    }
    occupants
}
