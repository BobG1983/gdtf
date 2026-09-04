#[cfg(feature = "dev_tools")]
pub(crate) mod procgen_stepper;

#[cfg(all(feature = "dev_tools", not(feature = "headless_test")))]
mod egui_context;

#[cfg(feature = "mcp")]
pub(crate) mod mcp;

mod plugin;
pub(crate) use plugin::DevAffordancesPlugin;
