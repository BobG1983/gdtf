//! Fall consequences: HP loss + a seeded injury, storey-monotone damage, seed
//! determinism, and hot-edited tuning (QA 5-7).

use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{CellLevel, Level, OccupancyGrid, PerStoreyDamage, SlabState, SurfaceGrid};

use super::harness::*;

/// Whether ANY `InjuryInflicted` addressed `entity` across the run (from the recorder).
fn injury_fired_for(app: &App, entity: Entity) -> bool {
    app.world()
        .get_resource::<InjuryLog>()
        .is_some_and(|log| log.targets.contains(&entity))
}

/// Install a populated Torso injury table + registry so a non-graze torso wound rolls a
/// named injury (the QA(5) `InjuryInflicted` proof) — the SHARED (category, severity) pool,
/// used AS-IS (no falling-specific source dimension; GTW-452 owns that).
fn install_torso_injury_content(app: &mut App) {
    use gdtf_battle_sim::{
        InjuryCategory, InjuryDef, InjuryName, InjuryRegistry, InjuryTables, InjuryWeight,
        InspectText, LogText, PopupText, PostHeal, Severity, WeightedInjuryEntry,
        WeightedInjuryTable,
    };
    let name = InjuryName::new("bruised_ribs".to_owned());
    // A minimal named def in the registry (no effects — the roll just needs a resolvable key).
    let def = InjuryDef {
        name:         name.clone(),
        category:     InjuryCategory::Torso,
        severity:     Severity::Minor,
        popup_text:   PopupText::new("Bruised!".to_owned()),
        log_text:     LogText::new("bruised ribs".to_owned()),
        inspect_text: InspectText::new("Bruised ribs from the fall.".to_owned()),
        effects:      Vec::new(),
        post_heal:    PostHeal::Deferred,
    };
    app.insert_resource(InjuryRegistry::new([(name.clone(), def)]));
    // Weight every non-graze/non-fatal severity for the Torso category to this one key, so any
    // wound tier that lands on the torso rolls it.
    let table = || {
        WeightedInjuryTable::new(vec![WeightedInjuryEntry::new(
            name.clone(),
            InjuryWeight::new(1),
        )])
    };
    app.insert_resource(InjuryTables::new([
        ((InjuryCategory::Torso, Severity::Minor), table()),
        ((InjuryCategory::Torso, Severity::Major), table()),
        ((InjuryCategory::Torso, Severity::Critical), table()),
    ]));
}

// ── QA(5): Hp dropped + seeded injury + InjuryInflicted + damage monotone ──────

/// QA(5): a fall drops Hp, fires a seeded `InjuryInflicted` for the faller, and the damage
/// is monotone in storeys (a deeper fall hurts at least as much). A CONTENT-FULL injury
/// table is used so a non-graze wound rolls a real named injury (the `InjuryInflicted` proof).
#[test]
fn fall_drops_hp_fires_injury_and_damage_is_monotone_in_storeys() {
    // A per-storey magnitude tuned so the fall's Torso penetrating damage (per_storey ×
    // storeys) plus the +5 torso part-mod plus the random roll (≤ 10) lands SAFELY in the
    // non-graze, non-fatal band (score ≥ e0=5, < e3=40) for a 2–3 storey fall — so a named
    // injury reliably rolls (Fatal / graze roll NO injury; the tabling rule). Value-agnostic:
    // the RELATION (HP drops, injury fires, monotone) is asserted, never a pinned magnitude.
    let per_storey = PerStoreyDamage::new(6);
    let run = |start_level: u8| -> (u16, bool) {
        let mut app = falls_app(SEED, per_storey);
        // A populated Torso injury table so a Minor/Major/Critical torso wound rolls a named
        // injury (the InjuryInflicted proof); reuses the SHARED (category, severity) pool.
        install_torso_injury_content(&mut app);
        let faller = spawn_faller(app.world_mut(), start_level);
        app.insert_resource(SurfaceGrid::new());
        app.insert_resource(OccupancyGrid::new());
        let before = hp_of(&app, faller);
        destroy_slab_and_settle(&mut app, start_level);
        let after = hp_of(&app, faller);
        (before - after, injury_fired_for(&app, faller))
    };

    let (lost_2, injury_2) = run(2);
    let (lost_3, injury_3) = run(3);

    assert!(lost_2 > 0, "a fall must drop Hp");
    assert!(
        lost_3 >= lost_2,
        "a deeper fall (3 storeys) must hurt at least as much as a shallow one (2) \
         (2:{lost_2} 3:{lost_3})"
    );
    assert!(
        lost_3 > lost_2,
        "the LINEAR per-storey scaling makes a 3-storey fall hurt strictly more"
    );
    // A seeded non-graze / non-fatal fall wound rolls a named injury from the populated table
    // → an InjuryInflicted message addressed to the faller (the C5 real-wiring proof). Both
    // the 2- and 3-storey falls land in the non-fatal band, so both fire.
    assert!(
        injury_2 && injury_3,
        "a seeded fall wound must roll a named injury and fire InjuryInflicted (2:{injury_2} 3:{injury_3})"
    );
}

// ── QA(6): determinism — same seed twice → identical outcome ──────────────────

/// QA(6): the same seed + same event order yields the IDENTICAL fall outcome (landing +
/// HP loss + storeys) across two independent apps.
#[test]
fn falls_are_deterministic_under_same_seed() {
    let run = || -> (u8, u16, u8) {
        let mut app = falls_app(SEED, PerStoreyDamage::new(20));
        install_torso_injury_content(&mut app);
        let faller = spawn_faller(app.world_mut(), 3);
        let mut surface = SurfaceGrid::new();
        surface.set_slab(
            CellLevel::new(column_cell(), Level::new(1)),
            SlabState::Present,
        );
        app.insert_resource(surface);
        app.insert_resource(OccupancyGrid::new());
        let before = hp_of(&app, faller);
        destroy_slab_and_settle(&mut app, 3);
        let signals = fall_signals(&app);
        let storeys = signals.first().map_or(0, |s| *s.storeys);
        (
            level_of(&app, faller),
            before - hp_of(&app, faller),
            storeys,
        )
    };
    assert_eq!(
        run(),
        run(),
        "same seed + same event order must yield identical fall outcomes"
    );
}

// ── QA(7): hot-edit per_storey_damage → different damage (formula) ────────────

/// QA(7): a hot-edit of `per_storey_damage` changes the blow — a larger leaf deals MORE Hp
/// loss for the same fall. The FORMULA relation is asserted, NOT a shipped magnitude.
#[test]
fn hot_edit_per_storey_damage_changes_fall_damage() {
    let lost = |per_storey: i32| -> u16 {
        let mut app = falls_app(SEED, PerStoreyDamage::new(per_storey));
        let faller = spawn_faller(app.world_mut(), 3);
        let mut surface = SurfaceGrid::new();
        surface.set_slab(
            CellLevel::new(column_cell(), Level::new(1)),
            SlabState::Present,
        );
        app.insert_resource(surface);
        app.insert_resource(OccupancyGrid::new());
        let before = hp_of(&app, faller);
        destroy_slab_and_settle(&mut app, 3);
        before - hp_of(&app, faller)
    };
    let small = lost(5);
    let large = lost(50);
    assert!(
        large > small,
        "a larger per_storey_damage leaf deals more fall damage (small:{small} large:{large})"
    );
}
