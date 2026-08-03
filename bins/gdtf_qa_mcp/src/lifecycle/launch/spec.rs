use super::{
    channel::QaChannel,
    values::{CargoPackage, EnvOverrides, FeatureList, FeatureName, WorkingDir},
};

const GAME_PACKAGE: &str = "grimdark_turfwar";

const GAME_FEATURES: [&str; 2] = ["dynamic_linking", "net_qa"];

const EDITOR_PACKAGE: &str = "gdtf_content_editor_bin";

const EDITOR_FEATURES: [&str; 2] = ["dynamic_linking", "net_qa"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSpec {
        package:     CargoPackage,
        features:    FeatureList,
        working_dir: Option<WorkingDir>,
        env:         EnvOverrides,
        channel:     QaChannel,
}

impl LaunchSpec {
        #[must_use]
    pub const fn new(
        package: CargoPackage,
        features: FeatureList,
        working_dir: Option<WorkingDir>,
        env: EnvOverrides,
        channel: QaChannel,
    ) -> Self {
        Self {
            package,
            features,
            working_dir,
            env,
            channel,
        }
    }

        #[must_use]
    pub const fn package(&self) -> &CargoPackage {
        &self.package
    }

        #[must_use]
    pub const fn features(&self) -> &FeatureList {
        &self.features
    }

        #[must_use]
    pub const fn working_dir(&self) -> Option<&WorkingDir> {
        self.working_dir.as_ref()
    }

        #[must_use]
    pub const fn env(&self) -> &EnvOverrides {
        &self.env
    }

        #[must_use]
    pub const fn channel(&self) -> &QaChannel {
        &self.channel
    }

                                    #[must_use]
    pub fn resolved_working_dir(&self) -> Option<WorkingDir> {
        self.working_dir
            .clone()
            .or_else(|| std::env::current_dir().ok().map(WorkingDir::new))
    }

                                #[must_use]
    pub fn is_same_launch_as(&self, other: &Self) -> bool {
        self.package == other.package
            && self.features == other.features
            && self.env == other.env
            && self.channel == other.channel
            && self.resolved_working_dir() == other.resolved_working_dir()
    }

                #[must_use]
    pub fn game_default() -> Self {
        Self::new(
            CargoPackage::new(GAME_PACKAGE.to_owned()),
            named_features(&GAME_FEATURES),
            None,
            EnvOverrides::default(),
            QaChannel::game(),
        )
    }

                    #[must_use]
    pub fn editor_default() -> Self {
        Self::new(
            CargoPackage::new(EDITOR_PACKAGE.to_owned()),
            named_features(&EDITOR_FEATURES),
            None,
            EnvOverrides::default(),
            QaChannel::editor(),
        )
    }
}

fn named_features(features: &[&str]) -> FeatureList {
    FeatureList::new(
        features
            .iter()
            .map(|feature| FeatureName::new((*feature).to_owned()))
            .collect(),
    )
}

impl Default for LaunchSpec {
    fn default() -> Self {
        Self::game_default()
    }
}
