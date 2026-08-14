use bevy::{
    app::App,
    winit::{UpdateMode, WinitSettings},
};

use crate::present::CapturePresentPlugin;

#[test]
fn winit_settings_are_continuous_on_both_modes() {
    let mut app = App::new();
    app.add_plugins(CapturePresentPlugin);
    let settings = app.world().get_resource::<WinitSettings>();
    assert!(
        settings.is_some_and(|winit| winit.focused_mode == UpdateMode::Continuous
            && winit.unfocused_mode == UpdateMode::Continuous),
        "CapturePresentPlugin must set WinitSettings to Continuous in both focus states, got \
         {settings:?}",
    );
}
