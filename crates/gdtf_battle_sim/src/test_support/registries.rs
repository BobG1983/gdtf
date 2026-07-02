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
    cover::{CoverHp, HeightBand},
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
        Accuracy, BaseSpread, DamageType, FISTS_KEY, FatalBias, FightMode, FightModeKind,
        FightModeSpec, FireMode, FireModeSpec, Handedness, Kickback, MagazineSize,
        MeleeWeaponRegistry, MeleeWeaponSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
        Reach, Shove, Stable, Strikes, TuCost, WeaponDamage, WeaponName, WeaponPunch,
        WeaponRegistry, WeaponShred, WeaponSpec,
    },
};

/// The weapon KEY every test ganger references — present in
/// [`test_weapon_registry`]. Every fixture builds gangers that resolve this key
/// against the test weapon registry, so a setup arms each one.
pub const TEST_WEAPON_KEY: &str = "test-weapon";

/// The armor KEY every test ganger references — present in
/// [`test_armor_registry`] (the armor mirror of [`TEST_WEAPON_KEY`]).
pub const TEST_ARMOR_KEY: &str = "test-armor";

/// The MELEE weapon KEY a test ganger that authors one references (GTW-505) — present
/// in [`test_melee_weapon_registry`] (the melee mirror of [`TEST_WEAPON_KEY`]).
pub const TEST_MELEE_WEAPON_KEY: &str = "test-melee";

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
        base_spread:      BaseSpread::new(0.25),
        accuracy:         Accuracy::new(1.0),
        kickback:         Kickback::new(0.4),
        fatal_bias:       FatalBias::new(7.0),
        damage:           WeaponDamage::new(12),
        punch:            WeaponPunch::new(5),
        shred:            WeaponShred::new(3),
        damage_type:      DamageType::Kinetic,
        magazine:         Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        fire_mode:        FireMode::new(vec![FireModeSpec::new(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.5),
            ModeShots::new(1),
        )]),
        stable:           Stable::new(false),
        // No knockback on the shared test weapon — a shove-tagged variant is built
        // per-test (the GTW-525 auto-shove tests spawn their own tagged weapon).
        shove:            Shove::new(false),
        handedness:       Handedness::OneHanded,
        // GTW-542: no attachments on the shared test weapon (an attachment-bearing variant
        // is built per-test); the empty list folds to the identity.
        attachment_slots: Vec::new(),
        // GTW-544: no DOT profile on the shared test weapon (a DOT-bearing variant is built
        // per-test); `None` is a non-DOT weapon, byte-identical to before this slice.
        dot:              None,
        on_death:         None,
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

/// An arbitrary [`MeleeWeaponSpec`] (NOT shipped magnitudes — mechanism only) carrying a
/// single `Swing`-kind fight mode, so a resolved bundle proves the
/// [`MeleeWeapon`](crate::weapon::MeleeWeapon) marker + [`FightMode`] landed (GTW-505).
/// SHARES the ranged damage newtypes; drops the ranged-only handling fields.
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
        // The shared test melee weapon does not knock back; a shove-tagged melee
        // weapon is built per-test where the auto-shove-on-connect is exercised.
        shove:       Shove::new(false),
    }
}

/// A [`MeleeWeaponRegistry`] holding the [`fists`](crate::weapon::FISTS_KEY) default
/// (under [`FISTS_KEY`]) AND the [`TEST_MELEE_WEAPON_KEY`] authored melee weapon — the
/// test-built registry a setup resolves each ganger's melee weapon against (GTW-505).
/// The `fists` entry makes a ganger that authors NO melee weapon resolve cleanly (the
/// `None → fists` default path); the `test-melee` entry exercises an authored key. Stands
/// in for the app's `Load`-built registry.
#[must_use]
pub fn test_melee_weapon_registry() -> MeleeWeaponRegistry {
    MeleeWeaponRegistry::new([
        // The fists default — `None`-authored gangers resolve to this.
        (
            WeaponName::new(FISTS_KEY.to_owned()),
            test_melee_weapon_spec(),
        ),
        // An authored melee weapon under the test key — a member that names it resolves here.
        (
            WeaponName::new(TEST_MELEE_WEAPON_KEY.to_owned()),
            test_melee_weapon_spec(),
        ),
    ])
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

/// A [`TerrainDefRegistry`] with the four test terrain definition UUIDs (GTW-491) — the
/// test-built UUID-keyed registry `setup_battle` resolves cover/slab piece UUIDs against in
/// the test harness (no `AssetServer`).
///
/// The four defs, keyed by the `test_pieces` UUID consts:
/// - `WALL` — a HIGH-band structural `Wall` sim-kind, presenter `Wall { graphic_name }`
///   (carries a graphic — the GTW-491 NET-NEW wall-graphic fact — and NO footfall).
/// - `SLAB` — a destructible `Slab` sim-kind, presenter `Slab { graphic_name, footfall:
///   Some(..) }` (a NAMED footfall, so the optional-footfall fact is exercised).
/// - `COVER` — a LOW-band `Cover` sim-kind, presenter `Cover { graphic_name }`.
/// - `FLOOR` — a `Slab`-kind walkable floor (no `Floor` variant in the new model), presenter
///   `Slab { graphic_name, footfall: None }`.
///
/// Magnitudes are ARBITRARY test data — they exist only to let the registry resolve the four
/// UUIDs that `SituationBuilder::wall_at` / `slab_at` and the situation test fixtures author.
/// Tests must not pin these magnitudes (brittle-test rule).
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
                    // A NAMED footfall, so the GTW-491 optional-footfall-on-slab fact (C4) is
                    // exercised by the standard fixture slab.
                    footfall:     Some(FootfallSound::new("test-step".to_owned())),
                },
                tags:           Vec::new(),
                on_death:       None,
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
            },
        ),
        (
            test_pieces::FLOOR,
            TerrainDef {
                key:            test_pieces::FLOOR,
                display_name:   display("Test Floor"),
                // The new model has no Floor sim-kind — a walkable floor is a Slab def.
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
            },
        ),
        (
            // GTW-502: a Slab carrying an explicit BlocksVision tag — occludes LoS/FoV at HIGH
            // despite being a slab (the tag-driven gap-closer the cover ledger never held).
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
            },
        ),
        (
            // GTW-502 C7 independence: a Slab carrying ONLY a BlocksPathfinding tag — blocks a
            // path but does NOT occlude vision.
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
            },
        ),
        (
            // GTW-502 height-awareness: a LOW-band Cover — occludes a LOW sightline, a HIGH
            // one clears it.
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
            },
        ),
    ])
}
