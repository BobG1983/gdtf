//! The shared weapon/armor test specs + registries + the `(cell, level)` key
//! helper — the consolidated single source the sim's own unit tests AND the
//! downstream crates' tests both reach for, replacing the per-module duplicates
//! that lived in `situation/test/support.rs` and `battle/test/support.rs`.
//!
//! Every magnitude here is ARBITRARY test data (NOT shipped tuning), so a test
//! that reads a resolved value proves the resolution path is faithful without
//! ever pinning a balance number (the brittle-test rule).

use crate::{
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorName, ArmorPiece, ArmorProtection,
        ArmorRegistry, ArmorSpec, ArmorType,
    },
    magazine::{Magazine, ReloadTu},
    metric::{Cell, CellLevel, Level},
    weapon::{
        Accuracy, BaseSpread, DamageType, FatalBias, FireMode, FireModeSpec, Kickback,
        MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, Stable, WeaponDamage,
        WeaponName, WeaponPunch, WeaponRegistry, WeaponShred, WeaponSpec,
    },
};

/// The weapon KEY every test ganger references — present in
/// [`test_weapon_registry`]. Every fixture builds gangers that resolve this key
/// against the test weapon registry, so a setup arms each one.
pub const TEST_WEAPON_KEY: &str = "test-weapon";

/// The armor KEY every test ganger references — present in
/// [`test_armor_registry`] (the armor mirror of [`TEST_WEAPON_KEY`]).
pub const TEST_ARMOR_KEY: &str = "test-armor";

/// Build a `(cell, level)` key from raw coordinates — the terse fixture helper
/// the situation builders + the per-crate tests use to place gangers and cover.
#[must_use]
pub fn key(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// An arbitrary [`WeaponSpec`] (NOT shipped magnitudes — mechanism only) carrying
/// a single-shot [`FireMode`] whose one mode is [`ModeKind::Single`], so a
/// resolved bundle proves the [`Weapon`](crate::weapon::Weapon) marker +
/// [`FireMode`] landed. The suit the [`TEST_WEAPON_KEY`] resolves to in
/// [`test_weapon_registry`].
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
        magazine:    Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        fire_mode:   FireMode::new(vec![FireModeSpec::new(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.5),
            ModeShots::new(1),
        )]),
        stable:      Stable::new(false),
    }
}

/// A [`WeaponRegistry`] holding the one [`TEST_WEAPON_KEY`] weapon — the
/// test-built registry a setup resolves each ganger's `weapon` key against (no
/// `AssetServer`). Stands in for the app's `Load`-built registry.
#[must_use]
pub fn test_weapon_registry() -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(TEST_WEAPON_KEY.to_owned()),
        test_weapon_spec(),
    )])
}

/// An arbitrary armor SPEC — distinct per-part magnitudes (NOT shipped tuning) so a
/// registry-resolved seed copy is provably faithful, never asserting a magnitude.
/// `base` shifts every per-part stat up by a fixed offset, so a test can pick a
/// known regime (e.g. `base 0` = paper-thin).
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

/// The default test armor SPEC — [`arbitrary_armor`] at `base 1`, the suit the
/// [`TEST_ARMOR_KEY`] resolves to in [`test_armor_registry`].
#[must_use]
pub const fn test_armor_spec() -> ArmorSpec {
    arbitrary_armor(1)
}

/// An [`ArmorRegistry`] holding the one [`TEST_ARMOR_KEY`] armor suit — the
/// test-built armor registry a setup resolves each ganger's `armor` key against
/// (no `AssetServer`), the armor mirror of [`test_weapon_registry`].
#[must_use]
pub fn test_armor_registry() -> ArmorRegistry {
    ArmorRegistry::new([(ArmorName::new(TEST_ARMOR_KEY.to_owned()), test_armor_spec())])
}
