mod ui_camera;
#[cfg(any(
    debug_assertions,
    all(feature = "dev_tools", not(feature = "headless_test"))
))]
pub(crate) use ui_camera::UiCamera;
pub(in crate::states::running) use ui_camera::spawn_ui_camera;
