//! Keep the window updating while it is unfocused, so a capture is never stale.

mod plugin;

#[cfg(test)]
mod test;

pub use plugin::CapturePresentPlugin;
