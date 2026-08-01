//! The canned links and lifecycles the fixtures dispatch against, and the two dispatch
//! helpers every test calls.

use gdtf_qa_mcp::{
    ChildPid, HostLifecycle, HostPair, HostSet, LaunchOutcome, LaunchSpec, McpError, OutputTail,
    QaLink, QaPort, StopOutcome, TailLines, WorkingDir, dispatch,
};
use gdtf_qa_protocol::{
    command::{
        ArgSchemaJson, ArgumentFault, CommandAvailability, CommandCatalogue, CommandEntry,
        CommandName, CommandOutcome, CommandReplyJson, CommandSummary, CommandTiming, RefusalNote,
        ReplySchemaJson, RunOptions, UnavailableCode,
    },
    message::{QaError, QaRequest, QaResponse, RunCommand, ServerNameNet},
};
use serde_json::Value;

/// The name the canned EDITOR host answers its (empty) catalogue under.
pub(crate) const EDITOR_HOST_NAME: &str = "gdtf-editor-net-qa";

/// The one command the canned GAME host offers — the same name the real game's
/// `GAME_COMMANDS` publishes.
pub(crate) const CANNED_COMMAND: &str = "app.phase";
/// The canned host's argument schema for [`CANNED_COMMAND`], carrying the
/// `"additionalProperties": false` a `deny_unknown_fields` argument type derives.
///
/// A LITERAL, not a derivation: this crate carries schema documents as opaque TEXT and links
/// no `schemars`. That the REAL game derives this shape is proved game-side, in
/// `crates/gdtf_app/tests/net_qa/commands.rs`; what these fixtures prove is what the courier
/// does with whatever document arrives.
pub(crate) const CANNED_ARG_SCHEMA: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","title":"AppPhaseArgs","type":"object","properties":{},"additionalProperties":false}"#;
/// The canned host's reply schema for [`CANNED_COMMAND`] — a nested record, as the real one is.
pub(crate) const CANNED_REPLY_SCHEMA: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","title":"AppPhaseReply","type":"object","properties":{"phase":{"type":"object"}},"required":["phase"]}"#;
/// The reply body the canned host answers a successful [`CANNED_COMMAND`] call with — the
/// five-level tuple, with the four nested levels absent because the canned host is at its menu.
pub(crate) const CANNED_REPLY: &str = r#"{"phase":{"app":"Running","running":"Menu","game":null,"battlescape":null,"aftermath":null}}"#;

/// The canned GAME host's one-command catalogue.
fn canned_catalogue() -> CommandCatalogue {
    CommandCatalogue::new(
        ServerNameNet::new("gdtf-net-qa".to_owned()),
        vec![CommandEntry::new(
            CommandName::from_static(CANNED_COMMAND),
            CommandSummary::from_static("Read where the app is at every level."),
            CommandTiming::Immediate,
            ArgSchemaJson::new(CANNED_ARG_SCHEMA.to_owned()),
            ReplySchemaJson::new(CANNED_REPLY_SCHEMA.to_owned()),
            CommandAvailability::Available,
        )],
    )
}

/// Spell out the riders that reached the host, and what each one carried.
///
/// The real host's `rider_refusal` names WHICH rider it has not built; this goes one step
/// further and reports each rider's VALUE, because that is the only way a fixture can tell
/// "the courier sent the rider" from "the courier sent the rider with the wrong payload" —
/// a `capture` whose file stem was dropped would otherwise read exactly like one that kept it.
fn riders_that_arrived(options: &RunOptions) -> String {
    let mut named = Vec::new();
    if let Some(budget) = options.await_ready.as_ref() {
        named.push(format!("await_ready={}", **budget));
    }
    if let Some(capture) = options.capture.as_ref() {
        named.push(match capture.name.as_ref() {
            Some(stem) => format!("capture={}", stem.as_str()),
            None => "capture=<host-chosen>".to_owned(),
        });
    }
    format!(
        "this build has no rider, and these arrived: {}",
        named.join(", ")
    )
}

/// The canned GAME host's answer to one `Run`, mirroring the REAL host's admission order.
///
/// It decides from the frame the COURIER built, which is what makes these fixtures able to
/// see a courier that dropped an argument or a rider: an unknown NAME is reported first, a
/// rider this build has not implemented next, and only then are the arguments decoded.
fn canned_run(run: &RunCommand) -> CommandOutcome {
    if run.command.as_str() != CANNED_COMMAND {
        return CommandOutcome::Unknown {
            known: vec![CommandName::from_static(CANNED_COMMAND)],
        };
    }
    if !run.options.is_plain() {
        return CommandOutcome::Unavailable {
            code: UnavailableCode::NotBuilt,
            note: RefusalNote::from_owned(riders_that_arrived(&run.options)),
        };
    }
    if run.arguments.as_str() != "{}" {
        return CommandOutcome::BadArguments {
            detail: ArgumentFault::new(format!(
                "unknown field, expected no fields: {}",
                run.arguments.as_str()
            )),
            schema: ArgSchemaJson::new(CANNED_ARG_SCHEMA.to_owned()),
        };
    }
    CommandOutcome::Ran {
        reply:       CommandReplyJson::new(CANNED_REPLY.to_owned()),
        attachments: Vec::new(),
    }
}

/// The GAME's canned link: it publishes the one-command catalogue and answers a `Run`
/// exactly as the real host's admission order does, so `tools/call` can be exercised with no
/// socket.
struct CannedGame;

impl QaLink for CannedGame {
    fn request(&mut self, request: QaRequest) -> Result<QaResponse, McpError> {
        match request {
            QaRequest::Catalogue => Ok(QaResponse::Catalogue(canned_catalogue())),
            QaRequest::Run(run) => Ok(QaResponse::Outcome(canned_run(&run))),
            QaRequest::Hello(_) => Ok(QaResponse::Error(QaError::Malformed)),
        }
    }
}

/// The EDITOR's canned link: it publishes an EMPTY catalogue under its own host name and
/// answers every `Run` `Unknown` — the mirror image of [`CannedGame`], so a call that
/// reached the wrong link is visible in the reply rather than silent.
struct CannedEditor;

impl QaLink for CannedEditor {
    fn request(&mut self, request: QaRequest) -> Result<QaResponse, McpError> {
        match request {
            QaRequest::Catalogue => Ok(QaResponse::Catalogue(CommandCatalogue::new(
                ServerNameNet::new(EDITOR_HOST_NAME.to_owned()),
                Vec::new(),
            ))),
            QaRequest::Run(_) => Ok(QaResponse::Outcome(CommandOutcome::Unknown {
                known: Vec::new(),
            })),
            QaRequest::Hello(_) => Ok(QaResponse::Error(QaError::Malformed)),
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

    fn stop(&mut self, _port: QaPort) -> StopOutcome {
        StopOutcome::NotRunning
    }

    fn stop_owned(&mut self) -> StopOutcome {
        StopOutcome::NotRunning
    }

    fn child_working_dir(&self) -> Option<WorkingDir> {
        // No child, so no directory — a reported path renders against the dispatching
        // process's own directory, which is what the pre-GTW-923 host always did.
        None
    }

    fn child_output(&self, _max: TailLines) -> Option<OutputTail> {
        None
    }
}

/// The two lines the GAME's canned lifecycle reports its child printed.
pub(crate) const GAME_LOG: &str = "first line from the game\nsecond line from the game";
/// The two lines the EDITOR's canned lifecycle reports its child printed.
///
/// Different text from [`GAME_LOG`] for the same reason the ports and pids differ: a `logs`
/// call that reached the wrong lifecycle has to be visible in the reply, not silent.
pub(crate) const EDITOR_LOG: &str = "first line from the editor\nsecond line from the editor";

/// The port + pid the GAME's canned lifecycle reports.
pub(crate) const GAME_PORT: u16 = 7616;
/// The pid the GAME's canned lifecycle reports.
pub(crate) const GAME_PID: u32 = 4242;
/// The port the EDITOR's canned lifecycle reports.
pub(crate) const EDITOR_PORT: u16 = 7617;
/// The pid the EDITOR's canned lifecycle reports.
pub(crate) const EDITOR_PID: u32 = 5150;

/// A lifecycle with canned launch / stop outcomes, so the three host-local tools can be
/// exercised through the real dispatch without spawning a process.
///
/// Each host gets one carrying its OWN port, pid and log text, so a launch, stop or logs
/// reply names which lifecycle actually ran — without that a swapped host lookup is
/// invisible.
struct CannedLifecycle {
    /// The port this lifecycle's launch reports.
    port: u16,
    /// The pid this lifecycle's launch and stop report.
    pid:  u32,
    /// What this lifecycle's child "printed", read by `logs`.
    log:  &'static str,
}

impl HostLifecycle for CannedLifecycle {
    fn launch(&mut self, _port: QaPort, _spec: &LaunchSpec) -> LaunchOutcome {
        LaunchOutcome::Launched {
            port: QaPort::new(self.port),
            pid:  ChildPid::new(self.pid),
        }
    }

    fn stop(&mut self, _port: QaPort) -> StopOutcome {
        self.stop_owned()
    }

    fn stop_owned(&mut self) -> StopOutcome {
        StopOutcome::Stopped {
            pid: ChildPid::new(self.pid),
        }
    }

    fn child_working_dir(&self) -> Option<WorkingDir> {
        None
    }

    fn child_output(&self, _max: TailLines) -> Option<OutputTail> {
        Some(OutputTail::new(self.log.to_owned()))
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
            log:  GAME_LOG,
        };
        let mut editor_life = CannedLifecycle {
            port: EDITOR_PORT,
            pid:  EDITOR_PID,
            log:  EDITOR_LOG,
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
