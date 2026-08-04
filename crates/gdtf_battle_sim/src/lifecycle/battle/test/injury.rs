use bevy::prelude::{App, Entity, Messages};

use super::support::*;
use crate::{
    acts::InjuryInflicted,
    armor::InjuryCategory,
    ganger::{Aiming, Direction, Facing, Position, Shooting, Toughness},
    injuries::{
        DamageContext, InflictedInjuries, InjuryDef, InjuryEffect, InjuryName, InjuryRegistry,
        InjuryTables, InjuryWeight, InspectText, LogText, PopupText, PostHeal, StatDelta,
        StatTarget, WeightedInjuryEntry, WeightedInjuryTable,
    },
    metric::CellLevel,
    severity::Severity,
    situation::Situation,
    test_support::{GangerSpawnBuilder, single_mode, test_weapon_spec},
};

pub(super) fn aim_debuff_catalog() -> (InjuryRegistry, InjuryTables) {
    let key = InjuryName::new("aim_debuff".to_owned());
    let def = InjuryDef {
        name:         key.clone(),
        category:     InjuryCategory::Torso,
        severity:     Severity::Minor,
        popup_text:   PopupText::new("AIM HURT".to_owned()),
        log_text:     LogText::new("takes an aim-fouling wound".to_owned()),
        inspect_text: InspectText::new("Aim Debuff -- -2 Aim".to_owned()),
        effects:      vec![InjuryEffect::Modify {
            stat:   StatTarget::Aim,
            amount: StatDelta::new(-2),
        }],
        post_heal:    PostHeal::Deferred,
    };
    let registry = InjuryRegistry::new([(key.clone(), def)]);
    let mut tables = InjuryTables::default();
    for category in InjuryCategory::ALL {
        for severity in [Severity::Minor, Severity::Major, Severity::Critical] {
            tables.insert(
                category,
                DamageContext::Ranged,
                severity,
                WeightedInjuryTable::new(vec![WeightedInjuryEntry::new(
                    key.clone(),
                    InjuryWeight::new(10),
                )]),
            );
        }
    }
    (registry, tables)
}

pub(super) fn duel_situation() -> Situation {
    let shooter = GangerSpawnBuilder::new()
        .at(CellLevel::new(Cell::new(5, 5), Level::new(0)))
        .faction(Faction::new(0))
        .facing(Facing::new(Direction::East))
        .aiming(Aiming::new(true))
        .build();
    let target = GangerSpawnBuilder::new()
        .at(CellLevel::new(Cell::new(10, 5), Level::new(0)))
        .faction(Faction::new(1))
        .toughness(Toughness::new(15.0))
        .build();
    SituationBuilder::new()
        .with_gangers([shooter, target])
        .player_faction(Faction::new(0))
        .build()
}

pub(super) fn duel_entities(app: &mut App) -> (Entity, Entity) {
    let world = app.world_mut();
    let mut q = world.query::<(Entity, &Faction)>();
    let (mut shooter, mut target) = (Entity::PLACEHOLDER, Entity::PLACEHOLDER);
    for (e, &fac) in q.iter(world) {
        if fac == Faction::new(0) {
            shooter = e;
        } else if fac == Faction::new(1) {
            target = e;
        }
    }
    (shooter, target)
}

struct BattleRun {
    emitted:         usize,
    gained:          usize,
    shooting_before: f32,
    shooting_after:  f32,
    gained_names:    Vec<InjuryName>,
}

pub(super) fn penetrating_weapon_registry() -> crate::weapon::WeaponRegistry {
    use crate::weapon::{
        Accuracy, BaseSpread, FatalBias, FireMode, Kickback, WeaponName, WeaponPunch,
        WeaponRegistry, WeaponShred, WeaponSpec,
    };
    let spec = WeaponSpec {
        base_spread: BaseSpread::new(0.05),
        accuracy: Accuracy::new(2.0),
        kickback: Kickback::new(0.1),
        fatal_bias: FatalBias::new(0.0),
        punch: WeaponPunch::new(20),
        shred: WeaponShred::new(10),
        fire_mode: FireMode::new(vec![single_mode(0.2, 1)]),
        stable: crate::weapon::Stable::new(true),
        ..test_weapon_spec()
    };
    WeaponRegistry::new([(WeaponName::new("test-weapon".to_owned()), spec)])
}

pub(super) fn paper_armor_registry() -> crate::armor::ArmorRegistry {
    use crate::armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorName, ArmorPiece, ArmorProtection,
        ArmorRegistry, ArmorSpec, ArmorType,
    };
    let spec = ArmorSpec::uniform(ArmorPiece::new(
        ArmorFloor::new(0),
        ArmorProtection::new(0),
        ArmorIntegrity::new(1),
        ArmorHardness::new(0),
        ArmorType::DEFAULT,
    ));
    ArmorRegistry::new([(ArmorName::new("test-armor".to_owned()), spec)])
}

fn run_battle(seed: u64) -> BattleRun {
    let mut app = headless_app();
    app.insert_resource(penetrating_weapon_registry());
    app.insert_resource(paper_armor_registry());
    let (registry, tables) = aim_debuff_catalog();
    app.insert_resource(registry);
    app.insert_resource(tables);
    app.world_mut().write_message(SetupBattleRequested::new(
        duel_situation(),
        BattleSeed::new(seed),
    ));
    for _ in 0..4 {
        app.update();
    }

    let (shooter, target) = duel_entities(&mut app);
    let target_cell = app
        .world()
        .get::<Position>(target)
        .map_or(Cell::new(10, 5), |p| p.cell());
    let shooting_before = app.world().get::<Shooting>(target).map_or(0.0, |s| **s);

    let mode = FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    );
    app.world_mut().write_message(FireRequested::new(
        shooter,
        mode,
        target_cell,
        Level::new(0),
    ));
    app.update();
    let emitted = drain_injuries(&app);
    app.update();

    let (gained, gained_names) =
        app.world()
            .get::<InflictedInjuries>(target)
            .map_or((0, Vec::new()), |l| {
                (
                    l.gained().len(),
                    l.gained().iter().map(|g| g.name.clone()).collect(),
                )
            });
    let shooting_after = app.world().get::<Shooting>(target).map_or(0.0, |s| **s);

    BattleRun {
        emitted,
        gained,
        shooting_before,
        shooting_after,
        gained_names,
    }
}

fn drain_injuries(app: &App) -> usize {
    let messages = app.world().resource::<Messages<InjuryInflicted>>();
    let mut cursor = messages.get_cursor();
    cursor.read(messages).count()
}

#[test]
fn live_fire_inflicts_an_injury_end_to_end() {
    let mut hit = None;
    for seed in [
        0xA1u64, 0xB2, 0xC3, 0xD4, 0xE5, 0xF6, 0x17, 0x28, 0x39, 0x4A,
    ] {
        let run = run_battle(seed);
        if run.emitted >= 1 && run.gained >= 1 {
            hit = Some(run);
            break;
        }
    }
    assert!(
        hit.is_some(),
        "across the seed sweep, at least one point-blank shot must land a tabled wound \
         and inflict an injury end-to-end (InjuryInflicted emitted + the ledger gained it)"
    );
    let Some(run) = hit else {
        return;
    };

    assert!(
        run.emitted >= 1,
        "the fire emitted an InjuryInflicted message"
    );
    assert!(
        run.gained >= 1,
        "the target's InflictedInjuries ledger gained the injury"
    );
    assert!(
        run.gained_names.iter().any(|n| **n == "aim_debuff"),
        "the gained injury is the rolled `aim_debuff` (the catalog's only entry)"
    );
    assert!(
        run.shooting_after < run.shooting_before,
        "the projector must apply the injury's stat delta — derived Shooting must fall \
         (before {}, after {})",
        run.shooting_before,
        run.shooting_after,
    );
}

#[test]
fn same_seed_reproduces_identical_injuries() {
    let seed = 0xA1u64;
    let run_a = run_battle(seed);
    let run_b = run_battle(seed);
    assert_eq!(
        run_a.gained, run_b.gained,
        "the same seed must inflict the same NUMBER of injuries across runs",
    );
    assert_eq!(
        run_a.gained_names, run_b.gained_names,
        "the same seed must inflict the same injuries (same keys, same order)",
    );
    assert!(
        (run_a.shooting_after - run_b.shooting_after).abs() < f32::EPSILON,
        "the same seed must reproduce the same post-injury derived Shooting",
    );
}
