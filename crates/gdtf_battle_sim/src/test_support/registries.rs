//! Test weapon, armor, and terrain registries.

use crate::{
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorName, ArmorPiece, ArmorProtection,
        ArmorRegistry, ArmorSpec, ArmorType,
    },
    equipment::attachments::{FittedAttachments, WeaponSlots},
    magazine::{Magazine, ReloadTu},
    metric::{Cell, CellLevel, Level},
    weapon::{
        Accuracy, AmmoType, BaseSpread, DamageType, FISTS_KEY, FatalBias, FightMode, FightModeKind,
        FightModeSpec, FireMode, FireModeSpec, Handedness, Kickback, MagazineSize,
        MeleeWeaponRegistry, MeleeWeaponSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
        Reach, Shove, Stable, Strikes, TrajectoryStyle, TuCost, WeaponDamage, WeaponName,
        WeaponPunch, WeaponRegistry, WeaponShred, WeaponSpec,
    },
};

/// Key for the shared test weapon.
pub const TEST_WEAPON_KEY: &str = "test-weapon";

/// Key for the shared test armor.
pub const TEST_ARMOR_KEY: &str = "test-armor";

/// Key for the shared test melee weapon.
pub const TEST_MELEE_WEAPON_KEY: &str = "test-melee";

/// Key for the weapon bolted to the test emplacement.
pub const TEST_MOUNTED_WEAPON_KEY: &str = "test-mounted";

/// Cell at `(x, y, level)`.
#[must_use]
pub fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// Basic kinetic sidearm spec.
#[must_use]
pub fn test_weapon_spec() -> WeaponSpec {
    WeaponSpec {
        base_spread: BaseSpread::new(0.25),
        accuracy:    Accuracy::new(1.0),
        kickback:    Kickback::new(0.4),
        fatal_bias:  FatalBias::new(7.0),
        damage:      WeaponDamage::new(12),
        punch:       WeaponPunch::new(5),
        shred:       WeaponShred::new(3),
        damage_type: DamageType::Kinetic,
        accepts:     AmmoType::Slug,
        magazine:    Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        fire_mode:   FireMode::new(vec![FireModeSpec::new(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.5),
            ModeShots::new(1),
        )]),
        stable:      Stable::new(false),
        shove:       Shove::new(false),
        handedness:  Handedness::OneHanded,
        trajectory:  TrajectoryStyle::Straight,
        slots:       WeaponSlots::default(),
        attachments: FittedAttachments::default(),
        dot:         None,
        on_death:    None,
    }
}

/// Registry with the test weapon and the test mounted weapon.
#[must_use]
pub fn test_weapon_registry() -> WeaponRegistry {
    WeaponRegistry::new([
        (
            WeaponName::new(TEST_WEAPON_KEY.to_owned()),
            test_weapon_spec(),
        ),
        (
            WeaponName::new(TEST_MOUNTED_WEAPON_KEY.to_owned()),
            test_weapon_spec(),
        ),
    ])
}

/// Basic melee weapon spec.
#[must_use]
pub fn test_melee_weapon_spec() -> MeleeWeaponSpec {
    MeleeWeaponSpec {
        damage:      WeaponDamage::new(9),
        punch:       WeaponPunch::new(3),
        shred:       WeaponShred::new(2),
        damage_type: DamageType::Rend,
        fatal_bias:  FatalBias::new(4.0),
        handedness:  Handedness::OneHanded,
        reach:       Reach::new(1),
        fight_mode:  FightMode::new(vec![FightModeSpec::new(
            FightModeKind::Swing,
            TuCost::new(20),
            Strikes::new(1),
        )]),
        shove:       Shove::new(false),
        slots:       WeaponSlots::default(),
        attachments: FittedAttachments::default(),
    }
}

/// Registry with fists and the test melee weapon.
#[must_use]
pub fn test_melee_weapon_registry() -> MeleeWeaponRegistry {
    MeleeWeaponRegistry::new([
        (
            WeaponName::new(FISTS_KEY.to_owned()),
            test_melee_weapon_spec(),
        ),
        (
            WeaponName::new(TEST_MELEE_WEAPON_KEY.to_owned()),
            test_melee_weapon_spec(),
        ),
    ])
}

/// Uniform armor scaled from a base value.
#[must_use]
pub const fn arbitrary_armor(base: i32) -> ArmorSpec {
    ArmorSpec::uniform(ArmorPiece::new(
        ArmorFloor::new(base),
        ArmorProtection::new(base + 1),
        ArmorIntegrity::new(base + 2),
        ArmorHardness::new(base + 3),
        ArmorType::DEFAULT,
    ))
}

/// Default test armor.
#[must_use]
pub const fn test_armor_spec() -> ArmorSpec {
    arbitrary_armor(1)
}

/// Registry containing only the test armor.
#[must_use]
pub fn test_armor_registry() -> ArmorRegistry {
    ArmorRegistry::new([(ArmorName::new(TEST_ARMOR_KEY.to_owned()), test_armor_spec())])
}
