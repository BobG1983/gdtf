//! GTW-918 clause 8: an editor that is not running the QA listener renders exactly as before.
//!
//! ## The feature-off half of the clause is a COMPILE-TIME fact, not a runtime one
//!
//! `crate::net_qa` — and therefore this whole present path — exists only under
//! `cfg(all(debug_assertions, feature = "net_qa"))` (the gate is applied at the module
//! declaration in `crate::lib` and at the wiring site in `crate::app`). A build without the
//! feature contains no [`EditorCapturePresentPlugin`] to add, so there is no runtime state a
//! test could observe: the assertion is that this file does not compile at all in that
//! configuration, which the feature-off `cargo dtest`/`dclippy` runs make every time.
//!
//! What IS observable, and what the test below pins, is the feature-ON, env-OFF half: the plugin
//! compiled in, constructed the way the binary constructs it, and inert.

use bevy::{app::App, camera::RenderTarget, prelude::*, winit::WinitSettings};

use crate::net_qa::{
    NetQaEditorPlugin,
    env::editor_net_qa_enabled,
    present::{blit::EditorQaPresentCamera, target::EditorQaCaptureTarget},
};

/// A `net_qa` editor whose env gate is OFF wires NO present path: no offscreen capture target,
/// no present camera, and no [`WinitSettings`] override.
///
/// Built through [`NetQaEditorPlugin::from_env`] — the exact constructor `crate::app` uses — so
/// this is the shipped inert configuration, not a stand-in. The env gate cannot be driven from a
/// test (`std::env::set_var` is unsafe in edition 2024 and the workspace forbids `unsafe`), so
/// the assertion is made only when the ambient gate is genuinely off; a run WITH
/// `GDTF_EDITOR_NET_QA` set is asserting the opposite arm, which the present-path tests already
/// cover, so it is skipped rather than inverted into a false pass.
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
