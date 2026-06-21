//! Structure tests for the inspect-panel ROOT layout (the GTW-275 overlay overhaul).
//!
//! The inspect panel must be an ABSOLUTE, fixed-% overlay anchored TOP-RIGHT — it hovers OVER
//! the map and contributes NOTHING to the world-map viewport inset (only the bottom bar
//! reduces the map). These tests drive the real [`spawn_inspect_panel`] system over a `World`
//! and assert the spawned [`InspectPanelRoot`]'s [`Node`] is:
//!
//! - `PositionType::Absolute` (an overlay, removed from layout flow);
//! - sized in RELATIVE viewport units — a FIXED [`Val::Vw`] width (so the map never shifts
//!   when panel contents change) and a [`Val::Vh`] max-height cap;
//! - anchored to the TOP-RIGHT corner (`top`/`right` set, `left`/`bottom` unset).
//!
//! They assert the unit *KIND* (Vw/Vh, Absolute), NOT any pixel magnitude — the contract's
//! "no px magnitudes" rule for the overhaul's structure tests.

use bevy::{
    MinimalPlugins,
    asset::AssetPlugin,
    ecs::system::SystemState,
    prelude::*,
    scene::ScenePlugin,
    ui::{Node, PositionType, Val},
};
use gdtf_ui::theme::default_theme;

use crate::states::running::game::battlescape::inspect_panel::{
    components::InspectPanelRoot, systems::spawn_inspect_panel,
};

/// The `SystemState` params `spawn_inspect_panel` reads, aliased to keep the tuple out of the
/// `type_complexity` deny lint (the system is `Commands`-driven with two optional resources).
type SpawnParams<'w, 's> = SystemState<(
    Commands<'w, 's>,
    Option<Res<'w, gdtf_ui::theme::GdtfTheme>>,
    Option<Res<'w, gdtf_battle_presenter::TopDownAtlases>>,
)>;

/// Runs the real `spawn_inspect_panel` system once and returns the spawned
/// `InspectPanelRoot`'s `Node`, if any. Drives the actual production spawn path via a
/// `SystemState` (the loaded fallback theme inserted, no atlases — the portrait takes
/// its `None` branch), not a hand-rolled copy.
///
/// GTW-322 — the panel builders now spawn through `Commands::spawn_scene` (a `bsn!`
/// [`Scene`](bevy::scene::Scene)), whose deferred `apply_scene` reads the
/// `AssetServer` + `Assets<ScenePatch>` resources; a raw `World` lacks them and panics
/// on apply. So the system is driven over an `App` world carrying `MinimalPlugins` +
/// `AssetPlugin` + `ScenePlugin` (the scene/asset infrastructure); `state.apply` runs
/// the queued scene application (dependency-free → resolves immediately) and the
/// `InspectPanelRoot` / `Node` insert, both before the assertion reads the `Node`.
fn spawn_root_node() -> Option<Node> {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.insert_resource(default_theme());
    let world = app.world_mut();

    let mut state: SpawnParams = SystemState::new(world);
    // `get` now returns a `Result` (Bevy 0.19); these params always validate.
    let Ok((commands, theme, atlases)) = state.get(world) else {
        return None;
    };
    // `spawn_inspect_panel` is `Commands`-driven; build it from the same params then apply.
    spawn_inspect_panel(commands, theme, atlases);
    state.apply(world);

    let mut roots = world.query_filtered::<&Node, With<InspectPanelRoot>>();
    roots.iter(world).next().cloned()
}

#[test]
fn inspect_panel_root_is_an_absolute_overlay() {
    let node = spawn_root_node();
    assert!(
        node.is_some(),
        "spawn_inspect_panel must spawn an InspectPanelRoot"
    );
    let Some(node) = node else {
        return;
    };
    assert_eq!(
        node.position_type,
        PositionType::Absolute,
        "the inspect panel root is an absolute overlay (contributes nothing to the viewport inset)",
    );
}

#[test]
fn inspect_panel_root_width_is_a_fixed_viewport_fraction() {
    let node = spawn_root_node();
    assert!(
        node.is_some(),
        "spawn_inspect_panel must spawn an InspectPanelRoot"
    );
    let Some(node) = node else {
        return;
    };
    // FIXED-% width in viewport-width units (`Val::Vw`) — so the map area never pops/shifts
    // left-right when the panel's contents change. Assert the unit KIND, not the magnitude.
    assert!(
        matches!(node.width, Val::Vw(_)),
        "the inspect panel width must be a fixed viewport-width fraction (Val::Vw), got {:?}",
        node.width,
    );
    // The height is capped in viewport-height units so a tall block can never overrun the map.
    assert!(
        matches!(node.max_height, Val::Vh(_)),
        "the inspect panel max-height must be a viewport-height fraction (Val::Vh), got {:?}",
        node.max_height,
    );
}

#[test]
fn inspect_panel_root_is_anchored_top_right() {
    let node = spawn_root_node();
    assert!(
        node.is_some(),
        "spawn_inspect_panel must spawn an InspectPanelRoot"
    );
    let Some(node) = node else {
        return;
    };
    // Anchored to the TOP-RIGHT corner: top + right are set (relative units), left + bottom
    // are left auto. The kind of `top`/`right` is a viewport fraction (Vh/Vw), not auto.
    assert!(
        matches!(node.top, Val::Vh(_)),
        "the inspect panel is anchored to the top (Val::Vh), got {:?}",
        node.top,
    );
    assert!(
        matches!(node.right, Val::Vw(_)),
        "the inspect panel is anchored to the right (Val::Vw), got {:?}",
        node.right,
    );
    assert_eq!(
        node.left,
        Val::Auto,
        "the top-right anchor leaves `left` auto (not a left anchor)",
    );
}
