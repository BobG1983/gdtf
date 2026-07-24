mod ui_camera;
// GTW-764: the UI-camera marker, re-exported crate-wide (through `running/mod.rs`) so the
// DEV-ONLY `net_qa` offscreen-capture retarget system can query it. GTW-819 adds a second
// consumer under `dev_tools` (the UI-stack coexistence spike binds the primary egui context to
// this camera), so the gate is the UNION of the two consumers' gates — without either feature
// the re-export has no consumer and would be an unused `pub(crate) use`.
#[cfg(any(all(debug_assertions, feature = "net_qa"), feature = "dev_tools"))]
pub(crate) use ui_camera::UiCamera;
pub(in crate::states::running) use ui_camera::spawn_ui_camera;
