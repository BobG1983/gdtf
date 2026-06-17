//! Unit tests for the presenter plugin seam (mode selection + the renderer-active marker).

use bevy::prelude::*;

use super::renderer::{BattlePresenterMode, BattlePresenterPlugin, TopDownRendererActive};

/// AC1 — the default presenter selects `TopDown`, builds without panic under
/// `MinimalPlugins`, and its top-down renderer inserts the marker resource.
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

/// AC1 — the TopDown-mode app carries the marker; the Iso-mode app does not
/// (proving the mode switch actually selects the iso branch and the top-down
/// renderer did not run). Neither app panics on `update()`.
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
