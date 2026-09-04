//! UI test app builder with optional camera.

mod app_builder;
#[cfg(test)]
mod test;

pub use app_builder::{NoCamera, UiTestAppBuilder, WithCamera};
