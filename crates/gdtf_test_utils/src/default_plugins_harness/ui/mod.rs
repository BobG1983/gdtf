//! Headless UI-layout + asset test harness for the GDTF Bevy app.
//!
//! See [`app_builder`] for the [`GdtfUiTestAppBuilder`] type-state builder and
//! the `no_renderer.rs` headless plugin set it composes.

mod app_builder;
#[cfg(test)]
mod test;

pub use app_builder::{GdtfUiTestAppBuilder, NoCamera, WithCamera};
