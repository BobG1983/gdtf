//! The `MinimalPlugins` type-state app builder for state-machine tests.

mod app_builder;
#[cfg(test)]
mod test;

pub use app_builder::{GdtfTestAppBuilder, NoState, WithState};
