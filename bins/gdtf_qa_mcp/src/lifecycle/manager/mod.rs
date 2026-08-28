//! Host process manager: launch, wait for readiness, stop.

pub mod host;
pub mod lifecycle;
pub mod port;
#[cfg(test)]
mod test;

pub use host::HostManager;
pub use lifecycle::HostLifecycle;
