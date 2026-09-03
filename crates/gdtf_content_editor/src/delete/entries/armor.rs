//! The armor delete: every gang member wearing it is written back wearing none.

use gdtf_assets::{ContentMemberKey, ContentSourcePaths, FindingFamily};
use gdtf_battle_sim::armor::{ArmorName, ArmorRegistry, ArmorSpec};
use gdtf_content_families::ArmorFamily;

use super::gangs::drop_from_gang_members;
use crate::{
    delete::registry::{DeleteEntry, DeleteScreen},
    mode::EditorMode,
};

/// The finding family label every armor reference finding carries.
pub(crate) const ARMOR_FAMILY: &str = "ArmorRegistry";

/// The delete for one armor record, offered on the Armor tab.
pub(crate) fn armor_delete_entry() -> DeleteEntry {
    DeleteEntry::new(
        FindingFamily::new(ARMOR_FAMILY.to_owned()),
        DeleteScreen::new(EditorMode::Armor, None),
        Box::new(|world, key| {
            let mut registry = world.get_resource_mut::<ArmorRegistry>()?;
            let spec = registry.remove(&armor_name(key))?;
            Some(Box::new(spec))
        }),
        Box::new(|world, key, record| {
            let Ok(spec) = record.downcast::<ArmorSpec>() else {
                return;
            };
            if let Some(mut registry) = world.get_resource_mut::<ArmorRegistry>() {
                registry.insert(armor_name(key), *spec);
            }
        }),
        Box::new(|world, key| {
            let sources = world.get_resource::<ContentSourcePaths<ArmorFamily>>()?;
            sources.path(key).cloned()
        }),
    )
    .with_drop(Box::new(|world, key| {
        let worn = armor_name(key);
        drop_from_gang_members(world, |member| {
            let names_it = member.armor.as_ref() == Some(&worn);
            if names_it {
                member.armor = None;
            }
            names_it
        })
    }))
}

// The registry key the member key names.
fn armor_name(key: &ContentMemberKey) -> ArmorName {
    ArmorName::new((**key).clone())
}
