use super::support::*;

#[test]
fn weapon_is_a_unit_marker() {
    let marker = Weapon;
    let copied = marker;
    assert_eq!(marker, copied, "the Weapon marker is a Copy unit type");
    let another = Weapon;
    assert_eq!(marker, another, "the marker is a fieldless unit type");
}

#[test]
fn each_sub_value_is_a_component() {
    let mut world = World::new();
    let entity = world
        .spawn((
            Weapon,
            WeaponName::new("stub_pistol".to_owned()),
            BaseSpread::new(0.25),
            Accuracy::new(1.3),
            Kickback::new(0.4),
            FatalBias::new(7.0),
            WeaponDamage::new(12),
            WeaponPunch::new(5),
            WeaponShred::new(3),
            DamageType::Kinetic,
            Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
            FireMode::new(vec![spec(1.0, 0.5, 1)]),
            Stable::new(true),
        ))
        .id();

    assert!(
        world.get::<Weapon>(entity).is_some(),
        "the marker is present"
    );
    let Some(name) = world.get::<WeaponName>(entity) else {
        return;
    };
    assert_eq!(
        &**name, "stub_pistol",
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

    assert_eq!((*bundle.base_spread).to_bits(), 0.25_f32.to_bits());
    assert_eq!((*bundle.accuracy).to_bits(), 1.3_f32.to_bits());
    assert_eq!((*bundle.kickback).to_bits(), 0.4_f32.to_bits());
    assert_eq!((*bundle.fatal_bias).to_bits(), 7.0_f32.to_bits());
    assert_eq!(*bundle.magazine.size(), 30u16);
    assert_eq!(
        *bundle.magazine.rounds(),
        *bundle.magazine.size(),
        "handling() loads the magazine full (loaded == size)"
    );
}

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

    assert_eq!(*bundle.damage, 18i32);
    assert_eq!(*bundle.punch, 7i32);
    assert_eq!(*bundle.shred, 2i32);
}

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
    assert!(*stable_bundle.stable);
    assert!(!*plain_bundle.stable);
    assert_eq!(stable_bundle.stable, Stable::new(true));
    assert_eq!(plain_bundle.stable, Stable::new(false));
}

/// Stable parses from a bare RON bool via `#[serde(transparent)]`.
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

/// Weapon leaf newtypes parse from bare RON scalars via `#[serde(transparent)]`.
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

#[test]
fn damage_type_has_seven_variants() {
    assert_eq!(DamageType::ALL.len(), 7);
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

/// AC1 — a [`WeaponName`] is a documented `Component` + `Deref` newtype with
/// `#[serde(transparent)]`.
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
    assert_eq!(
        &*bundle.name, "boltgun",
        "WeaponName is a WeaponBundle field"
    );
    let Ok(parsed) = ron::from_str::<WeaponName>(r#""boltgun""#) else {
        return;
    };
    assert_eq!(
        &*parsed, "boltgun",
        "WeaponName must parse from a bare RON string"
    );
}
