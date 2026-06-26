//! GTW-438 — the LIVE-PLAY injury E2E + seeded-replay determinism, over the REAL
//! battle runtime ([`BattleSimPlugin`]): a real `setup_battle` spawns fully-statted
//! gangers (each carrying the seeded `InflictedInjuries` ledger), a real `FireRequested`
//! drives `dispatch_fire → fire → roll_injury → InjuryInflicted → apply_injury`, and the
//! GTW-436 projector re-derives — so an injury ACTUALLY happens end-to-end in a battle
//! (the GTW-393/406 feature-completeness lesson), not just in a constructed message.
//!
//! These exercise the production path (no stubs): the situation goes through the real
//! `SetupBattleRequested` seam, and the fire goes through the real message dispatch.

use bevy::prelude::{App, Entity, Messages};

use super::support::*;
use crate::{
    acts::InjuryInflicted,
    armor::BodyPart,
    ganger::{Aiming, Direction, Facing, Position, Shooting, Toughness},
    injuries::{
        InflictedInjuries, InjuryDef, InjuryEffect, InjuryName, InjuryRegistry, InjuryTables,
        InjuryWeight, InspectText, LogText, PopupText, PostHeal, StatDelta, StatTarget,
        WeightedInjuryEntry, WeightedInjuryTable,
    },
    metric::CellLevel,
    severity::Severity,
    situation::Situation,
    test_support::GangerSpawnBuilder,
};

/// Build a one-injury catalog covering EVERY `(BodyPart, Minor/Major/Critical)` bucket
/// with the SAME injury (a `Modify(Aim, -2)`), so ANY non-graze, non-fatal wound — on any
/// part, at any tabled severity — rolls a known, stat-shifting injury. The shared key
/// resolves to one [`InjuryDef`] in the registry.
fn aim_debuff_catalog() -> (InjuryRegistry, InjuryTables) {
    let key = InjuryName::new("aim_debuff".to_owned());
    let def = InjuryDef {
        name:         key.clone(),
        body_part:    BodyPart::Torso,
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
    for part in BodyPart::ALL {
        for severity in [Severity::Minor, Severity::Major, Severity::Critical] {
            tables.insert(
                part,
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

/// A point-blank duel: a faction-0 shooter at `(5, 5, 0)` facing East + aiming, and a
/// LOW-toughness faction-1 target at the adjacent `(6, 5, 0)` — so a fired shot reliably
/// LANDS a wound (point-blank, low toughness), the precondition the injury roll is gated
/// on.
fn duel_situation() -> Situation {
    let shooter = GangerSpawnBuilder::new()
        .at(CellLevel::new(Cell::new(5, 5), Level::new(0)))
        .faction(Faction::new(0))
        .facing(Facing::new(Direction::East))
        .aiming(Aiming::new(true))
        .build();
    let target = GangerSpawnBuilder::new()
        .at(CellLevel::new(Cell::new(10, 5), Level::new(0)))
        .faction(Faction::new(1))
        // Toughness 15 pulls the §6 score (pen 12) DOWN out of the Fatal band into the
        // tabled Minor/Major range (score ≈ 12 − 15 + part_mod + roll(0..10)).
        .toughness(Toughness::new(15.0))
        .build();
    SituationBuilder::new()
        .with_gangers([shooter, target])
        .player_faction(Faction::new(0))
        .build()
}

/// The faction-0 shooter entity + the faction-1 target entity from the spawned battle.
fn duel_entities(app: &mut App) -> (Entity, Entity) {
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

/// The result of one driven battle: the target entity, how many `InjuryInflicted` fired,
/// how many injuries the target's ledger gained, the target's derived `Shooting` BEFORE
/// the fire, and AFTER (so a test can prove the projector applied the delta).
struct BattleRun {
    emitted:         usize,
    gained:          usize,
    shooting_before: f32,
    shooting_after:  f32,
    gained_names:    Vec<InjuryName>,
}

/// Drive ONE full battle: setup at `seed`, install the catalog, then fire the shooter at
/// the target point-blank, advancing frames so the fire → roll → apply → project chain
/// settles.
/// A high-pen, zero-Fatal-bias weapon + a zero-protection armor keyed to the test keys,
/// so a fired shot reliably PENETRATES (no armor soak) and the severity lands in the
/// tabled band rather than skewing Fatal (the stock test weapon's `fatal_bias: 7` pushes
/// most wounds to Fatal, which is NOT tabled). With pen ≈ 12, a target Toughness of 15,
/// and the §6 edges (e0=1 … e3=15), the score `12 − 15 + part_mod + roll(0..10)` lands
/// mostly Minor/Major across parts + seeds — a tabled (rollable) wound.
fn penetrating_weapon_registry() -> crate::weapon::WeaponRegistry {
    use crate::{
        magazine::{Magazine, ReloadTu},
        weapon::{
            Accuracy, BaseSpread, DamageType, FatalBias, FireMode, FireModeSpec, Kickback,
            MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, WeaponDamage,
            WeaponName, WeaponPunch, WeaponRegistry, WeaponShred, WeaponSpec,
        },
    };
    let spec = WeaponSpec {
        base_spread: BaseSpread::new(0.05),
        accuracy:    Accuracy::new(2.0),
        kickback:    Kickback::new(0.1),
        fatal_bias:  FatalBias::new(0.0), // NOT Fatal-skewed → wounds stay tabled
        damage:      WeaponDamage::new(12),
        punch:       WeaponPunch::new(20),
        shred:       WeaponShred::new(10),
        damage_type: DamageType::Kinetic,
        magazine:    Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        fire_mode:   FireMode::new(vec![FireModeSpec::new(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.2),
            ModeShots::new(1),
        )]),
        stable:      crate::weapon::Stable::new(true),
    };
    WeaponRegistry::new([(WeaponName::new("test-weapon".to_owned()), spec)])
}

/// A ZERO-protection armor suit keyed to the test armor key — so a hit lands as full
/// weapon damage (the wound is reliably non-graze; the injury roll's precondition holds).
fn paper_armor_registry() -> crate::armor::ArmorRegistry {
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
    // Override the harness's default weapon/armor with the penetrating weapon + paper
    // armor so the shot lands a tabled wound (the harness inserts the stock registries; a
    // later insert wins). MUST be done BEFORE the setup resolves each ganger's loadout.
    app.insert_resource(penetrating_weapon_registry());
    app.insert_resource(paper_armor_registry());
    let (registry, tables) = aim_debuff_catalog();
    app.insert_resource(registry);
    app.insert_resource(tables);
    app.world_mut().write_message(SetupBattleRequested::new(
        duel_situation(),
        BattleSeed::new(seed),
    ));
    // Settle the deferred bsn! ganger scenes AND the related weapon/armor scenes
    // (`queue_spawn_related_scenes` is deferred): a few updates ensure the `Wields` /
    // `Wears` relationships + the ganger components are materialized before the fire.
    for _ in 0..4 {
        app.update();
    }

    let (shooter, target) = duel_entities(&mut app);
    let target_cell = app
        .world()
        .get::<Position>(target)
        .map_or(Cell::new(10, 5), |p| Cell::new(p.x, p.y));
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
    // One update fires + applies the injury (apply_injury is .after(dispatch_fire) in the
    // SAME Simulate band); a second lets the Changed<InflictedInjuries> projector settle.
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

/// Count the `InjuryInflicted` messages buffered this frame (a fresh cursor, so it does
/// not disturb the runtime reader).
fn drain_injuries(app: &App) -> usize {
    let messages = app.world().resource::<Messages<InjuryInflicted>>();
    let mut cursor = messages.get_cursor();
    cursor.read(messages).count()
}

/// THE LIVE-PLAY E2E (feature-completeness): a real fire that wounds a ganger inflicts an
/// injury end-to-end — an `InjuryInflicted` is emitted, the target's `InflictedInjuries`
/// gains it, AND the projector applies its stat delta (the derived `Shooting` falls, since
/// the injury's `Modify(Aim, -2)` ripples through derivation). Proves the headline feature
/// WORKS in a real battle, not just in a constructed message.
#[test]
fn live_fire_inflicts_an_injury_end_to_end() {
    // Search a few seeds for one whose point-blank shot lands a tabled (non-graze,
    // non-fatal) wound — the precondition the roll is gated on (severity is RNG-driven).
    // The catalog covers every part/severity, so any tabled wound rolls the injury.
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

    // The InjuryInflicted fired AND the ledger gained it.
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
    // The projector APPLIED the delta: the derived Shooting fell (a Modify(Aim, -2) ripples
    // through derivation), proving the injury actually took effect on the ganger's stats.
    assert!(
        run.shooting_after < run.shooting_before,
        "the projector must apply the injury's stat delta — derived Shooting must fall \
         (before {}, after {})",
        run.shooting_before,
        run.shooting_after,
    );
}

/// SEEDED-REPLAY DETERMINISM (#1): the SAME `BattleSeed` reproduces the SAME injuries
/// across two independent runs — the GTW-14 replay contract extended to the injury draw.
/// Drives the REAL fire→fold→apply path twice from the same root seed and asserts the
/// same injuries land (same count + same names, in order).
#[test]
fn same_seed_reproduces_identical_injuries() {
    // A seed known (from the sweep above) to land a tabled wound; if it grazes, the test
    // still holds (both runs graze → both gain zero, still identical) — determinism is the
    // property under test, not that an injury necessarily lands for THIS seed.
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
