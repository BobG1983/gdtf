//! GTW-816: the DEV UI-stack swap harness — one comparison panel rendered through either
//! `bevy_ui` or egui, swapped live by a keyboard shortcut or by either stack's own button.
//!
//! The suite runs against the REAL Load flow ([`GdtfLoadTestAppBuilder`], a live
//! `AssetServer` rooted at the workspace `assets/`, headless: no GPU, no window) with the
//! real `UiSwapHarnessPlugin` added, driven to `AppState::Running` — so both stacks are
//! wired the way the running binary wires them, not re-created in the test.
//!
//! Split by concern:
//!
//! - [`harness`] — the shared app build, the readers, and the two input injectors (a real
//!   `KeyboardInput` pair, and the egui pointer events a windowless app cannot get from the
//!   OS).
//! - [`renders`] — that a swap moves the RENDERING: the inactive stack's entities are gone
//!   (no leak) and only the live stack's widget answers a pointer click.
//! - [`keyboard`] — the shortcut flips the live stack in play, and an unrelated key does
//!   not.
//! - [`double_swap`] — the multipass guard: one egui-side click is exactly one swap.
//! - `parity` (additionally `net_qa`-gated) — one identical intent script driven over the
//!   REAL wire into a REAL battle under each stack in turn, asserting identical act-bus and
//!   sim state. It lives HERE rather than in the `net_qa` suite because this is the app
//!   where the egui half is really registered, so its "egui run" is a run with a real egui
//!   pass. Its token-free snapshot projection lives beside it in `token_free`.
//!
//! The rest of the `net_qa` half of the harness — the `SwapUiStack` intent itself and the
//! `query_state` readback — lives in the `net_qa` suite (`tests/net_qa/ui_swap.rs`).
//!
//! `dev_tools`-gated like the module it pins (`cargo test -p gdtf_app --features
//! test-support,dev_tools,net_qa --test ui_swap`): without the feature the whole crate
//! compiles to an empty test binary, so CI's static suite never breaks on it.
#![cfg(feature = "dev_tools")]

mod double_swap;
mod harness;
mod keyboard;
#[cfg(feature = "net_qa")]
mod parity;
mod renders;
#[cfg(feature = "net_qa")]
mod token_free;
