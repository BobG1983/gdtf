mod material;
mod present;
mod shadow;

#[cfg(test)]
mod test;

pub(crate) use material::Saturation;
pub use material::{Brightness, TerrainFogMaterial, TerrainFogUniform};
pub use present::present_fog;
pub use shadow::{ShownSquadVisibility, promote_shown_fog};
