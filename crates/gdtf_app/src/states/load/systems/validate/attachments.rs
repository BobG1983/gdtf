//! GTW-582 C3(b): every **weapon's attachment keys** — the edge the setup-time
//! resolution (`resolve_pending_attachments`) SILENTLY skips on a missing key
//! (the fail-safe stays: a battle must never abort over a missing scope). This
//! check makes that silent drop a reported dangling reference at `Load`.

use bevy::prelude::{Res, ResMut};
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, FindingFamily, FindingReferrer, FindingTarget,
    ReferenceKeyScheme,
};
use gdtf_battle_sim::{
    equipment::attachments::AttachmentRegistry,
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};

/// `Check`: every RANGED and MELEE weapon spec's authored `attachments` keys
/// resolve in the [`AttachmentRegistry`] (GTW-549 item files, file-stem keyed).
/// The runtime skip semantics are untouched — this reports the mistake where it
/// is fixable, at `Load`.
///
/// Plain `Res` params are safe here: the `Check` set's window condition
/// verified them present (bevy-traps #1, guarded once at the set).
pub(super) fn check_weapon_attachment_refs(
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
