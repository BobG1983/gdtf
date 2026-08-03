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

type SpawnParams<'w, 's> = SystemState<(
    Commands<'w, 's>,
    Option<Res<'w, gdtf_ui::theme::GdtfTheme>>,
    Option<Res<'w, gdtf_battle_presenter::TopDownAtlases>>,
)>;

fn spawn_root_node() -> Option<Node> {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.insert_resource(default_theme());
    let world = app.world_mut();

    let mut state: SpawnParams = SystemState::new(world);
    let Ok((commands, theme, atlases)) = state.get(world) else {
        return None;
    };
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
    assert!(
        matches!(node.width, Val::Vw(_)),
        "the inspect panel width must be a fixed viewport-width fraction (Val::Vw), got {:?}",
        node.width,
    );
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
