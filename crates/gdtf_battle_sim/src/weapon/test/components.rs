//! Relocated tests for the per-stat weapon components — the marker, the leaf
//! newtypes, `DamageType`, and `WeaponName` (GTW-201; moved VERBATIM).

use super::support::*;

/// AC1 — `Weapon` is a **unit MARKER** (no data fields): it constructs from the
/// unit value, is `Copy`, and compares by value. A struct with a field could not
/// be built this way.
#[test]
fn weapon_is_a_unit_marker() {
    let marker = Weapon;
    let copied = marker; // Copy, not a move
    assert_eq!(marker, copied, "the Weapon marker is a Copy unit type");
    // A unit marker carries no data — two independently-built values are equal.
    let another = Weapon;
    assert_eq!(marker, another, "the marker is a fieldless unit type");
}

/// AC1 — each weapon sub-value derives `Component`: insert each onto a fresh
/// `World` entity and query it back. If any newtype lacked the `Component`
/// derive this would not compile, so the test IS the proof the derive is present
/// (and the values round-trip through the ECS).
#[test]
fn each_sub_value_is_a_component() {
    let mut world = World::new();
    let entity = world
        .spawn((
            Weapon,
            WeaponName::new("autogun".to_owned()),
            BaseSpread::new(0.25),
            Accuracy::new(1.3),
            Kickback::new(0.4),
            FatalBias::new(7.0),
            WeaponDamage::new(12),
            WeaponPunch::new(5),
            WeaponShred::new(3),
            DamageType::Kinetic,
            MagazineSize::new(30),
            FireMode::new(vec![spec(1.0, 0.5, 1)]),
            Stable::new(true),
        ))
        .id();

    // Every component queries back off the entity (mechanism, not magnitude;
    // distinct arbitrary literals so a mix-up would surface).
    assert!(
        world.get::<Weapon>(entity).is_some(),
        "the marker is present"
    );
    let Some(name) = world.get::<WeaponName>(entity) else {
        return;
    };
    assert_eq!(
        &**name, "autogun",
        "the weapon name round-trips through the ECS"
    );
    let Some(base) = world.get::<BaseSpread>(entity) else {
        return;
    };
    assert_eq!((**base).to_bits(), 0.25_f32.to_bits());
    let Some(damage) = world.get::<WeaponDamage>(entity) else {
        return;
    };
    assert_eq!(**damage, 12i32);
    let Some(damage_type) = world.get::<DamageType>(entity) else {
        return;
    };
    assert_eq!(*damage_type, DamageType::Kinetic);
    let Some(stable) = world.get::<Stable>(entity) else {
        return;
    };
    assert!(**stable, "the stable tag round-trips through the ECS");
}

/// AC1 — each weapon-number leaf derefs to its inner value (the no-bare-types
/// mechanism), read directly off the component newtypes. Built from
/// **arbitrary** literals (never pinned magnitudes).
#[test]
fn weapon_leaves_deref_to_inner() {
    let bundle = weapon_bundle(
        0.25,
        1.3,
        0.4,
        7.0,
        profile(12, 5, 3, DamageType::Kinetic),
        handling(30, false),
    );

    // Each f32 weapon number: deref reaches the inner f32 (bit-exact arbitrary
    // value — exactly representable literals, so this is an integer equality,
    // no float_cmp lint).
    assert_eq!((*bundle.base_spread).to_bits(), 0.25_f32.to_bits());
    assert_eq!((*bundle.accuracy).to_bits(), 1.3_f32.to_bits());
    assert_eq!((*bundle.kickback).to_bits(), 0.4_f32.to_bits());
    assert_eq!((*bundle.fatal_bias).to_bits(), 7.0_f32.to_bits());
    // The u16 weapon number: deref reaches the inner u16.
    assert_eq!(*bundle.magazine_size, 30u16);
}

/// AC1 — the `damage` / `punch` / `shred` components are distinct newtypes that
/// deref to their inner `i32`. Built from **arbitrary** literals (no pinned
/// magnitude); reads each leaf back through `Deref`.
#[test]
fn weapon_damage_leaves_deref_to_inner() {
    let bundle = weapon_bundle(
        0.1,
        1.0,
        0.2,
        4.0,
        profile(18, 7, 2, DamageType::Plasma),
        handling(12, false),
    );

    // Each i32 damage number derefs to its inner value (distinct arbitrary
    // literals so a field swap would be caught — mechanism, not magnitude).
    assert_eq!(*bundle.damage, 18i32);
    assert_eq!(*bundle.punch, 7i32);
    assert_eq!(*bundle.shred, 2i32);
}

/// AC3 — the `DamageType` component round-trips off the bundle (mechanism, not
/// magnitude). Pairs with `armor_piece_carries_armor_type` in `armor.rs`.
#[test]
fn weapon_carries_damage_type_round_trip() {
    let bundle = weapon_bundle(
        0.1,
        1.0,
        0.2,
        4.0,
        profile(1, 1, 1, DamageType::Rend),
        handling(1, false),
    );
    assert_eq!(bundle.damage_type, DamageType::Rend);
}

/// AC1 (GTW-199) — the `Stable` component round-trips and derefs to its inner
/// `bool`. A stable and a non-stable bundle are built so a field swap (or a
/// default) would be caught — mechanism, not a pinned balance value.
#[test]
fn weapon_carries_stable_tag_round_trip() {
    let stable_bundle = weapon_bundle(
        0.1,
        1.0,
        0.2,
        4.0,
        profile(1, 1, 1, DamageType::Kinetic),
        handling(1, true),
    );
    let plain_bundle = weapon_bundle(
        0.1,
        1.0,
        0.2,
        4.0,
        profile(1, 1, 1, DamageType::Kinetic),
        handling(1, false),
    );
    // The tag round-trips through the newtype's Deref to the inner bool.
    assert!(*stable_bundle.stable);
    assert!(!*plain_bundle.stable);
    assert_eq!(stable_bundle.stable, Stable::new(true));
    assert_eq!(plain_bundle.stable, Stable::new(false));
}

/// AC1 (GTW-199) — the `Stable` tag deserializes from a bare RON boolean
/// (`#[serde(transparent)]`): a `true` fragment parses to a stable tag, a
/// `false` fragment to a non-stable one (value read back, not a pinned score).
#[test]
fn stable_parses_from_bare_ron_bool() {
    let yes = ron::from_str::<Stable>("true");
    let no = ron::from_str::<Stable>("false");
    assert!(
        yes.is_ok(),
        "Stable must parse from a bare RON `true`: {yes:?}"
    );
    assert!(
        no.is_ok(),
        "Stable must parse from a bare RON `false`: {no:?}"
    );
    let (Ok(yes), Ok(no)) = (yes, no) else {
        return;
    };
    assert!(*yes);
    assert!(!*no);
}

/// C5 — each weapon-number leaf still round-trips as a bare RON scalar
/// (`#[serde(transparent)]`) even now it is a `Component`: parse a bare fragment
/// for each transparent leaf (value-agnostic — only that it parses into the
/// type).
#[test]
fn weapon_leaves_parse_from_bare_ron_scalars() {
    assert!(ron::from_str::<BaseSpread>("0.2").is_ok(), "base_spread");
    assert!(ron::from_str::<Accuracy>("1.1").is_ok(), "accuracy");
    assert!(ron::from_str::<Kickback>("0.3").is_ok(), "kickback");
    assert!(ron::from_str::<FatalBias>("5.0").is_ok(), "fatal_bias");
    assert!(ron::from_str::<WeaponDamage>("14").is_ok(), "damage");
    assert!(ron::from_str::<WeaponPunch>("6").is_ok(), "punch");
    assert!(ron::from_str::<WeaponShred>("4").is_ok(), "shred");
    assert!(ron::from_str::<MagazineSize>("24").is_ok(), "magazine_size");
    assert!(
        ron::from_str::<DamageType>("Kinetic").is_ok(),
        "damage_type"
    );
}

/// AC2 (one half) — `DamageType` has exactly 7 variants, in wheel-node order.
/// The mirror-parity half lives in `armor.rs` (it needs both enums).
#[test]
fn damage_type_has_seven_variants() {
    assert_eq!(DamageType::ALL.len(), 7);
    // Node order is pinned (matchup.md Table 1) — the parity test in armor.rs
    // relies on it.
    assert_eq!(
        DamageType::ALL,
        [
            DamageType::Shock,
            DamageType::Blast,
            DamageType::Chem,
            DamageType::Kinetic,
            DamageType::Plasma,
            DamageType::Rend,
            DamageType::Las,
        ]
    );
}

/// AC1 — a [`WeaponName`] is a documented `#[derive(Component, Deref,
/// Deserialize)]` newtype over `String`: it is a [`WeaponBundle`] field (read back
/// off a constructed bundle) and parses from a bare RON string
/// (`#[serde(transparent)]`).
#[test]
fn weapon_name_is_a_bundle_field_and_parses_from_ron() {
    let bundle = WeaponBundle::new(
        WeaponName::new("boltgun".to_owned()),
        BaseSpread::new(0.2),
        Accuracy::new(1.0),
        Kickback::new(0.1),
        FatalBias::new(3.0),
        profile(10, 4, 2, DamageType::Kinetic),
        handling(20, false),
    );
    // The name is a bundle field, read back through its Deref to the inner String.
    assert_eq!(
        &*bundle.name, "boltgun",
        "WeaponName is a WeaponBundle field"
    );
    // And it deserializes from a bare RON string.
    let Ok(parsed) = ron::from_str::<WeaponName>(r#""boltgun""#) else {
        return;
    };
    assert_eq!(
        &*parsed, "boltgun",
        "WeaponName must parse from a bare RON string"
    );
}
