mod plugin;
mod systems;
// GTW-582: re-export the cfg(test) tracing-capture scaffold one hop up so
pub(in crate::states) use plugin::LoadScenePlugin;
#[cfg(test)]
pub(crate) use systems::hot_reload_test_support;
mod fallbacks;
#[cfg(feature = "test-support")]
pub use fallbacks::seed_load_fallbacks;
mod resources;
crate::support_use!(resources::LoadedSituation;);
