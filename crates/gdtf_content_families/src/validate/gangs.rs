//! Cross-check gang equipment keys against weapon and armor registries.

use bevy::prelude::{Res, ResMut};
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, FindingFamily, FindingReferrer, FindingTarget,
    ReferenceKeyScheme,
};
use gdtf_battle_sim::{
    armor::ArmorRegistry,
    ganger::GangRegistry,
    weapon::{FISTS_KEY, MeleeWeaponRegistry, WeaponName, WeaponRegistry},
};

/// Record dangling gang member weapon/armor/melee references.
pub fn check_gang_equipment_refs(
    gangs: Res<GangRegistry>,
    weapons: Res<WeaponRegistry>,
    melee_weapons: Res<MeleeWeaponRegistry>,
    armor: Res<ArmorRegistry>,
    mut report: ResMut<ContentIntegrityReport>,
) {
    let fists = WeaponName::new(FISTS_KEY.to_owned());
    for (gang, roster) in gangs.iter() {
        for member in &roster.members {
            let referrer =
                || FindingReferrer::new(format!("gang `{}` member `{}`", **gang, *member.name));
            if weapons.spec(&member.weapon).is_none() {
                report.record(ContentFinding::DanglingRef {
                    referrer: referrer(),
                    target: FindingTarget::new((*member.weapon).clone()),
                    family: FindingFamily::new("WeaponRegistry".to_owned()),
                    scheme: ReferenceKeyScheme::FileStem,
                });
            }
            if armor.spec(&member.armor).is_none() {
                report.record(ContentFinding::DanglingRef {
                    referrer: referrer(),
                    target: FindingTarget::new((*member.armor).clone()),
                    family: FindingFamily::new("ArmorRegistry".to_owned()),
                    scheme: ReferenceKeyScheme::FileStem,
                });
            }
            let melee_key = member.melee_weapon.as_ref().unwrap_or(&fists);
            if melee_weapons.spec(melee_key).is_none() {
                report.record(ContentFinding::DanglingRef {
                    referrer: referrer(),
                    target: FindingTarget::new((**melee_key).clone()),
                    family: FindingFamily::new("MeleeWeaponRegistry".to_owned()),
                    scheme: ReferenceKeyScheme::FileStem,
                });
            }
        }
    }
}
