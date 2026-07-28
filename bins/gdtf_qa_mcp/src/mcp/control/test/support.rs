//! The fixtures the control-tool tests drive: a stub lifecycle that records the recipe it
//! was handed, a link that records its re-pointing, and a directory that is not the test
//! process's own.

use gdtf_qa_protocol::envelope::{QaRequest, QaResponse};

use crate::{
    error::McpError,
    lifecycle::{HostLifecycle, LaunchOutcome, LaunchSpec, StopOutcome},
    link::{QaLink, QaPort},
};

/// A lifecycle whose launch / stop return canned outcomes — no real processes. It keeps
/// the recipe it was handed, so a test can check what the tool asked for.
pub(super) struct StubLifecycle {
    /// The outcome `launch` returns.
    pub(super) launch:    LaunchOutcome,
    /// The outcome `stop` returns.
    pub(super) stop:      StopOutcome,
    /// The recipe of the last `launch` call, if any.
    pub(super) last_spec: Option<LaunchSpec>,
}

impl StubLifecycle {
    /// A stub that reports the given launch outcome and nothing to stop.
    pub(super) const fn launching(launch: LaunchOutcome) -> Self {
        Self {
            launch,
            stop: StopOutcome::NotRunning,
            last_spec: None,
        }
    }
}

impl HostLifecycle for StubLifecycle {
    fn launch(&mut self, _port: QaPort, spec: &LaunchSpec) -> LaunchOutcome {
        self.last_spec = Some(spec.clone());
        self.launch.clone()
    }

    fn stop(&mut self) -> StopOutcome {
        self.stop.clone()
    }
}

/// A game link that records the last port it was re-pointed at.
pub(super) struct RecordingLink {
    /// The port the last `retarget` set, if any.
    pub(super) retargeted: Option<QaPort>,
}

impl QaLink for RecordingLink {
    fn request(&mut self, _request: QaRequest) -> Result<QaResponse, McpError> {
        Err(McpError::Disconnected)
    }

    fn retarget(&mut self, port: QaPort) {
        self.retargeted = Some(port);
    }
}

/// A real directory that is NOT the test process's own.
///
/// The recipe's directory falls back to the host's current directory when the recipe names
/// none, so a fixture equal to that current directory would let a launch that dropped the
/// recipe's directory entirely still render the expected value. This one cannot.
pub(super) fn a_directory_that_is_not_the_hosts() -> std::path::PathBuf {
    let dir = std::env::temp_dir();
    let Ok(here) = std::env::current_dir() else {
        unreachable!("the test process has a current directory");
    };
    assert!(dir.is_dir(), "the system temp directory exists: {dir:?}");
    assert_ne!(dir, here, "the fixture directory differs from the host's");
    dir
}
