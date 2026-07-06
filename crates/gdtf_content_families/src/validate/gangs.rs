//! GTW-582: the **gang rosters'** outbound reference edges — every member's
//! weapon / armor / melee-weapon keys (all file-stem keyed), including the
//! implicit `fists` default a melee-less member resolves to at setup. These are
//! ALSO still guarded abort-first at battle-request time
//! ([`BattleSetupError`](gdtf_battle_sim::situation::BattleSetupError), C3(a));
//! this check surfaces the same mistakes at `Load`, for EVERY roster (not just
//! the fielded one).

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

/// `Check`: every roster member's equipment keys resolve — `weapon` in the
/// [`WeaponRegistry`], `armor` in the [`ArmorRegistry`], `melee_weapon` in the
/// [`MeleeWeaponRegistry`] (or, when the member authors none, the shipped
/// [`FISTS_KEY`] default it will resolve to at setup — a missing `fists` file
/// dangles EVERY melee-less member, so it is reported once per such member's
/// gang context).
///
/// Plain `Res` params by contract: the registering HOST's `Check`-set window
/// condition must have verified them present (`bevy-traps.md` #1, guarded once
/// at the host's set — see the [module doc](super)).
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
                    target:   FindingTarget::new((*member.weapon).clone()),
                    family:   FindingFamily::new("WeaponRegistry".to_owned()),
                    scheme:   ReferenceKeyScheme::FileStem,
                });
            }
            if armor.spec(&member.armor).is_none() {
                report.record(ContentFinding::DanglingRef {
                    referrer: referrer(),
                    target:   FindingTarget::new((*member.armor).clone()),
                    family:   FindingFamily::new("ArmorRegistry".to_owned()),
                    scheme:   ReferenceKeyScheme::FileStem,
                });
            }
            // The melee key: the authored one, else the implicit `fists`
            // default `setup_battle` will substitute (GTW-505) — a dangling
            // default is as battle-aborting as a dangling authored key.
            let melee_key = member.melee_weapon.as_ref().unwrap_or(&fists);
            if melee_weapons.spec(melee_key).is_none() {
                report.record(ContentFinding::DanglingRef {
                    referrer: referrer(),
                    target:   FindingTarget::new((**melee_key).clone()),
                    family:   FindingFamily::new("MeleeWeaponRegistry".to_owned()),
                    scheme:   ReferenceKeyScheme::FileStem,
                });
            }
        }
    }
}
