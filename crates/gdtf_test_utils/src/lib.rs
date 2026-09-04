//! Shared test helpers for GDTF crates: harness builders, input, probes, and GPU checks.
//!
//! The engine-level half lives in `cobalt_test_utils` and is re-exported here, so a caller
//! keeps one import path.

/// Headless apps with a full DefaultPlugins stack.
pub mod default_plugins_harness;
/// MinimalPlugins harness builders and battle scenarios.
pub mod minimal_harness;

pub use cobalt_test_utils::{
    MessageProbe, MessageProbePlugin, NoCamera, UiTestAppBuilder, WindowedTestAppBuilder,
    WithCamera, advance,
    advance::{advance_until, advance_until_load_state, advance_until_resource_exists},
    clear_keys, clear_mouse, current_state, drain_message_probe,
    gpu_probe::{
        self, FORCE_NO_GPU_ENV, GpuAdapterProbe, gpu_adapter_probe, gpu_adapter_probe_forced,
    },
    input, press_key, press_left, press_mouse, press_ui_button, probe, probed, state,
};
pub use default_plugins_harness::load::GdtfLoadTestAppBuilder;
pub use minimal_harness::{
    builder::{GdtfTestAppBuilder, NoState, WithState},
    scenarios::BattleAppBuilder,
};
