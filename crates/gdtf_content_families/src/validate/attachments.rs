//! Cross-check weapon attachment keys against the attachment registry.

use bevy::prelude::{Res, ResMut};
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, ContentMemberKey, FindingFamily, FindingReferrer,
    FindingTarget, ReferenceField, ReferenceKeyScheme, ReferringRecord,
};
use gdtf_battle_sim::{
    equipment::attachments::AttachmentRegistry,
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};

/// Record dangling weapon/melee attachment references.
pub fn check_weapon_attachment_refs(
    weapons: Res<WeaponRegistry>,
    melee_weapons: Res<MeleeWeaponRegistry>,
    attachments: Res<AttachmentRegistry>,
    mut report: ResMut<ContentIntegrityReport>,
) {
    let ranged = weapons
        .iter()
        .map(|(name, spec)| ("weapon", "WeaponRegistry", name, &spec.attachments));
    let melee = melee_weapons.iter().map(|(name, spec)| {
        (
            "melee weapon",
            "MeleeWeaponRegistry",
            name,
            &spec.attachments,
        )
    });
    for (kind, registry, name, keys) in ranged.chain(melee) {
        for key in keys.iter() {
            if attachments.spec(key).is_none() {
                report.record(ContentFinding::DanglingRef {
                    referrer:         FindingReferrer::new(format!(
                        "{kind} `{}` attachments",
                        **name
                    )),
                    referring_record: ReferringRecord::new(
                        FindingFamily::new(registry.to_owned()),
                        ContentMemberKey::new((**name).clone()),
                        ReferenceField::new("attachments".to_owned()),
                    ),
                    target:           FindingTarget::new((**key).clone()),
                    family:           FindingFamily::new("AttachmentRegistry".to_owned()),
                    scheme:           ReferenceKeyScheme::FileStem,
                });
            }
        }
    }
}
