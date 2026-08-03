//! Shooter stability score and the cone / recoil multipliers it produces.

mod curve;
mod gate;
mod score;
pub mod terrain_brace;
mod types;

#[cfg(test)]
mod test;

pub use score::stability;
pub use terrain_brace::TerrainBraced;
pub use types::{
    ConeMult, EmplacementStability, RecoilGrowth, StabilityScore, StabilityTerms,
    SuppressionStability,
};
