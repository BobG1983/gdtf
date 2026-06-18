//! Relocated tests for the spawn bundle + the `WeaponStats` borrow-view
//! (GTW-201; moved VERBATIM).

use super::support::*;

/// AC2 — a `WeaponBundle` spawns an entity carrying the full weapon component
/// set + the `Weapon` marker; every component (and the marker) queries back off
/// the spawned entity. Built from arbitrary literals (mechanism, not magnitude).
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

    // The marker + the name + all eleven stat components are present.
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
    // GTW-275: the capacity is no longer a standalone MagazineSize component — the
    // bundle carries the Magazine grouping (size + reload_tu + loaded), spawned full.
    assert!(world.get::<Magazine>(entity).is_some(), "magazine");
    assert!(world.get::<FireMode>(entity).is_some(), "fire_mode");
    assert!(world.get::<Stable>(entity).is_some(), "stable");

    // A spot value round-trips off the spawned entity (distinct literals).
    let Some(damage) = world.get::<WeaponDamage>(entity) else {
        return;
    };
    assert_eq!(**damage, 12i32);

    // GTW-275: the spawned magazine carries the size-30 capacity and is loaded FULL
    // (loaded == size) — the spawn-full path. The reload_tu magnitude is tunable, so
    // it is not pinned; only the size + full-load RELATION is asserted.
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
    assert!(magazine.is_full(), "the spawned magazine is full");
}

/// AC3 — a [`WeaponStats`] borrow-view assembled off a bundle reads the same
/// stats as the bundle's components: the read-shape the §1/§6 readers take is
/// faithful to the stored components. (`WeaponBundle::stats` is the convenience
/// assembler; a query-based system builds the view from its queried components.)
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
    // Both sides deref to the same inner value: the view borrows the components.
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
