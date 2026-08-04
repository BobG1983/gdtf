use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{
    prelude::{CellLevel, Level, OccupancyGrid},
    surface::{SlabState, SurfaceGrid},
    tuning::PerStoreyDamage,
};

use super::harness::*;

fn injury_fired_for(app: &App, entity: Entity) -> bool {
    app.world()
        .get_resource::<InjuryLog>()
        .is_some_and(|log| log.targets.contains(&entity))
}

fn install_torso_injury_content(app: &mut App) {
    use gdtf_battle_sim::{
        armor::InjuryCategory,
        injuries::{
            DamageContext, InjuryDef, InjuryName, InjuryRegistry, InjuryTables, InjuryWeight,
            InspectText, LogText, PopupText, PostHeal, WeightedInjuryEntry, WeightedInjuryTable,
        },
        severity::Severity,
    };
    let name = InjuryName::new("bruised_ribs".to_owned());
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
    let table = || {
        WeightedInjuryTable::new(vec![WeightedInjuryEntry::new(
            name.clone(),
            InjuryWeight::new(1),
        )])
    };
    app.insert_resource(InjuryTables::new([
        (
            (InjuryCategory::Torso, DamageContext::Fall, Severity::Minor),
            table(),
        ),
        (
            (InjuryCategory::Torso, DamageContext::Fall, Severity::Major),
            table(),
        ),
        (
            (
                InjuryCategory::Torso,
                DamageContext::Fall,
                Severity::Critical,
            ),
            table(),
        ),
    ]));
}

#[test]
fn fall_drops_hp_fires_injury_and_damage_is_monotone_in_storeys() {
    let per_storey = PerStoreyDamage::new(6);
    let run = |start_level: u8| -> (u16, bool) {
        let mut app = falls_app(SEED, per_storey);
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
    assert!(
        injury_2 && injury_3,
        "a seeded fall wound must roll a named injury and fire InjuryInflicted (2:{injury_2} 3:{injury_3})"
    );
}

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
