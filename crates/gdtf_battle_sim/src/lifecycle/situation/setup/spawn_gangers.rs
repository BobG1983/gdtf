//! [`spawn_gangers`] — phase 1 of [`setup_battle`](super::setup_battle): pour each
//! resolved ganger + its related equipment entities into the ECS, keying occupancy
//! off the authored `at` + the synchronously-reserved `Entity`.

use bevy::{
    prelude::Commands,
    scene::{CommandsSceneExt, EntityCommandsSceneExt},
};

use super::{
    armor_scenes::worn_piece_scenes,
    ganger_scene::ganger_scene,
    weapon_scenes::{wielded_melee_weapon_scenes, wielded_weapon_scenes},
};
use crate::{
    armor::{ArmorSpec, Wears},
    clearance::silhouette_band,
    ganger::GangMember,
    occupancy::OccupantPlacement,
    situation::PlacedGanger,
    tuning::GangerStatTuning,
    weapon::{MeleeWeaponBundle, PendingAttachments, WeaponBundle, WeaponSpawnSiblings, Wields},
};

/// Phase 1 — spawn each ganger with its full component set + seeded worn armor + the
/// resolved [`WeaponBundle`] as a single `bsn!` Scene (GTW-322), keeping the
/// synchronously-reserved Entity handle (never a numeric id — GTW-10 / GTW-12).
/// `commands.spawn_scene(..)` reserves the Entity id immediately (so it can key
/// occupancy), but the scene's COMPONENTS materialize a frame later on the
/// `SpawnScene` schedule — so occupancy is keyed off the ganger's authored `at`
/// value + the reserved id, never off the deferred
/// [`Position`](crate::ganger::Position) component.
pub(super) fn spawn_gangers(
    commands: &mut Commands,
    resolved: &[(&PlacedGanger, &GangMember)],
    weapon_bundles: Vec<(WeaponBundle, WeaponSpawnSiblings, PendingAttachments)>,
    melee_bundles: Vec<(MeleeWeaponBundle, PendingAttachments)>,
    armor_specs: Vec<ArmorSpec>,
    stat_tuning: &GangerStatTuning,
) -> Vec<OccupantPlacement> {
    let mut occupants = Vec::with_capacity(resolved.len());
    for (
        (
            ((placed, member), (weapon_bundle, weapon_siblings, weapon_pending)),
            (melee_bundle, melee_pending),
        ),
        armor_spec,
    ) in resolved
        .iter()
        .copied()
        .zip(weapon_bundles)
        .zip(melee_bundles)
        .zip(armor_specs)
    {
        // The ganger carries its OWN state only — NO equipment stat data (GTW-323
        // slice 3, ADR-0004). The empty InflictedWounds record (GTW-279) and the
        // GTW-291 display ceilings (HpMax / WoundsMax) ride inside `ganger_scene`; the
        // weapon + armor stats live on the related entities spawned below. GTW-414: the
        // ganger's identity + attributes come from the resolved gang-roster `member`, its
        // placement + faction from the situation-side `placed`.
        let entity = commands
            .spawn_scene(ganger_scene(placed, member, stat_tuning))
            .id();
        // GTW-323 slice 1 (ADR-0004): spawn the six worn-armor-piece entities from the
        // resolved spec and relate them to this ganger via `Wears` — using `bsn!`
        // (`queue_spawn_related_scenes::<Wears>(bsn_list!{..})`), the post-GTW-322 spawn
        // form. The framework inserts `WornBy(entity)` on each spawned piece, whose hook
        // populates the ganger's `Wears` collection. `linked_spawn` makes the pieces
        // battle-local (despawning the ganger cascade-despawns them). These piece
        // entities are the ONLY armor storage — the sim's hit-pipeline read+wear AND the
        // presenter read both go through the relationship (no on-ganger `WornArmor`).
        commands
            .entity(entity)
            .queue_spawn_related_scenes::<Wears>(worn_piece_scenes(&armor_spec));
        // GTW-323 slice 2 (ADR-0004): spawn the wielded-weapon entity from the resolved
        // `WeaponBundle` and relate it to this ganger via `Wields` — using `bsn!`
        // (`queue_spawn_related_scenes::<Wields>(..)`), the post-GTW-322 spawn form
        // (mirroring the `Wears` spawn above). The framework inserts `WieldedBy(entity)`
        // on the spawned weapon, whose hook populates the ganger's `Wields` collection.
        // `linked_spawn` makes the weapon battle-local (despawning the ganger
        // cascade-despawns it). This weapon entity is the ONLY weapon storage — the
        // sim's `fire()` read+wear AND the presenter's weapon panel / fire-mode reads
        // both go through `ganger → Wields → the weapon entity` (no on-ganger copy).
        // GTW-544/547: the optional `dot` / `on_death` siblings ride onto the SAME weapon
        // entity via the scene composition. GTW-549: the resolved attachment effects ride as a
        // PendingAttachments marker the post-spawn `apply_pending_attachments` system applies via
        // the `attach_to_weapon` extension (the deferred-spawn bridge — the weapon entity's stat
        // components exist once the scene materializes).
        commands
            .entity(entity)
            .queue_spawn_related_scenes::<Wields>(wielded_weapon_scenes(
                &weapon_bundle,
                weapon_siblings,
                weapon_pending,
            ));
        // GTW-505: spawn the wielded MELEE weapon entity from the resolved
        // `MeleeWeaponBundle` and relate it to this ganger via the SAME `Wields`
        // relationship (the ranged weapon spawn above). The framework inserts
        // `WieldedBy(entity)` on the spawned melee weapon, adding it to the ganger's
        // `Wields` collection ALONGSIDE the ranged weapon. The melee entity carries the
        // `MeleeWeapon` marker, so the ranged-firing path resolves the GUN via
        // `Wields::ranged_weapon` (which excludes it) — relating it never regresses ranged
        // firing (GTW-505 C5). RESULT: every spawned ganger wields BOTH a ranged and a
        // melee weapon (the GTW-37 D3 ruling — any ganger can melee). GTW-554: the melee
        // weapon's slot-gated attachment effects ride as its own PendingAttachments marker.
        commands
            .entity(entity)
            .queue_spawn_related_scenes::<Wields>(wielded_melee_weapon_scenes(
                &melee_bundle,
                melee_pending,
            ));
        // The occupant's silhouette band is derived from its authored stance
        // (standing → HIGH, kneeling → MID, prone → LOW) so the grid pour places the
        // occupant AND its band together (GTW-304). Keyed off the authored `at` value
        // (NOT the deferred Position component) and the reserved Entity id.
        occupants.push(OccupantPlacement::new(
            placed.at,
            entity,
            silhouette_band(*placed.stance),
        ));
    }
    occupants
}
