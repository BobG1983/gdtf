//! Headless app with a real primary window.

/// Windowed test app builder.
pub mod app_builder;
#[cfg(test)]
mod test;

pub use app_builder::{PINNED_DELTA, WindowedTestAppBuilder};
