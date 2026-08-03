use bevy::prelude::{Entity, World};

use super::support::{
    ENEMY, PLAYER, active_of, brain_app, drain_fires, ground, place_occupant, spawn_combatant,
    tu_of,
};
use crate::{
    ganger::Direction,
    magazine::{LoadedRounds, Magazine, ReloadTu},
    metric::Cell,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        Handedness, HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots,
        ModeTuPercent, MountedWeapon, Shove, Stable, WeaponBundle, WeaponDamage, WeaponName,
        WeaponPunch, WeaponShred, WieldedBy,
    },
};

const FRAME_CAP: usize = 80;

fn man_loaded_mount(world: &mut World, ganger: Entity, ammo: u16) {
    let mode = FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    );
    let bundle = WeaponBundle::new(
        WeaponName::new("test-mount".to_owned()),
        BaseSpread::new(0.05),
        Accuracy::new(2.0),
        Kickback::new(0.2),
        FatalBias::new(0.0),
        DamageProfile::new(
            WeaponDamage::new(40),
            WeaponPunch::new(20),
            WeaponShred::new(10),
            DamageType::Kinetic,
        ),
        HandlingProfile::new(
            Magazine::new(
                LoadedRounds::new(ammo),
                MagazineSize::new(30),
                ReloadTu::new(12),
            ),
            FireMode::new(vec![mode]),
            Stable::new(true),
            Shove::new(false),
            Handedness::OneHanded,
        ),
    );
    world.spawn((WieldedBy::new(ganger), bundle, MountedWeapon));
}

#[test]
fn ai_mounted_enemy_engages_on_the_mount() {
    let mut app = brain_app();
    let player_at = ground(8, 5);
    let enemy = spawn_combatant(
        app.world_mut(),
        ground(2, 5),
        ENEMY,
        Direction::East,
        100,
        0,
    );
    man_loaded_mount(app.world_mut(), enemy, 6);
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 2);
    place_occupant(&mut app, player_at, player);

    let mut fires = Vec::new();
    for _ in 0..FRAME_CAP {
        app.update();
        fires.extend(drain_fires(&mut app));
        if active_of(&app) == PLAYER {
            break;
        }
    }

    assert!(
        fires.iter().any(|f| f.shooter == enemy
            && f.target_cell == Cell::new(8, 5)
            && *f.target_level == 0),
        "a mounted AI enemy with an empty carried gun must engage on the loaded mount at the \
         player's (8,5,0) cell: {fires:?}",
    );
    assert!(
        tu_of(&app, enemy) < 100,
        "the enemy spent TU engaging on the mount: {}",
        tu_of(&app, enemy),
    );
}
