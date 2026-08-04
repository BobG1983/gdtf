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

pub(crate) const EDITOR_HOST_NAME: &str = "gdtf-editor-net-qa";

pub(crate) const CANNED_COMMAND: &str = "app.phase";
pub(crate) const CANNED_ARG_SCHEMA: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","title":"AppPhaseArgs","type":"object","properties":{},"additionalProperties":false}"#;
pub(crate) const CANNED_REPLY_SCHEMA: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","title":"AppPhaseReply","type":"object","properties":{"phase":{"type":"object"}},"required":["phase"]}"#;
pub(crate) const CANNED_REPLY: &str = r#"{"phase":{"app":"Running","running":"Menu","game":null,"battlescape":null,"aftermath":null}}"#;

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
        None
    }

    fn child_output(&self, _max: TailLines) -> Option<OutputTail> {
        None
    }
}

pub(crate) const GAME_LOG: &str = "first line from the game\nsecond line from the game";
pub(crate) const EDITOR_LOG: &str = "first line from the editor\nsecond line from the editor";

pub(crate) const GAME_PORT: u16 = 7616;
pub(crate) const GAME_PID: u32 = 4242;
pub(crate) const EDITOR_PORT: u16 = 7617;
pub(crate) const EDITOR_PID: u32 = 5150;

struct CannedLifecycle {
    port: u16,
    pid:  u32,
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

pub(crate) fn dispatch_json(line: &str) -> Value {
    let Some(response) = dispatch_line(line, false) else {
        unreachable!("a request with an id yields a response line");
    };
    serde_json::from_str(&response).unwrap_or(Value::Null)
}

pub(crate) fn dispatch_lifecycle_json(line: &str) -> Value {
    let Some(response) = dispatch_line(line, true) else {
        unreachable!("a request with an id yields a response line");
    };
    serde_json::from_str(&response).unwrap_or(Value::Null)
}
