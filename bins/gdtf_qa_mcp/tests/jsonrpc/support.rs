//! The canned links and lifecycles the fixtures dispatch against, and the two dispatch
//! helpers every test calls.

use gdtf_qa_mcp::{
    ChildPid, HostLifecycle, HostPair, HostSet, LaunchOutcome, LaunchSpec, McpError, QaLink,
    QaPort, StopOutcome, dispatch,
};
use gdtf_qa_protocol::{
    envelope::{InjectReceipt, QaError, QaRequest, QaResponse},
    view::{
        AppFlowView, AppStateNet, BattleActiveNet, CaughtUpNet, EditorQueryKind,
        EditorQueryOptionsView, EditorQueryReply, EditorQueryTopicView, EditorQueryView,
        EditorReadinessNet,
    },
};
use serde_json::Value;

/// The GAME's canned link: answers `app_flow` with a Running snapshot and `send_input`
/// with a queued receipt, so `tools/call` can be exercised with no socket.
///
/// It rejects the editor's two requests the way a real game does — the game binary has no
/// editor to query — which is what makes the fixtures able to tell the two hosts apart: an
/// editor tool that reached this link renders an error instead of a topic list.
struct CannedGame;

impl QaLink for CannedGame {
    fn request(&mut self, request: QaRequest) -> Result<QaResponse, McpError> {
        match request {
            QaRequest::GetAppFlow => Ok(QaResponse::AppFlow(AppFlowView::new(
                AppStateNet::Running,
                BattleActiveNet::new(true),
                Vec::new(),
                CaughtUpNet::new(true),
                None,
                None,
            ))),
            QaRequest::Inject(_) => Ok(QaResponse::Injected(InjectReceipt::Queued)),
            _ => Ok(QaResponse::Error(QaError::BadRequest)),
        }
    }
}

/// The EDITOR's canned link: answers the options query with a Load-phase topic list and a
/// `Readiness` query with `Editing`, and rejects the game's requests — the mirror image of
/// [`CannedGame`], so a game tool that reached the editor link renders an error.
struct CannedEditor;

impl QaLink for CannedEditor {
    fn request(&mut self, request: QaRequest) -> Result<QaResponse, McpError> {
        match request {
            QaRequest::GetEditorQueryOptions => {
                Ok(QaResponse::EditorQueryOptions(EditorQueryOptionsView::new(
                    EditorReadinessNet::Load,
                    vec![EditorQueryTopicView::offered(EditorQueryKind::Validation)],
                )))
            }
            QaRequest::QueryEditor(EditorQueryKind::Readiness) => {
                Ok(QaResponse::EditorQuery(EditorQueryReply::new(
                    EditorReadinessNet::Editing,
                    EditorQueryView::Readiness(EditorReadinessNet::Editing),
                )))
            }
            _ => Ok(QaResponse::Error(QaError::BadRequest)),
        }
    }
}

/// A lifecycle the forwarding-tool fixtures never invoke — only present so `dispatch` has
/// its argument.
struct NoLifecycle;

impl HostLifecycle for NoLifecycle {
    fn launch(&mut self, _port: QaPort, _spec: &LaunchSpec) -> LaunchOutcome {
        unreachable!("the forwarding-tool fixtures never launch");
    }

    fn stop(&mut self) -> StopOutcome {
        StopOutcome::NotRunning
    }
}

/// The port + pid the GAME's canned lifecycle reports.
pub(crate) const GAME_PORT: u16 = 7616;
/// The pid the GAME's canned lifecycle reports.
pub(crate) const GAME_PID: u32 = 4242;
/// The port the EDITOR's canned lifecycle reports.
pub(crate) const EDITOR_PORT: u16 = 7617;
/// The pid the EDITOR's canned lifecycle reports.
pub(crate) const EDITOR_PID: u32 = 5150;

/// A lifecycle with canned launch / stop outcomes, so the two host-local tools can be
/// exercised through the real dispatch without spawning a process.
///
/// Each host gets one carrying its OWN port and pid, so a launch or stop reply names which
/// lifecycle actually ran — without that a swapped host lookup is invisible.
struct CannedLifecycle {
    /// The port this lifecycle's launch reports.
    port: u16,
    /// The pid this lifecycle's launch and stop report.
    pid:  u32,
}

impl HostLifecycle for CannedLifecycle {
    fn launch(&mut self, _port: QaPort, _spec: &LaunchSpec) -> LaunchOutcome {
        LaunchOutcome::Launched {
            port: QaPort::new(self.port),
            pid:  ChildPid::new(self.pid),
        }
    }

    fn stop(&mut self) -> StopOutcome {
        StopOutcome::Stopped {
            pid: ChildPid::new(self.pid),
        }
    }
}

/// Dispatch a literal line through the real dispatch with canned hosts, returning the
/// raw response line (or `None` for a notification).
pub(crate) fn dispatch_line(line: &str, lifecycles: bool) -> Option<String> {
    let (mut game_link, mut editor_link) = (CannedGame, CannedEditor);
    if lifecycles {
        let mut game_life = CannedLifecycle {
            port: GAME_PORT,
            pid:  GAME_PID,
        };
        let mut editor_life = CannedLifecycle {
            port: EDITOR_PORT,
            pid:  EDITOR_PID,
        };
        let mut hosts = HostSet::new(
            HostPair::new(&mut game_link, &mut game_life),
            HostPair::new(&mut editor_link, &mut editor_life),
        );
        return dispatch(line, &mut hosts);
    }
    let (mut game_life, mut editor_life) = (NoLifecycle, NoLifecycle);
    let mut hosts = HostSet::new(
        HostPair::new(&mut game_link, &mut game_life),
        HostPair::new(&mut editor_link, &mut editor_life),
    );
    dispatch(line, &mut hosts)
}

/// Dispatch a literal line and parse the response line to a JSON value.
pub(crate) fn dispatch_json(line: &str) -> Value {
    let Some(response) = dispatch_line(line, false) else {
        unreachable!("a request with an id yields a response line");
    };
    serde_json::from_str(&response).unwrap_or(Value::Null)
}

/// Dispatch a literal line against the canned lifecycle and parse the response.
pub(crate) fn dispatch_lifecycle_json(line: &str) -> Value {
    let Some(response) = dispatch_line(line, true) else {
        unreachable!("a request with an id yields a response line");
    };
    serde_json::from_str(&response).unwrap_or(Value::Null)
}
