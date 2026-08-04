//! Test weapon, armor, and terrain registries.

use crate::{
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorName, ArmorPiece, ArmorProtection,
        ArmorRegistry, ArmorSpec, ArmorType,
    },
    cover::{CoverHp, HeightBand},
    equipment::attachments::WeaponSlots,
    magazine::{Magazine, ReloadTu},
    metric::{Cell, CellLevel, Level},
    slab::SlabHp,
    terrain::{
        def::{
            TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainTag,
        },
        piece::{FootfallSound, TerrainGraphicKey},
    },
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
        attachments: Vec::new(),
        dot:         None,
        on_death:    None,
    }
}

/// Registry containing only the test weapon.
#[must_use]
pub fn test_weapon_registry() -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(TEST_WEAPON_KEY.to_owned()),
        test_weapon_spec(),
    )])
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
        attachments: Vec::new(),
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

/// Registry of all named test terrain pieces.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "a flat table of named TerrainDef literals (one per test piece UUID) — the length \
              is the literal count, not branching complexity; splitting it into per-def helpers \
              would only scatter the single shared registry definition with no clarity gain"
)]
pub fn test_terrain_registry() -> TerrainDefRegistry {
    use super::situation::test_pieces;

    let display = |name: &str| TerrainDisplayName::new(name.to_owned());
    let graphic = |role: &str| TerrainGraphicKey::new(role.to_owned());

    TerrainDefRegistry::new([
        (
            test_pieces::WALL,
            TerrainDef {
                key:            test_pieces::WALL,
                display_name:   display("Test Wall"),
                sim_kind:       TerrainSimKind::Wall {
                    hp:               CoverHp::new(120),
                    armor_protection: ArmorProtection::new(8),
                    armor_hardness:   ArmorHardness::new(4),
                    height_band:      HeightBand::High,
                },
                presenter_kind: TerrainPresenterKind::Wall {
                    graphic_name: graphic("wall"),
                },
                tags:           Vec::new(),
                on_death:       None,
                blocks_pathing: None,
                blocks_los:     None,
            },
        ),
        (
            test_pieces::SLAB,
            TerrainDef {
                key:            test_pieces::SLAB,
                display_name:   display("Test Slab"),
                sim_kind:       TerrainSimKind::Slab {
                    hp:               SlabHp::new(120),
                    armor_protection: ArmorProtection::new(4),
                    armor_hardness:   ArmorHardness::new(2),
                },
                presenter_kind: TerrainPresenterKind::Slab {
                    graphic_name: graphic("slab"),
                    footfall:     Some(FootfallSound::new("test-step".to_owned())),
                },
                tags:           Vec::new(),
                on_death:       None,
                blocks_pathing: None,
                blocks_los:     None,
            },
        ),
        (
            test_pieces::COVER,
            TerrainDef {
                key:            test_pieces::COVER,
                display_name:   display("Test Cover"),
                sim_kind:       TerrainSimKind::Cover {
                    hp:               CoverHp::new(30),
                    armor_protection: ArmorProtection::new(2),
                    armor_hardness:   ArmorHardness::new(1),
                    height_band:      HeightBand::Low,
                },
                presenter_kind: TerrainPresenterKind::Cover {
                    graphic_name: graphic("cover"),
                },
                tags:           Vec::new(),
                on_death:       None,
                blocks_pathing: None,
                blocks_los:     None,
            },
        ),
        (
            test_pieces::FLOOR,
            TerrainDef {
                key:            test_pieces::FLOOR,
                display_name:   display("Test Floor"),
                sim_kind:       TerrainSimKind::Slab {
                    hp:               SlabHp::new(60),
                    armor_protection: ArmorProtection::new(1),
                    armor_hardness:   ArmorHardness::new(0),
                },
                presenter_kind: TerrainPresenterKind::Slab {
                    graphic_name: graphic("floor"),
                    footfall:     None,
                },
                tags:           Vec::new(),
                on_death:       None,
                blocks_pathing: None,
                blocks_los:     None,
            },
        ),
        (
            test_pieces::VISION_SLAB,
            TerrainDef {
                key:            test_pieces::VISION_SLAB,
                display_name:   display("Test Vision Slab"),
                sim_kind:       TerrainSimKind::Slab {
                    hp:               SlabHp::new(120),
                    armor_protection: ArmorProtection::new(4),
                    armor_hardness:   ArmorHardness::new(2),
                },
                presenter_kind: TerrainPresenterKind::Slab {
                    graphic_name: graphic("vision-slab"),
                    footfall:     None,
                },
                tags:           vec![TerrainTag::BlocksVision],
                on_death:       None,
                blocks_pathing: None,
                blocks_los:     None,
            },
        ),
        (
            test_pieces::PATH_SLAB,
            TerrainDef {
                key:            test_pieces::PATH_SLAB,
                display_name:   display("Test Path Slab"),
                sim_kind:       TerrainSimKind::Slab {
                    hp:               SlabHp::new(120),
                    armor_protection: ArmorProtection::new(4),
                    armor_hardness:   ArmorHardness::new(2),
                },
                presenter_kind: TerrainPresenterKind::Slab {
                    graphic_name: graphic("path-slab"),
                    footfall:     None,
                },
                tags:           vec![TerrainTag::BlocksPathfinding],
                on_death:       None,
                blocks_pathing: None,
                blocks_los:     None,
            },
        ),
        (
            test_pieces::LOW_VISION_COVER,
            TerrainDef {
                key:            test_pieces::LOW_VISION_COVER,
                display_name:   display("Test Low Cover"),
                sim_kind:       TerrainSimKind::Cover {
                    hp:               CoverHp::new(30),
                    armor_protection: ArmorProtection::new(2),
                    armor_hardness:   ArmorHardness::new(1),
                    height_band:      HeightBand::Low,
                },
                presenter_kind: TerrainPresenterKind::Cover {
                    graphic_name: graphic("low-cover"),
                },
                tags:           Vec::new(),
                on_death:       None,
                blocks_pathing: None,
                blocks_los:     None,
            },
        ),
    ])
}
