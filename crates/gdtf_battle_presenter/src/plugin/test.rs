use bevy::prelude::*;

use super::{BattlePresenterMode, BattlePresenterPlugin, TopDownRendererActive};
use crate::PresenterSystems;

#[test]
fn default_presenter_selects_topdown_and_inserts_the_marker() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(BattlePresenterPlugin::default());
    app.update();

    assert_eq!(
        BattlePresenterPlugin::default().mode(),
        BattlePresenterMode::TopDown,
        "the default presenter mode must be TopDown",
    );
    assert!(
        app.world()
            .get_resource::<TopDownRendererActive>()
            .is_some(),
        "the default (TopDown) presenter must insert the TopDownRendererActive marker",
    );
}

#[test]
fn mode_switch_selects_the_named_renderer_branch() {
    let mut topdown_app = App::new();
    topdown_app
        .add_plugins(MinimalPlugins)
        .add_plugins(BattlePresenterPlugin::new(BattlePresenterMode::TopDown));
    topdown_app.update();

    let mut iso_app = App::new();
    iso_app
        .add_plugins(MinimalPlugins)
        .add_plugins(BattlePresenterPlugin::new(BattlePresenterMode::Iso));
    iso_app.update();

    assert!(
        topdown_app
            .world()
            .get_resource::<TopDownRendererActive>()
            .is_some(),
        "the TopDown-mode presenter must carry the TopDownRendererActive marker",
    );
    assert!(
        iso_app
            .world()
            .get_resource::<TopDownRendererActive>()
            .is_none(),
        "the Iso-mode presenter must NOT carry the TopDown marker — the iso branch ran",
    );
}

#[test]
fn draw_stages_run_scene_then_compose_then_overlay() {
        #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    enum StageTag {
        Scene,
        Compose,
        Overlay,
    }
        #[derive(Resource, Default)]
    struct RunOrder(Vec<StageTag>);

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(BattlePresenterPlugin::default())
        .init_resource::<RunOrder>()
        .add_systems(
            Update,
            (|mut log: ResMut<RunOrder>| log.0.push(StageTag::Overlay))
                .in_set(PresenterSystems::Overlay),
        )
        .add_systems(
            Update,
            (|mut log: ResMut<RunOrder>| log.0.push(StageTag::Scene))
                .in_set(PresenterSystems::Scene),
        )
        .add_systems(
            Update,
            (|mut log: ResMut<RunOrder>| log.0.push(StageTag::Compose))
                .in_set(PresenterSystems::Compose),
        );
    app.update();

    assert_eq!(
        app.world().resource::<RunOrder>().0,
        vec![StageTag::Scene, StageTag::Compose, StageTag::Overlay],
        "the Draw stages must run chained Scene → Compose → Overlay (GTW-623 C1)",
    );
}
