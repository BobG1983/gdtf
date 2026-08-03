pub mod channel;
pub mod spec;
pub mod values;

#[cfg(test)]
mod test;

pub use channel::QaChannel;
pub use spec::LaunchSpec;
pub use values::{
    CargoPackage, EnvOverrides, EnvVar, EnvVarName, EnvVarValue, FeatureList, FeatureName,
    WorkingDir,
};
