mod ui_camera;
// GTW-764: the UI-camera marker, re-exported crate-wide (through `running/mod.rs`) so the
// DEV-ONLY `net_qa` offscreen-capture retarget system can query it. The second consumer is
// `crate::dev::egui_context` under `dev_tools` (it binds the primary egui context to this
// camera, and compiles out under `test-support` with the `EguiPlugin` add it belongs to), so
// the gate is the UNION of the two consumers' gates — outside both the re-export has no
// consumer and would be an unused `pub(crate) use`.
#[cfg(any(
    all(debug_assertions, feature = "net_qa"),
    all(feature = "dev_tools", not(feature = "test-support"))
))]
pub(crate) use ui_camera::UiCamera;
pub(in crate::states::running) use ui_camera::spawn_ui_camera;
