//! Scene scaffold: Load-scoped resources live only across the Load span.

use bevy::{app::App, ecs::resource::Resource};
use cobalt_state_scoped::StateScopedResourceAppExt as _;
use cobalt_test_utils::{MinimalTestAppBuilder, advance_until};
use gdtf_battle_sim::{
    injuries::InjuryRegistry,
    situation::Situation,
    tuning::{CombatTuning, GangerStatTuning},
    weapon::WeaponRegistry,
};
use gdtf_content_families::situation::LoadedSituation;
use gdtf_game::test_support::{AppState, app_state, load_released};

/// Fixed steps a frame runs, so Intro's steps advance per frame and never off the real clock.
const ONE_STEP_A_FRAME: u32 = 1;

#[derive(Resource, Debug, PartialEq, Eq)]
struct LoadScopedProbe(u8);

impl LoadScopedProbe {
    const fn seeded() -> Self {
        Self(0xA5)
    }
}

fn scaffold_walk_app() -> App {
    let mut app =
        MinimalTestAppBuilder::new_with_scene_support(gdtf_game::test_support::register_headless)
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
    app.insert_resource(bevy::time::TimeUpdateStrategy::FixedTimesteps(
        ONE_STEP_A_FRAME,
    ));
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

    advance_until(&mut app, |app| app_state(app) == AppState::Load);

    assert_eq!(
        app.world().get_resource::<LoadScopedProbe>(),
        Some(&LoadScopedProbe::seeded()),
        "the probe must be PRESENT with its exact seeded value while in Load",
    );

    advance_until(&mut app, load_released);
    assert!(
        app.world().get_resource::<LoadScopedProbe>().is_none(),
        "the probe must be REMOVED once Load exits",
    );

    advance_until(&mut app, |app| app_state(app) == AppState::Running);
}
