//! Render into an offscreen image, then blit that image back to the window.

mod blit;
mod plugin;
mod target;

#[cfg(test)]
mod test;

pub use blit::{PRESENT_LAYER, PRESENT_ORDER, PresentCamera, PresentSprite};
pub use plugin::{CapturePresentPlugin, PresentSystems};
pub use target::QaCaptureTarget;
