//! The Options screen's systems (GTW-637): spawn, the Continue action, the sound
//! toggle adapter, and the widget theming.
//!
//! Wiring only (module-layout rule 2): submodule declarations and the re-exports
//! the scene plugin and the spawn path name. Each concern lives in its own leaf —
//! [`spawn`] builds the tree, [`actions`] bridges the Continue button's input to an
//! `On<Activate>` observer that changes state, [`settings_input`] is the ONE widget
//! adapter (checkbox native `ValueChange` → typed intent → settings → label), and
//! [`theming`] paints/repaints the checkbox toggle from the theme + setting.

mod actions;
mod settings_input;
mod spawn;
pub(in crate::states::running::options) mod theming;

pub(in crate::states::running::options) use actions::{
    bridge_continue_activation, continue_activated,
};
pub(in crate::states::running::options) use settings_input::{
    apply_sound_setting, sound_activated, sync_sound_value_label,
};
pub(in crate::states::running::options) use spawn::{clear_options_nav_map, spawn_options_screen};
pub(in crate::states::running::options) use theming::paint_sound_toggle;

#[cfg(test)]
mod screenshot;
#[cfg(test)]
mod test;
