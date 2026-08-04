//! Cross-check weapon attachment keys against the attachment registry.

use bevy::prelude::{Res, ResMut};
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, FindingFamily, FindingReferrer, FindingTarget,
    ReferenceKeyScheme,
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
        .map(|(name, spec)| ("weapon", name, &spec.attachments));
    let melee = melee_weapons
        .iter()
        .map(|(name, spec)| ("melee weapon", name, &spec.attachments));
    for (kind, name, keys) in ranged.chain(melee) {
        for key in keys {
            if attachments.spec(key).is_none() {
                report.record(ContentFinding::DanglingRef {
                    referrer: FindingReferrer::new(format!("{kind} `{}` attachments", **name)),
                    target:   FindingTarget::new((**key).clone()),
                    family:   FindingFamily::new("AttachmentRegistry".to_owned()),
                    scheme:   ReferenceKeyScheme::FileStem,
                });
            }
        }
    }
}
