use bevy::scene::{Scene, SceneList, bsn, bsn_list, template_value};

use crate::weapon::{
    Accuracy, BaseSpread, FatalBias, FightMode, FireMode, Kickback, MeleeWeapon, MeleeWeaponBundle,
    PendingAttachments, Reach, Shove, Stable, Weapon, WeaponBundle, WeaponDamage, WeaponName,
    WeaponPunch, WeaponShred, WeaponSpawnSiblings,
};

pub(super) fn wielded_weapon_scenes(
    weapon: &WeaponBundle,
    siblings: WeaponSpawnSiblings,
    pending: PendingAttachments,
) -> impl SceneList {
    bsn_list! { wielded_weapon_scene(weapon, siblings, pending) }
}

fn wielded_weapon_scene(
    weapon: &WeaponBundle,
    siblings: WeaponSpawnSiblings,
    pending: PendingAttachments,
) -> impl Scene {
    let weapon_name = (*weapon.name).clone();
    let base_spread = *weapon.base_spread;
    let accuracy = *weapon.accuracy;
    let kickback = *weapon.kickback;
    let fatal_bias = *weapon.fatal_bias;
    let weapon_damage = *weapon.damage;
    let weapon_punch = *weapon.punch;
    let weapon_shred = *weapon.shred;
    let fire_mode = (*weapon.fire_mode).clone();
    let stable = *weapon.stable;
    let shove = *weapon.shove;
    let damage_type = weapon.damage_type;
    let magazine = weapon.magazine;
    let handedness = weapon.handedness;
    let trajectory = weapon.trajectory;
    let dot = siblings.dot().map(template_value);
    let on_death = siblings.on_death().cloned().map(template_value);
    let pending = template_value(pending);
    (
        bsn! {
            Weapon
            WeaponName::new(weapon_name)
            BaseSpread::new(base_spread)
            Accuracy::new(accuracy)
            Kickback::new(kickback)
            FatalBias::new(fatal_bias)
            WeaponDamage::new(weapon_damage)
            WeaponPunch::new(weapon_punch)
            WeaponShred::new(weapon_shred)
            FireMode::new(fire_mode)
            Stable::new(stable)
            Shove::new(shove)
        },
        template_value(damage_type),
        template_value(magazine),
        template_value(handedness),
        template_value(trajectory),
        dot,
        on_death,
        pending,
    )
}

pub(super) fn wielded_melee_weapon_scenes(
    weapon: &MeleeWeaponBundle,
    pending: PendingAttachments,
) -> impl SceneList {
    bsn_list! { wielded_melee_weapon_scene(weapon, pending) }
}

fn wielded_melee_weapon_scene(
    weapon: &MeleeWeaponBundle,
    pending: PendingAttachments,
) -> impl Scene {
    let weapon_name = (*weapon.name).clone();
    let weapon_damage = *weapon.damage;
    let weapon_punch = *weapon.punch;
    let weapon_shred = *weapon.shred;
    let fatal_bias = *weapon.fatal_bias;
    let reach = *weapon.reach;
    let fight_mode = (*weapon.fight_mode).clone();
    let shove = *weapon.shove;
    let damage_type = weapon.damage_type;
    let handedness = weapon.handedness;
    let pending = template_value(pending);
    (
        bsn! {
            MeleeWeapon
            WeaponName::new(weapon_name)
            WeaponDamage::new(weapon_damage)
            WeaponPunch::new(weapon_punch)
            WeaponShred::new(weapon_shred)
            FatalBias::new(fatal_bias)
            Reach::new(reach)
            FightMode::new(fight_mode)
            Shove::new(shove)
        },
        template_value(damage_type),
        template_value(handedness),
        pending,
    )
}
