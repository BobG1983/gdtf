//! Unit tests for the presenter plugin seam (mode selection + the renderer-active marker)
//! and the GTW-623 chained draw-stage contract.

use bevy::prelude::*;

use super::{BattlePresenterMode, BattlePresenterPlugin, TopDownRendererActive};
use crate::PresenterSystems;

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

/// GTW-623 (C1 / C2 / A1) — the three draw stages are chained `Scene` → `Compose` →
/// `Overlay` by the ONE `configure_sets` in `TopDownRendererPlugin::build`: ordering
/// between draw systems is STAGE MEMBERSHIP, not pairwise `.after` edges.
///
/// One probe system per stage records its run order into a shared log; the probes are
/// deliberately REGISTERED in reverse (Overlay, Scene, Compose) so insertion order
/// cannot fake the pass — only the chained set config can produce
/// `[Scene, Compose, Overlay]`.
#[test]
fn draw_stages_run_scene_then_compose_then_overlay() {
    /// A probe's stage tag, in the order the chain must produce.
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    enum StageTag {
        Scene,
        Compose,
        Overlay,
    }
    /// The observed run order the three probes append to.
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
