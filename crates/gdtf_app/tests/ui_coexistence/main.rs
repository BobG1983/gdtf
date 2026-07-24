//! GTW-819: the UI-stack coexistence proof-of-concept — one `bevy_ui` button and one egui
//! button alive at the same time, in the same running app, in one `AppState`.
//!
//! The suite runs against the REAL Load flow ([`GdtfLoadTestAppBuilder`], a live `AssetServer`
//! rooted at the workspace `assets/`, headless: no GPU, no window) with the real
//! `UiCoexistencePlugin` added, driven to `AppState::Running` — so both stacks are wired the
//! same way the running binary wires them, not re-created in the test.
//!
//! Split by concern:
//!
//! - [`harness`] — the shared app build, the entity/tally readers, and the egui pointer-click
//!   injector (the one piece a windowless headless app cannot get from the OS).
//! - [`alive`] — that both stacks really are up together: `EguiPlugin` added, the primary egui
//!   context bound to the SAME camera `bevy_ui` renders through, and the `bevy_ui` button
//!   entity laid out with a non-zero size at the same time.
//! - [`clicks`] — the load-bearing coexistence assertions: each button's activation produces
//!   its OWN observable effect while both stacks are alive, and neither swallows the other's
//!   click.
//!
//! `dev_tools`-gated like the module it pins (`cargo test -p gdtf_app --features
//! test-support,dev_tools --test ui_coexistence`): without the feature the whole crate
//! compiles to an empty test binary, so CI's static suite never breaks on it.
#![cfg(feature = "dev_tools")]

mod alive;
mod clicks;
mod harness;
