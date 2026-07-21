mod ui_camera;
// GTW-764: the UI-camera marker, re-exported crate-wide (through `running/mod.rs`) so the
// DEV-ONLY `net_qa` offscreen-capture retarget system can query it. cfg-gated to `net_qa` —
// the re-export has no consumer without the feature.
#[cfg(all(debug_assertions, feature = "net_qa"))]
pub(crate) use ui_camera::UiCamera;
pub(in crate::states::running) use ui_camera::spawn_ui_camera;
