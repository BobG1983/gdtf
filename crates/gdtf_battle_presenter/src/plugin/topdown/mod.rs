//! The top-down renderer's registration, split per concern: the plugin shell
//! (`plugin`) plus one registrar module per draw concern (terrain / gangers / fx /
//! camera / fog / overlays).

mod camera;
mod fog;
mod fx;
mod gangers;
mod overlays;
mod plugin;
mod terrain;

pub use plugin::{TopDownRendererActive, TopDownRendererPlugin};
