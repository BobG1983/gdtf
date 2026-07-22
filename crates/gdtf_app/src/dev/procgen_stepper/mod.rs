//! The DEV-ONLY procgen load-time stepper (GTW-655): an egui Next/Auto/Skip overlay over
//! the sim's staged procgen driver, shown at every battle load while active. Replaces the
//! GTW-434 menu-invoked standalone procgen-visualizer scene (retired in the same change).
//!
//! See [`plugin`] for the wiring rationale and the module-doc walk-through of the seam.

mod commands;
mod drive;
mod gate;
mod plugin;
// The pure schematic-overview painter `ui` draws (GTW-732) — NOT gated on `test-support` so its
// geometry compiles + unit-tests there too, unlike `ui` itself (which needs a primary egui
// context the headless harness lacks — see `schematic`'s module doc).
mod schematic;
// The egui panel itself needs a primary window — excluded from `test-support` builds (the
// headless integration test's `no_renderer.rs`-style app has none), mirroring the
// content-editor's own headless harness (see `plugin`'s module doc). The real binary never
// enables `test-support`, so this never changes shipped wiring.
#[cfg(not(feature = "test-support"))]
mod ui;
// Pins the primary egui context to the captured UI camera (GTW-732) so the schematic lands in
// the `net_qa` offscreen capture rather than the headless window — see the module doc. Same
// `test-support` exclusion as `ui` (both name `bevy_egui` context types the headless harness's
// egui-less build lacks).
#[cfg(not(feature = "test-support"))]
mod egui_context;

// `ProcgenStepperPlugin` is the only item the binary consumes (via the dev aggregate
// plugin, `crate::dev::plugin`), so it is re-exported in BOTH configurations, at the same
// `test-support` visibility flip the type itself uses (`support_item!` in `plugin`) —
// mirrors the `net_qa` plugin's own `support_item!` re-export precedent exactly.
crate::support_use!(plugin::ProcgenStepperPlugin;);

// `battle_setup_runs_directly` is consumed by `battle_sim::plugin` (the run condition that
// gates `request_battle_setup` off while the stepper is active) — a PRODUCTION consumer
// outside this module, so it stays `pub(crate)` unconditionally (not test-support-gated).
// The env-var gate reader + `AutoStepDelay` are consumed ONLY by the GTW-655 integration test
// (which drives the ENGAGED path directly, bypassing egui — the egui closure never runs
// headlessly — and unit-checks the gate mirrors `from_env`, the house recognised-truthy
// convention), so they widen to `pub` only under `test-support`.
#[cfg(feature = "test-support")]
pub use commands::AutoStepDelay;
// The command/latch TRIO (`StepCommand` / `PendingStepCommand` / `AutoRunning`) has a SECOND
// out-of-module consumer since GTW-766: the `net_qa` stepper-drive dispatch
// (`crate::dev::net_qa::stepper`) writes the same latch the egui panel writes. So the re-export
// exists under EITHER feature — `support_use!` widens it to `pub` under `test-support` (the
// `test_support` ledger needs it) and `pub(crate)` otherwise (the net_qa dispatch names it
// crate-wide). Gated on `any(...)` (not unconditional) so a plain `dev_tools`-only build stays
// warning-clean: the trio is reached in-crate via `super::commands::…` (drive / ui), so an
// unconditional re-export would be an unused `pub(crate) use`.
#[cfg(any(feature = "test-support", feature = "net_qa"))]
crate::support_use!(commands::{AutoRunning, PendingStepCommand, StepCommand};);
pub(crate) use gate::battle_setup_runs_directly;
#[cfg(feature = "test-support")]
pub use gate::stepper_enabled;
// The schematic painter (GTW-732) — re-exported under `test-support` so it stays a reachable
// public API item (keeping its geometry linked + unit-tested even when `ui`, the only in-crate
// caller, is excluded from headless builds).
#[cfg(feature = "test-support")]
pub use schematic::draw_schematic;
