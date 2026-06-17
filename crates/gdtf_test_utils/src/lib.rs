//! Headless test harnesses for the GDTF Bevy app.
//!
//! Two type-state builders are provided:
//!
//! - [`GdtfTestAppBuilder`] — a `MinimalPlugins` app wired with the real GDTF
//!   state machine, for state-machine / system-effect tests (no renderer, no
//!   asset stack, no UI layout).
//! - [`GdtfUiTestAppBuilder`] — a `DefaultPlugins` app in the official
//!   `no_renderer.rs` headless configuration (no GPU, no window), for tests that
//!   need real `bevy_ui` **layout geometry** ([`bevy::ui::ComputedNode`]) or a
//!   live [`bevy::asset::AssetServer`]. See [`ui`](crate::GdtfUiTestAppBuilder).
//!
//! [`GdtfTestAppBuilder`] is a **type-state** builder over [`bevy::app::App`]:
//! it wires the real GDTF state machine (via
//! [`gdtf_app::test_support::register_headless`]) onto a minimal, deterministic
//! headless app, and the type parameter `Phase` makes
//! [`build`](GdtfTestAppBuilder::build) *unreachable* until an initial
//! [`AppState`](gdtf_app::test_support::AppState) has been chosen. There is no
//! fallible step — `build()` is infallible by construction, so the harness never
//! needs `unwrap`/`expect`/`panic`.
//!
//! Determinism comes from [`bevy::time::TimeUpdateStrategy::FixedTimesteps`]: one
//! `App::update()` runs the fixed-update loop exactly once, so state transitions
//! advance at a fixed, repeatable rate.
//!
//! This example is `ignore`d by the doctest harness: the dev/gate green suite
//! enables Bevy `dynamic_linking`, under which rustdoc's merged-doctest runner
//! cannot load `libstd.dylib` and every doctest in the crate fails to launch
//! (a known `dynamic_linking` dyld limitation, not a defect in this code). The
//! behavior the example shows is verified for real — and compile-checked — by
//! [`GdtfTestAppBuilder`]'s unit test and `gdtf_app`'s `state_walk` integration
//! tests, so the snippet stays purely illustrative here.
//!
//! ```ignore
//! use bevy::state::state::State;
//! use gdtf_app::test_support::{AppState, RunningState};
//! use gdtf_test_utils::GdtfTestAppBuilder;
//!
//! // `build()` only exists once an initial state is selected — this is the
//! // whole point of the type-state: a missing initial state is a *compile*
//! // error, not a runtime panic.
//! let mut app = GdtfTestAppBuilder::new()
//!     .starting_in(AppState::Running)
//!     .build();
//! app.update();
//!
//! assert_eq!(
//!     app.world().resource::<State<AppState>>().get(),
//!     &AppState::Running
//! );
//! assert_eq!(
//!     app.world().resource::<State<RunningState>>().get(),
//!     &RunningState::Menu,
//! );
//! ```

mod advance;
mod builder;
mod load;
mod ui;

pub use advance::advance_until;
pub use builder::{GdtfTestAppBuilder, NoState, WithState};
pub use load::GdtfLoadTestAppBuilder;
pub use ui::{GdtfUiTestAppBuilder, NoCamera, WithCamera};
