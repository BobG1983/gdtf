use bevy::{
    asset::AssetServer,
    prelude::{Node, default},
    ui::ComputedNode,
};

use super::*;

#[test]
fn ui_layout_runs_and_asset_server_present_headless() {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();

    // A node with an explicit pixel size: once `bevy_ui` runs its layout, the
    // entity gains a `ComputedNode` whose `size()` resolves to this extent.
    // Without `UiPlugin` (i.e. if the headless render/ui stack were not wired)
    // no `ComputedNode` would ever appear, so this assertion fails if the
    // harness regresses to a non-UI plugin set — the pin-discriminating check.
    let node = app
        .world_mut()
        .spawn(Node {
            width: bevy::ui::Val::Px(120.0),
            height: bevy::ui::Val::Px(80.0),
            ..default()
        })
        .id();

    // A few frames so the asset + render schedules initialise and `bevy_ui`'s
    // layout system computes the node.
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
