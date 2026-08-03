//! Launch recipes: package, features, env, and QA channel.

pub mod channel;
pub mod spec;
pub mod values;

pub use channel::QaChannel;
pub use spec::LaunchSpec;
pub use values::{
    CargoPackage, EnvOverrides, EnvVar, EnvVarName, EnvVarValue, FeatureList, FeatureName,
    WorkingDir,
};
