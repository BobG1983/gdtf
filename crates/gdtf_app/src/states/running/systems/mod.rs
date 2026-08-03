mod ui_camera;
#[cfg(any(
    all(debug_assertions, feature = "net_qa"),
    all(feature = "dev_tools", not(feature = "test-support"))
))]
pub(crate) use ui_camera::UiCamera;
pub(in crate::states::running) use ui_camera::spawn_ui_camera;
