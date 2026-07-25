//! The Options screen's systems (GTW-637): spawn, the Continue action, the setting
//! adapters, and the widget theming.
//!
//! Wiring only (module-layout rule 2): submodule declarations and the re-exports
//! the scene plugin and the spawn path name. Each concern lives in its own leaf —
//! [`spawn`] builds the tree, [`actions`] bridges the Continue button's input to an
//! `On<Activate>` observer that changes state, [`settings_input`] is the sound-toggle
//! adapter (checkbox native `ValueChange` → typed intent → settings → label),
//! [`stepper_setting`] is the same shape for the DEV-ONLY procgen-stepper toggle plus the
//! engagement it drives (`dev_tools` builds only, GTW-868), and [`theming`] paints /
//! repaints the checkbox toggles from the theme + settings.

mod actions;
mod settings_input;
mod spawn;
#[cfg(feature = "dev_tools")]
mod stepper_setting;
pub(in crate::states::running::options) mod theming;

pub(in crate::states::running::options) use actions::{
    bridge_continue_activation, continue_activated,
};
pub(in crate::states::running::options) use settings_input::{
    apply_sound_setting, sound_activated, sync_sound_value_label,
};
pub(in crate::states::running::options) use spawn::{clear_options_nav_map, spawn_options_screen};
#[cfg(feature = "dev_tools")]
pub(in crate::states::running::options) use stepper_setting::{
    apply_stepper_setting, paint_stepper_toggle, stepper_activated, sync_stepper_engagement,
    sync_stepper_value_label,
};
pub(in crate::states::running::options) use theming::paint_sound_toggle;

#[cfg(test)]
mod screenshot;
#[cfg(test)]
mod test;
