//! Launch recipes: package, features, and QA channel.

pub mod channel;
pub mod spec;
pub mod values;

pub use channel::McpChannel;
pub use spec::LaunchSpec;
pub use values::{CargoPackage, CargoProfile, EnvVarName, FeatureList, FeatureName, WorkingDir};
