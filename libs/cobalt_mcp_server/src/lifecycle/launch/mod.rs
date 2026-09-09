//! Launch recipes: package, features, profile and working directory.

pub mod spec;
pub mod values;

pub use spec::LaunchSpec;
pub use values::{CargoPackage, CargoProfile, FeatureList, FeatureName, WorkingDir};
