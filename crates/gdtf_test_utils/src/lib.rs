pub mod advance;
pub mod default_plugins_harness;
pub mod gpu_probe;
pub mod input;
pub mod minimal_harness;
pub mod probe;
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
