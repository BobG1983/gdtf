//! Probe whether a GPU adapter is available for real-GPU tests.

use bevy::tasks::block_on;
use wgpu::{Instance, RequestAdapterOptions};

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

/// Probe for a GPU adapter, asking the machine rather than any ambient setting.
#[must_use]
pub fn gpu_adapter_probe() -> GpuAdapterProbe {
    gpu_adapter_probe_forced(false)
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
    use super::{GpuAdapterProbe, gpu_adapter_probe_forced};

    #[test]
    fn force_no_gpu_hook_reports_absent_without_panicking() {
        let verdict = gpu_adapter_probe_forced(true);
        assert_eq!(
            verdict,
            GpuAdapterProbe::Absent,
            "the injected force-absent flag must make the probe report Absent (the skip path)",
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
}
