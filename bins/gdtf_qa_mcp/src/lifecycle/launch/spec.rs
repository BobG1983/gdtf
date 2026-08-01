//! [`LaunchSpec`] — the whole recipe one launch runs: which package, which features, in
//! which directory, with which extra environment, over which QA channel (GTW-875,
//! GTW-808).
//!
//! Before this existed the recipe was four hardcoded `cargo run` arguments inside
//! `CargoSpawner::spawn`, so no caller could ask for a `dev_tools` build, and every launch
//! built whatever checkout the MCP host itself happened to be running in. The spec is
//! passed per launch, so one running MCP host can drive a plain build, a `dev_tools`
//! build, and a build in a git worktree in turn — and the second target (the editor) is
//! another spec rather than another hardcoded branch, which is exactly how GTW-808 added
//! it.

use super::{
    channel::QaChannel,
    values::{CargoPackage, EnvOverrides, FeatureList, FeatureName, WorkingDir},
};

/// The game package a default launch builds.
const GAME_PACKAGE: &str = "grimdark_turfwar";

/// The features a default game launch enables: fast dev linking plus the QA channel the
/// readiness probe and every forwarding tool depend on.
const GAME_FEATURES: [&str; 2] = ["dynamic_linking", "net_qa"];

/// The editor BINARY package a default editor launch builds (GTW-808).
///
/// The binary crate, not the library: `crates/gdtf_content_editor` has no `[[bin]]`, and
/// only the binary's own `net_qa` passthrough (GTW-878) reaches the library feature.
const EDITOR_PACKAGE: &str = "gdtf_content_editor_bin";

/// The features a default editor launch enables — the `edqarun` alias's pair.
const EDITOR_FEATURES: [&str; 2] = ["dynamic_linking", "net_qa"];

/// What one launch runs.
///
/// A plain aggregate of four typed values (not a newtype — it has more than one field),
/// read through its accessors so the spawner never touches a bare `String` / `PathBuf`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSpec {
    /// The cargo package to build and run.
    package:     CargoPackage,
    /// The features to enable on it.
    features:    FeatureList,
    /// The directory to run the launcher in — `None` inherits the MCP host's own.
    working_dir: Option<WorkingDir>,
    /// Extra environment variables set on the child.
    env:         EnvOverrides,
    /// The two variable names this child's QA control channel is driven by.
    channel:     QaChannel,
}

impl LaunchSpec {
    /// Build a launch recipe from its five parts.
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

    /// The cargo package this launch builds and runs.
    #[must_use]
    pub const fn package(&self) -> &CargoPackage {
        &self.package
    }

    /// The features this launch enables.
    #[must_use]
    pub const fn features(&self) -> &FeatureList {
        &self.features
    }

    /// The directory the launcher runs in, or `None` to inherit the host's.
    #[must_use]
    pub const fn working_dir(&self) -> Option<&WorkingDir> {
        self.working_dir.as_ref()
    }

    /// The extra environment variables set on the child.
    #[must_use]
    pub const fn env(&self) -> &EnvOverrides {
        &self.env
    }

    /// The two variable names this child's QA control channel is driven by.
    #[must_use]
    pub const fn channel(&self) -> &QaChannel {
        &self.channel
    }

    /// The directory this launch actually builds in: the recipe's own, or the MCP host's
    /// current directory when the recipe named none.
    ///
    /// `None` only when the recipe names no directory AND the host's own cannot be read.
    /// Callers use this rather than [`working_dir`](Self::working_dir) whenever the answer
    /// is reported to an agent or compared against another recipe: "named no directory"
    /// and "named the host's own directory" are the same checkout, and a caller deciding
    /// WHICH TREE IS UNDER TEST must not have to work that out (GTW-875).
    #[must_use]
    pub fn resolved_working_dir(&self) -> Option<WorkingDir> {
        self.working_dir
            .clone()
            .or_else(|| std::env::current_dir().ok().map(WorkingDir::new))
    }

    /// Whether `other` asks for the same build as this recipe: same package, same
    /// features, same environment overrides, and the same RESOLVED directory.
    ///
    /// The manager asks this before answering an ensure-style
    /// [`AlreadyRunning`](crate::lifecycle::LaunchOutcome::AlreadyRunning): keeping a child
    /// built from a different recipe and reporting success is how QA ends up passing
    /// against code nobody reviewed (GTW-875).
    #[must_use]
    pub fn is_same_launch_as(&self, other: &Self) -> bool {
        self.package == other.package
            && self.features == other.features
            && self.env == other.env
            && self.channel == other.channel
            && self.resolved_working_dir() == other.resolved_working_dir()
    }

    /// The recipe a `launch(host="game")` call runs when it names nothing of its own: the game
    /// package with dynamic linking and the QA channel, in the host's own directory, with
    /// no extra environment. This is exactly what the launcher hardcoded before GTW-875.
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

    /// The recipe a `launch(host="editor")` call runs when it names nothing of its own: the editor
    /// BINARY package with dynamic linking and its own QA channel, in the host's own
    /// directory, with no extra environment (GTW-808). It is the `cargo edqarun` alias's
    /// command, which is what GTW-878 clause 1 shipped.
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

/// Wrap a list of feature-name literals as a [`FeatureList`].
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
