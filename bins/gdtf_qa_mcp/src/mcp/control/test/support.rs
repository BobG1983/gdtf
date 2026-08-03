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

pub(super) struct StubLifecycle {
        pub(super) launch:       LaunchOutcome,
        pub(super) stop:         StopOutcome,
        pub(super) last_spec:    Option<LaunchSpec>,
            pub(super) stopped_port: Option<QaPort>,
        pub(super) output:       Option<OutputTail>,
                            pub(super) asked_for:    Cell<Option<TailLines>>,
}

impl StubLifecycle {
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
        None
    }

    fn child_output(&self, max: TailLines) -> Option<OutputTail> {
        self.asked_for.set(Some(max));
        self.output.clone()
    }
}

pub(super) struct RecordingLink {
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

pub(super) fn a_directory_that_is_not_the_hosts() -> std::path::PathBuf {
    let dir = std::env::temp_dir();
    let Ok(here) = std::env::current_dir() else {
        unreachable!("the test process has a current directory");
    };
    assert!(dir.is_dir(), "the system temp directory exists: {dir:?}");
    assert_ne!(dir, here, "the fixture directory differs from the host's");
    dir
}
