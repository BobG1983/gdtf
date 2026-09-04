//! The ranged weapon delete: gang members lose it, emplacements take the replacement.

use bevy::prelude::World;
use gdtf_assets::{ContentMemberKey, ContentSourcePaths, FindingFamily};
use gdtf_battle_sim::weapon::{WeaponName, WeaponRegistry, WeaponSpec};
use gdtf_content_families::WeaponsFamily;

use super::{
    gangs::{GANG_FAMILY, drop_from_gang_members},
    terrain_defs::replace_mounted_weapon,
};
use crate::{
    delete::{
        offer::{ReplacementCandidate, ReplacementLabel, labelled_candidates},
        registry::{DeleteEntry, DeleteScreen},
        resolution::{DroppedReferences, delete_assets_root},
    },
    mode::EditorMode,
};

/// The finding family label every ranged weapon reference finding carries.
pub(crate) const WEAPON_FAMILY: &str = "WeaponRegistry";

// The family label a terrain def's own mounted-weapon reference is recorded under.
const TERRAIN_REFERRER: &str = "TerrainDefRegistry";

/// The delete for one ranged weapon, offered on the Weapon tab.
pub(crate) fn weapon_delete_entry() -> DeleteEntry {
    DeleteEntry::new(
        FindingFamily::new(WEAPON_FAMILY.to_owned()),
        DeleteScreen::new(EditorMode::Weapon, None),
        Box::new(|world, key| {
            let mut registry = world.get_resource_mut::<WeaponRegistry>()?;
            let spec = registry.remove(&weapon_name(key))?;
            Some(Box::new(spec))
        }),
        Box::new(|world, key, record| {
            let Ok(spec) = record.downcast::<WeaponSpec>() else {
                return;
            };
            if let Some(mut registry) = world.get_resource_mut::<WeaponRegistry>() {
                registry.insert(weapon_name(key), *spec);
            }
        }),
        Box::new(|world, key| {
            let sources = world.get_resource::<ContentSourcePaths<WeaponsFamily>>()?;
            sources.path(key).cloned()
        }),
    )
    .with_candidates(Box::new(weapon_candidates))
    .dropping_from(
        FindingFamily::new(GANG_FAMILY.to_owned()),
        Box::new(|world, key| {
            let held = weapon_name(key);
            drop_from_gang_members(world, |member| {
                let names_it = member.weapon.as_ref() == Some(&held);
                if names_it {
                    member.weapon = None;
                }
                names_it
            })
        }),
    )
    .replacing_in(
        FindingFamily::new(TERRAIN_REFERRER.to_owned()),
        Box::new(|world, key, replacement| {
            let deleted = weapon_name(key);
            let chosen = weapon_name(replacement);
            delete_assets_root(world).map_or(DroppedReferences::Failed, |root| {
                replace_mounted_weapon(world, &root, &deleted, &chosen)
            })
        }),
    )
}

// The registry key the member key names.
fn weapon_name(key: &ContentMemberKey) -> WeaponName {
    WeaponName::new((**key).clone())
}

// Every weapon as a replacement row; a weapon keys on its file stem, already unique.
fn weapon_candidates(world: &World) -> Vec<ReplacementCandidate> {
    world
        .get_resource::<WeaponRegistry>()
        .map_or_else(Vec::new, |registry| {
            labelled_candidates(registry.iter().map(|(name, _spec)| {
                (
                    ContentMemberKey::new((**name).clone()),
                    ReplacementLabel::new((**name).clone()),
                )
            }))
        })
}
