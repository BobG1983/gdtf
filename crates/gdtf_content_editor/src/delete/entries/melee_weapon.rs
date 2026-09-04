//! The melee weapon delete: every gang member holding it is written back holding none.

use gdtf_assets::{ContentMemberKey, ContentSourcePaths, FindingFamily};
use gdtf_battle_sim::weapon::{MeleeWeaponRegistry, MeleeWeaponSpec, WeaponName};
use gdtf_content_families::MeleeWeaponsFamily;

use super::gangs::{GANG_FAMILY, drop_from_gang_members};
use crate::{
    delete::{
        registry::{DeleteEntry, DeleteScreen},
        resolution::DroppedReferences,
    },
    mode::EditorMode,
};

/// The finding family label every melee weapon reference finding carries.
pub(crate) const MELEE_WEAPON_FAMILY: &str = "MeleeWeaponRegistry";

/// The delete for one melee weapon record, offered on the Melee weapon tab.
pub(crate) fn melee_weapon_delete_entry() -> DeleteEntry {
    DeleteEntry::new(
        FindingFamily::new(MELEE_WEAPON_FAMILY.to_owned()),
        DeleteScreen::new(EditorMode::MeleeWeapon, None),
        Box::new(|world, key| {
            let mut registry = world.get_resource_mut::<MeleeWeaponRegistry>()?;
            let spec = registry.remove(&weapon_name(key))?;
            Some(Box::new(spec))
        }),
        Box::new(|world, key, record| {
            let Ok(spec) = record.downcast::<MeleeWeaponSpec>() else {
                return;
            };
            if let Some(mut registry) = world.get_resource_mut::<MeleeWeaponRegistry>() {
                registry.insert(weapon_name(key), *spec);
            }
        }),
        Box::new(|world, key| {
            let sources = world.get_resource::<ContentSourcePaths<MeleeWeaponsFamily>>()?;
            sources.path(key).cloned()
        }),
    )
    .dropping_from(
        FindingFamily::new(GANG_FAMILY.to_owned()),
        Box::new(|world, key| {
            // A member written back to no melee weapon resolves to the default, which has to be there.
            let resolves = world
                .get_resource::<MeleeWeaponRegistry>()
                .is_some_and(|registry| registry.fists().is_some());
            if !resolves {
                return DroppedReferences::Nothing;
            }
            let held = weapon_name(key);
            drop_from_gang_members(world, |member| {
                let names_it = member.melee_weapon.as_ref() == Some(&held);
                if names_it {
                    member.melee_weapon = None;
                }
                names_it
            })
        }),
    )
}

// The registry key the member key names.
fn weapon_name(key: &ContentMemberKey) -> WeaponName {
    WeaponName::new((**key).clone())
}
