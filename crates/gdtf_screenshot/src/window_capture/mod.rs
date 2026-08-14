//! Capture the window's own frame by overriding its output color attachment.

mod attachment;
mod blit;
mod plugin;
mod png;
mod shot;
mod target;

#[cfg(test)]
mod test;

pub use plugin::WindowCapturePlugin;
pub(crate) use shot::CaptureShot;
pub use target::CaptureImage;
