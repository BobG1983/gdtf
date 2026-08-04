//! Present command stays inert when net-QA is not active.
use bevy::{app::App, camera::RenderTarget, prelude::*, winit::WinitSettings};

use crate::net_qa::{
    NetQaEditorPlugin,
    env::editor_net_qa_enabled,
    present::{blit::EditorQaPresentCamera, target::EditorQaCaptureTarget},
};

#[test]
fn an_env_gated_off_editor_wires_no_present_path() {
    if editor_net_qa_enabled() {
        return;
    }
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(NetQaEditorPlugin::from_env());
    for _ in 0..4 {
        app.update();
    }

    assert!(
        app.world().get_resource::<WinitSettings>().is_none(),
        "an inert editor must not override WinitSettings — its power behaviour is untouched",
    );
    assert!(
        app.world()
            .get_resource::<EditorQaCaptureTarget>()
            .is_none(),
        "an inert editor must create no offscreen capture target",
    );
    let mut present = app
        .world_mut()
        .query_filtered::<&RenderTarget, With<EditorQaPresentCamera>>();
    let count = present.iter(app.world()).count();
    assert!(
        count == 0,
        "an inert editor must spawn no present camera, found {count}",
    );
}
