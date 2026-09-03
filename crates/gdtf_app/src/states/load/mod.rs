mod plugin;
mod systems;
// Re-export the cfg(test) tracing-capture scaffold one hop up so
pub(in crate::states) use plugin::LoadScenePlugin;
#[cfg(test)]
pub(crate) use systems::hot_reload_test_support;
mod fallbacks;
#[cfg(feature = "headless_test")]
pub use fallbacks::seed_load_fallbacks;
mod resources;
