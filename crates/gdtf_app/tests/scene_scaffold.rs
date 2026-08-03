use bevy::{app::App, ecs::resource::Resource};
use gdtf_app::test_support::{AppState, LoadedSituation, app_state, load_released};
use gdtf_battle_sim::{
    injuries::InjuryRegistry,
    situation::Situation,
    tuning::{CombatTuning, GangerStatTuning},
    weapon::WeaponRegistry,
};
use gdtf_state_scoped::StateScopedResourceAppExt as _;
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};

const WALK_BUDGET: u32 = 64;

#[derive(Resource, Debug, PartialEq, Eq)]
struct LoadScopedProbe(u8);

impl LoadScopedProbe {
        const fn seeded() -> Self {
        Self(0xA5)
    }
}

fn scaffold_walk_app() -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .default_start()
        .build();

    app.init_state_scoped_resource(AppState::Load, LoadScopedProbe::seeded);

    app.world_mut()
        .insert_resource(gdtf_ui::theme::default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(GangerStatTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::MeleeWeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::equipment::attachments::AttachmentRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::terrain::def::TerrainDefRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::UuidThemeRegistry::default());
    app.world_mut()
        .insert_resource(LoadedSituation::new(Situation::default()));
    app
}

#[test]
fn state_scoped_probe_lives_exactly_across_the_load_span() {
    let mut app = scaffold_walk_app();

    app.update();
    assert_eq!(app_state(&app), AppState::Init, "the walk starts in Init");
    assert!(
        app.world().get_resource::<LoadScopedProbe>().is_none(),
        "the Load-scoped probe must be ABSENT before Load is entered",
    );

    let reached_load = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Load,
        WALK_BUDGET,
    );
    assert!(
        reached_load,
        "Init's marker scaffold must advance the walk to Load"
    );

    assert_eq!(
        app.world().get_resource::<LoadScopedProbe>(),
        Some(&LoadScopedProbe::seeded()),
        "the probe must be PRESENT with its exact seeded value while in Load",
    );

    let released = advance_until(&mut app, load_released, WALK_BUDGET);
    assert!(
        released,
        "the seeded gates must let Load release to Intro (or beyond)"
    );
    assert!(
        app.world().get_resource::<LoadScopedProbe>().is_none(),
        "the probe must be REMOVED once Load exits",
    );

    let reached_running = advance_until(
        &mut app,
        |app| app_state(app) == AppState::Running,
        WALK_BUDGET,
    );
    assert!(
        reached_running,
        "Intro's marker scaffold must advance the walk to Running",
    );
}
