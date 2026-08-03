use bevy::{
    asset::AssetServer,
    prelude::{Node, default},
    ui::ComputedNode,
};

use super::*;

#[test]
fn ui_layout_runs_and_asset_server_present_headless() {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();

    let node = app
        .world_mut()
        .spawn(Node {
            width: bevy::ui::Val::Px(120.0),
            height: bevy::ui::Val::Px(80.0),
            ..default()
        })
        .id();

    for _ in 0..3 {
        app.update();
    }

    let computed = app.world().get::<ComputedNode>(node);
    assert!(
        computed.is_some(),
        "spawned Node should have received a ComputedNode — proves the bevy_ui \
         layout machinery ran under the headless harness",
    );

    if let Some(computed) = computed {
        let size = computed.size();
        assert!(
            (size.x - 120.0).abs() < 0.5 && (size.y - 80.0).abs() < 0.5,
            "ComputedNode size {size:?} should resolve to the node's explicit \
             120x80 px extent — proves layout was actually computed, not just \
             the component inserted",
        );
    }

    assert!(
        app.world().get_resource::<AssetServer>().is_some(),
        "AssetServer resource should be present — proves the asset machinery \
         is wired in the headless harness",
    );
}
