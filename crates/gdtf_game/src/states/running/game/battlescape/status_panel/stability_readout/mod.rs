mod components;
mod spawn;
mod update;

pub(in crate::states::running::game::battlescape::status_panel) use spawn::spawn_stability_readout;
pub(in crate::states::running::game::battlescape::status_panel) use update::update_stability_readout;

#[cfg(feature = "headless_test")]
pub(crate) mod test_support {
    pub use super::components::StabilityBar;
}
