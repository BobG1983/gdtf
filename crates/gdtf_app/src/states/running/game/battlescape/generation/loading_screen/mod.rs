mod components;
mod plugin;
mod spawn;

pub(in crate::states::running::game::battlescape::generation) use plugin::LoadingScreenPlugin;

#[cfg(feature = "headless_test")]
pub(crate) mod test_support {
    pub use super::components::LoadingScreenRoot;
}
