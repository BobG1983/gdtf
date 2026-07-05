//! The DEV-ONLY screenshot / in-engine visual-QA + fire-trigger affordance (GTW-297,
//! extended GTW-306). The gates, the capture / fire config, the plugin, and the systems
//! live in the `plugin` submodule; see it for the full rationale and the exact
//! invocation. Tests live in the sibling `test` submodule.
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
//! Optional env vars (all `dev_capture` debug-build only, inert when unset):
//!
//! - `GDTF_CAPTURE_FRAME=<n>` — how many `BattleRunning` frames to wait before capturing
//!   ONE frame (default 15). The PNG lands at exactly `GDTF_CAPTURE_PATH`.
//! - `GDTF_CAPTURE_FRAMES="12,14,16,18"` — capture a frame SEQUENCE in one run (GTW-306);
//!   wins over `GDTF_CAPTURE_FRAME`. One PNG per frame, `.fNN`-tagged
//!   (`out.f12.png`, `out.f14.png`, …); the app exits after the last.
//! - `GDTF_FIRE_AT_FRAME=<n>` — at `BattleRunning` frame `n` the selected player ganger
//!   fires at the nearest enemy via the REAL fire path (GTW-306), so the captured
//!   sequence records a live shot's FX.
//! - `GDTF_FIRE_MODE=<single|burst|full>` — fire the triggered shot in this authored mode
//!   (read off the shooter's `FireMode` selector) rather than the resident
//!   `SelectedFireMode` (GTW-306). `full` / `burst` produce a MULTI-ROUND volley the FX
//!   stagger spreads out — the path to capture a staggered volley.
//!
//! Example FX QA run — fire a FULL-AUTO volley at frame 8, capture the staggered travel +
//! impacts across the volley window:
//!
//! ```text
//! GDTF_AUTOBATTLE=1 GDTF_CAPTURE_PATH=/abs/shot.png \
//!   GDTF_FIRE_AT_FRAME=8 GDTF_FIRE_MODE=full \
//!   GDTF_CAPTURE_FRAMES="10,14,18,22,26,30" \
//!   cargo run -p grimdark_turfwar \
//!   --features "grimdark_turfwar/dynamic_linking,gdtf_app/dev_capture"
//! ```
//!
//! `Read` the PNG(s) to verify the HUD / FX; the app exits on its own.

mod capture_config;
mod plugin;
mod screenshot;
mod trigger_config;
mod triggers;

// `DevCapturePlugin` is the only item the binary consumes (via `gdtf_app.rs`). Plain
// `pub(crate)` re-export: nothing OUTSIDE the crate names the capture items (the config
// tests are the in-crate `#[cfg(test)]` sibling), so it stays `unreachable_pub`-clean
// without the `test-support` visibility flip the `auto_battle` affordance needs.
pub(crate) use plugin::DevCapturePlugin;

#[cfg(test)]
mod test;
