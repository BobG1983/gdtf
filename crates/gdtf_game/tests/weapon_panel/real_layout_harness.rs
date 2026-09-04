use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, AssetServer, Assets},
    audio::AudioPlugin,
    ecs::error::warn,
    gizmos::GizmoPlugin,
    log::LogPlugin,
    prelude::*,
    render::{RenderPlugin, settings::WgpuSettings},
    text::Font,
    window::{ExitCondition, Window, WindowPlugin, WindowResolution},
    winit::WinitPlugin,
};
use gdtf_content_families::situation::LoadedSituation;
use gdtf_game::test_support::{self, AppState, BattleScapeState, RunningState};
use gdtf_test_utils::advance_until;
use gdtf_ui::theme::default_theme;

fn workspace_assets_root() -> String {
    let Some(root) = cobalt_ron_assets::workspace_assets_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root.to_string_lossy().into_owned()
}

const REFERENCE_WIDTH: u32 = 1280;
const REFERENCE_HEIGHT: u32 = 720;

fn new_real_layout_app() -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: None,
                    ..default()
                }
                .into(),
                ..default()
            })
            .disable::<WinitPlugin>()
            .disable::<LogPlugin>()
            .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
            .disable::<GizmoPlugin>()
            .disable::<AudioPlugin>()
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(REFERENCE_WIDTH, REFERENCE_HEIGHT),
                    ..default()
                }),
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(AssetPlugin {
                file_path: workspace_assets_root(),
                ..default()
            }),
    );
    app.set_error_handler(warn);
    app.insert_resource(bevy::time::TimeUpdateStrategy::FixedTimesteps(1));
    test_support::register_scenes_with_default_plugins(&mut app);
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Running);
    app
}

fn seed_load_gate_registries(app: &mut App) {
    app.world_mut().insert_resource(LoadedSituation::new(
        gdtf_battle_sim::situation::Situation::default(),
    ));
    app.world_mut()
        .insert_resource(gdtf_battle_sim::tuning::CombatTuning::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::WeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::MeleeWeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::equipment::attachments::AttachmentRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::injuries::InjuryRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::terrain::def::TerrainDefRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::UuidThemeRegistry::default());
}

fn insert_real_theme_and_wait_for_font(app: &mut App) {
    let mut theme = default_theme();
    let asset_server = app.world().resource::<AssetServer>().clone();
    let font_path: String = (*theme.default_font).clone();
    let font: Handle<Font> = asset_server.load(font_path);
    theme.text.font = font.clone();
    theme.button.font = font.clone();
    app.world_mut().insert_resource(theme);

    advance_until(app, move |app| {
        app.world().resource::<Assets<Font>>().get(&font).is_some()
    });
}

pub(crate) fn real_layout_battle_running_app() -> App {
    let mut app = new_real_layout_app();
    seed_load_gate_registries(&mut app);

    insert_real_theme_and_wait_for_font(&mut app);

    advance_until(&mut app, |app| {
        app.world()
            .get_resource::<State<RunningState>>()
            .map(|state| *state.get())
            == Some(RunningState::Menu)
    });
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    advance_until(&mut app, |app| {
        app.world()
            .get_resource::<State<BattleScapeState>>()
            .map(|state| *state.get())
            == Some(BattleScapeState::BattleRunning)
    });
    app
}
