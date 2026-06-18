//! The DEV-ONLY screenshot / in-engine visual-QA affordance (GTW-297). The gate, the
//! capture config, the plugin, and the capture system live in the `plugin` submodule;
//! see it for the full rationale and the exact invocation. Tests live in the sibling
//! `test` submodule.
//!
//! ## Invocation
//!
//! Drive the app into a live battle with the GTW-223 auto-battle affordance, point the
//! capture at an absolute PNG path, and build with the `dev_capture` feature (a debug
//! build — the affordance is double-gated on `debug_assertions`):
//!
//! ```text
//! GDTF_AUTOBATTLE=1 GDTF_CAPTURE_PATH=/abs/out.png \
//!   cargo run -p grimdark_turfwar \
//!   --features "grimdark_turfwar/dynamic_linking,gdtf_app/dev_capture"
//! ```
//!
//! Optional: `GDTF_CAPTURE_FRAME=<n>` overrides how many `BattleRunning` frames to wait
//! before capturing (default 15). The PNG lands at `GDTF_CAPTURE_PATH` and the app then
//! exits on its own; `Read` the PNG to verify the HUD.

mod plugin;

// `DevCapturePlugin` is the only item the binary consumes (via `gdtf_app.rs`). Plain
// `pub(crate)` re-export: nothing OUTSIDE the crate names the capture items (the config
// tests are the in-crate `#[cfg(test)]` sibling), so it stays `unreachable_pub`-clean
// without the `test-support` visibility flip the `auto_battle` affordance needs.
pub(crate) use plugin::DevCapturePlugin;

#[cfg(test)]
mod test;
