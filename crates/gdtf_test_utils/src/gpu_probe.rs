//! A NON-panicking wgpu-adapter probe for the real-GPU render-to-texture tests
//! (GTW-527).
//!
//! The presenter's `*_readback.rs` pixel-proof tests build a Bevy render `App` with
//! the DEFAULT `RenderPlugin` render creation — a REAL wgpu adapter, not the
//! `WgpuSettings { backends: None }` config the non-readback headless draw tests use.
//! On a GPU-less runner (`ubuntu-latest` with no GPU and no lavapipe/llvmpipe software
//! driver) there is no adapter, so `RenderPlugin::finish()` —
//! `initialize_renderer` → `request_adapter().expect("Unable to find a GPU!")` — PANICS
//! inside `app.finish()`. That panic runs BEFORE each test's own
//! `app.get_sub_app(RenderApp)?` no-adapter skip guard, so the intended graceful skip
//! was DEAD CODE on GPU-less CI (it passed only where a GPU is always present, e.g.
//! macOS/Metal).
//!
//! This probe closes that gap: it asks wgpu whether a usable adapter exists BEFORE any
//! render `App` is built, WITHOUT building (and therefore without `finish`-ing) the app,
//! and WITHOUT panicking when the answer is "none". A readback test calls
//! [`gpu_adapter_probe`] as its FIRST step and, on [`GpuAdapterProbe::Absent`], logs a
//! SKIP line and returns — so `app.finish()` (and its `expect`) is never reached on a
//! GPU-less runner, while a GPU machine still runs the full real-GPU proof unchanged.
//!
//! It uses the SAME `wgpu` the app uses: `gdtf_test_utils` depends on `wgpu` pinned to
//! the workspace `Cargo.lock` version (the version `bevy_render` links), and Cargo
//! unifies the two into ONE crate instance — so the probe's adapter query mirrors the
//! query `bevy_render::renderer::initialize_renderer` runs.

use bevy::tasks::block_on;
use wgpu::{Instance, RequestAdapterOptions};

/// The env var that FORCES the probe to report [`GpuAdapterProbe::Absent`] without
/// touching wgpu — the deterministic skip-path test hook (GTW-527 C4).
///
/// A CI-green build has no GPU to reproduce the skip on, so this hook lets a GPU
/// machine drive the exact absent-adapter branch the guard takes on a GPU-less runner,
/// proving the guard returns cleanly (no `app.finish()` panic) rather than proceeding.
/// Any value except unset / empty / `"0"` / `"false"` (case-insensitive) counts as ON.
pub const FORCE_NO_GPU_ENV: &str = "GDTF_TEST_FORCE_NO_GPU";

/// The verdict of a non-panicking GPU-adapter probe: whether a usable wgpu adapter
/// exists in this environment.
///
/// A named verdict rather than a bare `bool` — the two cases carry the domain meaning
/// "run the real-GPU proof" vs "skip it, no adapter". Construct it only via
/// [`gpu_adapter_probe`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GpuAdapterProbe {
    /// A usable wgpu adapter was found — the caller SHOULD build the render `App` and
    /// run the full real-GPU proof.
    Present,
    /// No usable wgpu adapter (a GPU-less runner, or the [`FORCE_NO_GPU_ENV`] hook is
    /// on) — the caller MUST skip: log a note and return BEFORE building/finishing the
    /// render `App`.
    Absent,
}

impl GpuAdapterProbe {
    /// Whether a usable adapter is present (`true` only for [`GpuAdapterProbe::Present`]).
    #[must_use]
    pub const fn is_present(self) -> bool {
        matches!(self, Self::Present)
    }

    /// Whether the caller must SKIP the real-GPU proof (`true` for
    /// [`GpuAdapterProbe::Absent`]).
    #[must_use]
    pub const fn should_skip(self) -> bool {
        matches!(self, Self::Absent)
    }
}

/// Whether a [`FORCE_NO_GPU_ENV`] value string switches the hook ON. Unset (modelled as
/// `None`) / empty / `"0"` / `"false"` (case-insensitive, whitespace-trimmed) all read as
/// OFF; any other value reads as ON. Pure (takes the value, reads no env) so it is
/// unit-testable without mutating the process environment (the workspace denies
/// `unsafe-code`, and edition-2024 `std::env::set_var` is `unsafe`).
fn force_value_is_on(value: Option<&str>) -> bool {
    match value {
        Some(raw) => {
            let v = raw.trim().to_ascii_lowercase();
            !(v.is_empty() || v == "0" || v == "false")
        }
        None => false,
    }
}

/// Whether the [`FORCE_NO_GPU_ENV`] hook is switched ON in the current process env.
fn force_no_gpu() -> bool {
    force_value_is_on(std::env::var(FORCE_NO_GPU_ENV).ok().as_deref())
}

/// Probe — WITHOUT building a Bevy render `App` and WITHOUT panicking — whether a usable
/// wgpu adapter exists in this environment.
///
/// If the [`FORCE_NO_GPU_ENV`] test hook is on (read from the process env), this returns
/// [`GpuAdapterProbe::Absent`] WITHOUT touching wgpu (GTW-527 C4); otherwise it delegates
/// to [`gpu_adapter_probe_forced`] with the hook OFF to run the real wgpu query.
///
/// This does NOT build a `RenderPlugin` / `App`, so nothing here can reach the
/// `app.finish()` adapter `expect` — that is the whole point: the caller uses the verdict
/// to decide whether to build the app at all.
#[must_use]
pub fn gpu_adapter_probe() -> GpuAdapterProbe {
    gpu_adapter_probe_forced(force_no_gpu())
}

/// The probe core, with the FORCE-no-GPU decision passed in explicitly (rather than read
/// from the process env) so the [`GpuAdapterProbe::Absent`] skip branch is testable
/// WITHOUT mutating the environment (which would need denied `unsafe` in edition 2024).
///
/// When `force_absent` is `true` it returns [`GpuAdapterProbe::Absent`] WITHOUT touching
/// wgpu. Otherwise it instantiates a wgpu [`Instance`] (the SAME `wgpu` version
/// `bevy_render` uses, via Cargo's crate unification) with the platform-default backends
/// and requests an adapter with the default options — mirroring the query
/// `bevy_render::renderer::initialize_renderer` makes just before its
/// `.expect("Unable to find a GPU!")`. The request returns a `Result`;
/// [`GpuAdapterProbe::Present`] means an adapter came back, [`GpuAdapterProbe::Absent`]
/// means none did — the error case is folded into `Absent`, never unwrapped, so the probe
/// cannot panic on absence.
#[must_use]
pub fn gpu_adapter_probe_forced(force_absent: bool) -> GpuAdapterProbe {
    if force_absent {
        return GpuAdapterProbe::Absent;
    }

    // Platform-default backends (Metal/Vulkan/DX12 off the same env wgpu honours),
    // matching the default `WgpuSettings` `bevy_render` builds the real adapter with.
    let instance = Instance::default();

    // Default options: HighPerformance power preference, no compatible surface, no
    // forced fallback — the headless-adapter query. `request_adapter` is async; block on
    // it with bevy's own `block_on` (same bevy version, no extra runtime dep).
    let requested = block_on(instance.request_adapter(&RequestAdapterOptions::default()));

    // Fold BOTH the `Ok(adapter)` and the `Err(RequestAdapterError)` cases into the typed
    // verdict — the error (no adapter satisfied the options) is the GPU-less signal, NOT
    // a panic. `.is_ok()` here can never panic on absence.
    if requested.is_ok() {
        GpuAdapterProbe::Present
    } else {
        GpuAdapterProbe::Absent
    }
}

#[cfg(test)]
mod tests {
    use super::{GpuAdapterProbe, force_value_is_on, gpu_adapter_probe_forced};

    /// GTW-527 C4 — the FORCE-no-GPU decision makes the probe report `Absent` WITHOUT
    /// panicking, so the guarded skip path a readback test takes on a GPU-less runner is
    /// proven clean on a GPU machine: the probe returns a verdict (never panics) and the
    /// guard's early-return branch is exercised. It drives the decision directly (rather
    /// than through the process env) so no denied-`unsafe` `std::env::set_var` is needed;
    /// the env-string parsing that maps the [`FORCE_NO_GPU_ENV`](super::FORCE_NO_GPU_ENV)
    /// value to this bool is covered by [`force_value_maps_env_string_to_on_off`].
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

        // With the hook OFF the probe runs the REAL wgpu query — a non-panicking verdict
        // either way (Present on this GPU machine, Absent on a GPU-less runner). C4 only
        // needs "no panic"; the value is environment-dependent so it is not asserted.
        let _real = gpu_adapter_probe_forced(false);
    }

    /// The env-string → on/off mapping the hook uses: unset / empty / `"0"` / `"false"`
    /// (case-insensitive, whitespace-trimmed) are OFF; any other value is ON.
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
