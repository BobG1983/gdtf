use super::support::*;

#[test]
fn weapon_bundle_spawns_an_armed_entity() {
    let mut world = World::new();
    let bundle = weapon_bundle(
        0.25,
        1.3,
        0.4,
        7.0,
        profile(12, 5, 3, DamageType::Kinetic),
        handling(30, false),
    );
    let entity = world.spawn(bundle).id();

    assert!(
        world.get::<Weapon>(entity).is_some(),
        "the bundle carries the Weapon marker",
    );
    assert!(
        world.get::<WeaponName>(entity).is_some(),
        "the bundle carries the WeaponName",
    );
    assert!(world.get::<BaseSpread>(entity).is_some(), "base_spread");
    assert!(world.get::<Accuracy>(entity).is_some(), "accuracy");
    assert!(world.get::<Kickback>(entity).is_some(), "kickback");
    assert!(world.get::<FatalBias>(entity).is_some(), "fatal_bias");
    assert!(world.get::<WeaponDamage>(entity).is_some(), "damage");
    assert!(world.get::<WeaponPunch>(entity).is_some(), "punch");
    assert!(world.get::<WeaponShred>(entity).is_some(), "shred");
    assert!(world.get::<DamageType>(entity).is_some(), "damage_type");
    assert!(world.get::<Magazine>(entity).is_some(), "magazine");
    assert!(world.get::<FireMode>(entity).is_some(), "fire_mode");
    assert!(world.get::<Stable>(entity).is_some(), "stable");

    let Some(damage) = world.get::<WeaponDamage>(entity) else {
        return;
    };
    assert_eq!(**damage, 12i32);

    let Some(magazine) = world.get::<Magazine>(entity) else {
        return;
    };
    assert_eq!(
        *magazine.size(),
        30u16,
        "the magazine capacity is the authored 30"
    );
    assert_eq!(
        *magazine.rounds(),
        *magazine.size(),
        "the spawned magazine is loaded FULL (loaded == size)"
    );
    assert!(*magazine.is_full(), "the spawned magazine is full");
}

#[test]
fn weapon_stats_view_borrows_the_components() {
    let bundle = weapon_bundle(
        0.2,
        1.1,
        0.3,
        5.0,
        profile(14, 6, 4, DamageType::Blast),
        handling(24, true),
    );
    let stats = bundle.stats();
    assert_eq!(
        (**stats.base_spread).to_bits(),
        (*bundle.base_spread).to_bits()
    );
    assert_eq!(**stats.damage, *bundle.damage);
    assert_eq!(**stats.punch, *bundle.punch);
    assert_eq!(**stats.shred, *bundle.shred);
    assert_eq!(*stats.damage_type, bundle.damage_type);
    assert_eq!(
        (**stats.fatal_bias).to_bits(),
        (*bundle.fatal_bias).to_bits()
    );
    assert!(**stats.stable);
}
