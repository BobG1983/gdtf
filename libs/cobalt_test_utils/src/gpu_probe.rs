//! Probe whether a GPU adapter is available for real-GPU tests.

use bevy::tasks::block_on;
use wgpu::{Instance, RequestAdapterOptions};

/// Env var that forces the probe to report absent (skip path).
pub const FORCE_NO_GPU_ENV: &str = "COBALT_TEST_FORCE_NO_GPU";

/// Result of a GPU adapter probe.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GpuAdapterProbe {
    /// An adapter was found.
    Present,
    /// No adapter (or forced absent).
    Absent,
}

impl GpuAdapterProbe {
    /// Whether an adapter is present.
    #[must_use]
    pub const fn is_present(self) -> bool {
        matches!(self, Self::Present)
    }

    /// Whether the caller should skip real-GPU tests.
    #[must_use]
    pub const fn should_skip(self) -> bool {
        matches!(self, Self::Absent)
    }
}

fn force_value_is_on(value: Option<&str>) -> bool {
    match value {
        Some(raw) => {
            let v = raw.trim().to_ascii_lowercase();
            !(v.is_empty() || v == "0" || v == "false")
        }
        None => false,
    }
}

fn force_no_gpu() -> bool {
    force_value_is_on(std::env::var(FORCE_NO_GPU_ENV).ok().as_deref())
}

/// Probe for a GPU adapter, honoring [`FORCE_NO_GPU_ENV`].
#[must_use]
pub fn gpu_adapter_probe() -> GpuAdapterProbe {
    gpu_adapter_probe_forced(force_no_gpu())
}

/// Probe for a GPU adapter; when `force_absent` is true, always returns [`GpuAdapterProbe::Absent`].
#[must_use]
pub fn gpu_adapter_probe_forced(force_absent: bool) -> GpuAdapterProbe {
    if force_absent {
        return GpuAdapterProbe::Absent;
    }

    let instance = Instance::default();

    let requested = block_on(instance.request_adapter(&RequestAdapterOptions::default()));

    if requested.is_ok() {
        GpuAdapterProbe::Present
    } else {
        GpuAdapterProbe::Absent
    }
}

#[cfg(test)]
mod tests {
    use super::{GpuAdapterProbe, force_value_is_on, gpu_adapter_probe_forced};

    #[test]
    fn force_no_gpu_hook_reports_absent_without_panicking() {
        let verdict = gpu_adapter_probe_forced(true);
        assert_eq!(
            verdict,
            GpuAdapterProbe::Absent,
            "the FORCE_NO_GPU hook must make the probe report Absent (the skip path)",
        );
        assert!(
            verdict.should_skip(),
            "an Absent verdict must instruct the caller to skip the real-GPU proof",
        );
        assert!(
            !verdict.is_present(),
            "an Absent verdict must not report a present adapter",
        );

        let _real = gpu_adapter_probe_forced(false);
    }

    #[test]
    fn force_value_maps_env_string_to_on_off() {
        for on in ["1", "true", "TRUE", "yes", " on "] {
            assert!(force_value_is_on(Some(on)), "{on:?} must read as ON");
        }
        for off in [
            None,
            Some(""),
            Some("   "),
            Some("0"),
            Some("false"),
            Some("FALSE"),
        ] {
            assert!(!force_value_is_on(off), "{off:?} must read as OFF");
        }
    }
}
