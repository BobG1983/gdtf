//! Shared test helpers for GDTF crates: harness builders, input, probes, and GPU checks.

/// Frame advance helpers.
pub mod advance;
/// Headless apps with a full DefaultPlugins stack.
pub mod default_plugins_harness;
/// GPU adapter presence probe for skip-or-run decisions.
pub mod gpu_probe;
/// Synthetic keyboard, mouse, and UI input.
pub mod input;
/// MinimalPlugins harness builders and battle scenarios.
pub mod minimal_harness;
/// Message capture probes.
pub mod probe;
/// Bevy state inspection.
pub mod state;

pub use advance::{advance_until, advance_until_load_state, advance_until_resource_exists};
pub use default_plugins_harness::{
    load::GdtfLoadTestAppBuilder,
    ui::{GdtfUiTestAppBuilder, NoCamera, WithCamera},
};
pub use gpu_probe::{
    FORCE_NO_GPU_ENV, GpuAdapterProbe, gpu_adapter_probe, gpu_adapter_probe_forced,
};
pub use input::{clear_keys, clear_mouse, press_key, press_left, press_mouse, press_ui_button};
pub use minimal_harness::{
    builder::{GdtfTestAppBuilder, NoState, WithState},
    scenarios::BattleAppBuilder,
};
pub use probe::{MessageProbe, MessageProbePlugin, drain_message_probe, probed};
pub use state::current_state;
