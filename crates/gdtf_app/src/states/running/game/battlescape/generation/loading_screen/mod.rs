mod components;
mod plugin;
mod spawn;

pub(in crate::states::running::game::battlescape::generation) use plugin::LoadingScreenPlugin;

#[cfg(feature = "test-support")]
pub(crate) mod test_support {
    pub use super::components::LoadingScreenRoot;
}

#[cfg(all(debug_assertions, feature = "net_qa"))]
mod capture;
