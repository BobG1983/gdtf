//! The attachment delete: every weapon fitted with it is written back without it.

use std::path::Path;

use bevy::prelude::World;
use gdtf_assets::{ContentMemberKey, ContentSourcePaths, FindingFamily};
use gdtf_battle_sim::{
    equipment::attachments::{
        AttachmentName, AttachmentRegistry, AttachmentSpec, FittedAttachments,
    },
    weapon::{MeleeWeaponRegistry, MeleeWeaponSpec, WeaponName, WeaponRegistry, WeaponSpec},
};
use gdtf_content_families::AttachmentsFamily;

use crate::{
    delete::{
        registry::{DeleteEntry, DeleteScreen},
        resolution::{DroppedReferences, delete_assets_root},
    },
    melee_weapon_form::write_melee_weapon_in,
    mode::EditorMode,
    weapon_form::write_weapon_in,
};

/// The finding family label every attachment reference finding carries.
pub(crate) const ATTACHMENT_FAMILY: &str = "AttachmentRegistry";

/// The delete for one attachment record, offered on the Attachment tab.
pub(crate) fn attachment_delete_entry() -> DeleteEntry {
    DeleteEntry::new(
        FindingFamily::new(ATTACHMENT_FAMILY.to_owned()),
        DeleteScreen::new(EditorMode::Attachment, None),
        Box::new(|world, key| {
            let mut registry = world.get_resource_mut::<AttachmentRegistry>()?;
            let spec = registry.remove(&attachment_name(key))?;
            Some(Box::new(spec))
        }),
        Box::new(|world, key, record| {
            let Ok(spec) = record.downcast::<AttachmentSpec>() else {
                return;
            };
            if let Some(mut registry) = world.get_resource_mut::<AttachmentRegistry>() {
                registry.insert(attachment_name(key), *spec);
            }
        }),
        Box::new(|world, key| {
            let sources = world.get_resource::<ContentSourcePaths<AttachmentsFamily>>()?;
            sources.path(key).cloned()
        }),
    )
    .with_drop(Box::new(|world, key| {
        let fitted = attachment_name(key);
        let Some(root) = delete_assets_root(world) else {
            return DroppedReferences::Failed;
        };
        let ranged = drop_from_ranged_weapons(world, &root, &fitted);
        if ranged == DroppedReferences::Failed {
            return DroppedReferences::Failed;
        }
        combined(ranged, drop_from_melee_weapons(world, &root, &fitted))
    }))
}

// The registry key the member key names.
fn attachment_name(key: &ContentMemberKey) -> AttachmentName {
    AttachmentName::new((**key).clone())
}

// A failed half loses to nothing else, and one rewrite is enough to re-check.
const fn combined(left: DroppedReferences, right: DroppedReferences) -> DroppedReferences {
    match (left, right) {
        (DroppedReferences::Failed, _) | (_, DroppedReferences::Failed) => {
            DroppedReferences::Failed
        }
        (DroppedReferences::Rewritten, _) | (_, DroppedReferences::Rewritten) => {
            DroppedReferences::Rewritten
        }
        _ => DroppedReferences::Nothing,
    }
}

// The fitted list without `name`, or nothing when the list never held it.
fn without(fitted: &FittedAttachments, name: &AttachmentName) -> Option<FittedAttachments> {
    fitted.contains(name).then(|| {
        FittedAttachments::new(fitted.iter().filter(|key| *key != name).cloned().collect())
    })
}

// Rewrite every ranged spec fitted with `name`, writing each file first.
fn drop_from_ranged_weapons(
    world: &mut World,
    root: &Path,
    name: &AttachmentName,
) -> DroppedReferences {
    let Some(registry) = world.get_resource::<WeaponRegistry>() else {
        return DroppedReferences::Nothing;
    };
    let rewritten: Vec<(WeaponName, WeaponSpec)> = registry
        .iter()
        .filter_map(|(key, spec)| {
            without(&spec.attachments, name).map(|attachments| {
                let mut next = spec.clone();
                next.attachments = attachments;
                (key.clone(), next)
            })
        })
        .collect();
    if rewritten.is_empty() {
        return DroppedReferences::Nothing;
    }
    for (key, spec) in &rewritten {
        if write_weapon_in(root, key, spec).is_err() {
            return DroppedReferences::Failed;
        }
    }
    let Some(mut registry) = world.get_resource_mut::<WeaponRegistry>() else {
        return DroppedReferences::Failed;
    };
    for (key, spec) in rewritten {
        registry.insert(key, spec);
    }
    DroppedReferences::Rewritten
}

// Rewrite every melee spec fitted with `name`, writing each file first.
fn drop_from_melee_weapons(
    world: &mut World,
    root: &Path,
    name: &AttachmentName,
) -> DroppedReferences {
    let Some(registry) = world.get_resource::<MeleeWeaponRegistry>() else {
        return DroppedReferences::Nothing;
    };
    let rewritten: Vec<(WeaponName, MeleeWeaponSpec)> = registry
        .iter()
        .filter_map(|(key, spec)| {
            without(&spec.attachments, name).map(|attachments| {
                let mut next = spec.clone();
                next.attachments = attachments;
                (key.clone(), next)
            })
        })
        .collect();
    if rewritten.is_empty() {
        return DroppedReferences::Nothing;
    }
    for (key, spec) in &rewritten {
        if write_melee_weapon_in(root, key, spec).is_err() {
            return DroppedReferences::Failed;
        }
    }
    let Some(mut registry) = world.get_resource_mut::<MeleeWeaponRegistry>() else {
        return DroppedReferences::Failed;
    };
    for (key, spec) in rewritten {
        registry.insert(key, spec);
    }
    DroppedReferences::Rewritten
}
