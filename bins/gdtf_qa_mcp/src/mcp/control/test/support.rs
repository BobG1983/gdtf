//! The fixtures the control-tool tests drive: a stub lifecycle that records the recipe it
//! was handed, a link that records its re-pointing, and a directory that is not the test
//! process's own.

use core::cell::Cell;

use gdtf_qa_protocol::message::{QaRequest, QaResponse};

use crate::{
    error::McpError,
    lifecycle::{
        HostLifecycle, LaunchFailure, LaunchOutcome, LaunchSpec, OutputTail, SpawnError,
        StopOutcome, TailLines, WorkingDir,
    },
    link::{QaLink, QaPort},
};

/// A lifecycle whose launch / stop return canned outcomes — no real processes. It keeps
/// the recipe it was handed, so a test can check what the tool asked for.
pub(super) struct StubLifecycle {
    /// The outcome `launch` returns.
    pub(super) launch:       LaunchOutcome,
    /// The outcome `stop` returns.
    pub(super) stop:         StopOutcome,
    /// The recipe of the last `launch` call, if any.
    pub(super) last_spec:    Option<LaunchSpec>,
    /// The port of the last `stop` call, if any — the fact a stop needs to establish who
    /// holds the port at all (GTW-926).
    pub(super) stopped_port: Option<QaPort>,
    /// What the child has printed, or `None` for a stub owning no child (GTW-943).
    pub(super) output:       Option<OutputTail>,
    /// The line cap the last `child_output` call was handed.
    ///
    /// Recorded rather than applied: the cap is the RING's to apply, so what a fixture can
    /// prove is that the parsed argument reaches the lifecycle at all. Without this, a
    /// handler that dropped the caller's cap and asked for the default would still render
    /// the cap it echoes, and nothing would fail (GTW-943).
    pub(super) asked_for:    Cell<Option<TailLines>>,
}

impl StubLifecycle {
    /// A stub that reports the given launch outcome and nothing to stop.
    pub(super) const fn launching(launch: LaunchOutcome) -> Self {
        Self {
            launch,
            stop: StopOutcome::NotRunning,
            last_spec: None,
            stopped_port: None,
            output: None,
            asked_for: Cell::new(None),
        }
    }

    /// A stub whose child has printed `output` — or, for `None`, one owning no child.
    ///
    /// Its launch outcome is never read: a `logs` call touches only `child_output`.
    pub(super) const fn printing(output: Option<OutputTail>) -> Self {
        Self {
            launch: LaunchOutcome::Failed(LaunchFailure::Spawn(SpawnError::new(String::new()))),
            stop: StopOutcome::NotRunning,
            last_spec: None,
            stopped_port: None,
            output,
            asked_for: Cell::new(None),
        }
    }
}

impl HostLifecycle for StubLifecycle {
    fn launch(&mut self, _port: QaPort, spec: &LaunchSpec) -> LaunchOutcome {
        self.last_spec = Some(spec.clone());
        self.launch.clone()
    }

    fn stop(&mut self, port: QaPort) -> StopOutcome {
        self.stopped_port = Some(port);
        self.stop.clone()
    }

    fn stop_owned(&mut self) -> StopOutcome {
        self.stop.clone()
    }

    fn child_working_dir(&self) -> Option<WorkingDir> {
        // The control tools neither read nor render a capture, so these fixtures have no
        // child directory to report. The attachment path's use of it is covered where it
        // matters — `tests/jsonrpc/courier_attach.rs` and `tests/jsonrpc/screenshot_cwd.rs`.
        None
    }

    fn child_output(&self, max: TailLines) -> Option<OutputTail> {
        // The line CAP is the ring's to apply (`ProcessChild::output_tail`), so this fixture
        // RECORDS what it was handed and returns what it was built with. Recording is the
        // half that matters here: `tests/lifecycle/child_output.rs` proves the real manager
        // applies the cap, and this proves the handler passes the caller's value down rather
        // than its own default.
        self.asked_for.set(Some(max));
        self.output.clone()
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
