//! The launch recipe — what one launch runs, as typed values (GTW-875).
//!
//! - [`values`] — the newtypes a recipe is written in: [`CargoPackage`], [`FeatureName`] /
//!   [`FeatureList`], [`WorkingDir`], and [`EnvVar`] / [`EnvOverrides`].
//! - [`spec`] — the [`LaunchSpec`] aggregate the spawner reads, plus the default game
//!   recipe.

pub mod spec;
pub mod values;

#[cfg(test)]
mod test;

pub use spec::LaunchSpec;
pub use values::{
    CargoPackage, EnvOverrides, EnvVar, EnvVarName, EnvVarValue, FeatureList, FeatureName,
    WorkingDir,
};
