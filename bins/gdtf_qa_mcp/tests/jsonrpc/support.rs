use gdtf_qa_mcp::{
    ChildPid, HostLifecycle, HostPair, HostSet, InstanceId, LaunchOutcome, LaunchSpec, McpError,
    OutputTail, QaLink, QaPort, RecordedInstance, StopOutcome, TailLines, WorkingDir, dispatch,
};
use gdtf_qa_protocol::{
    command::{
        ArgSchemaRon, ArgumentFault, CommandAvailability, CommandCatalogue, CommandEntry,
        CommandName, CommandOutcome, CommandReplyRon, CommandSummary, CommandTiming, RefusalNote,
        ReplySchemaRon, RunOptions, UnavailableCode,
    },
    message::{QaError, QaRequest, QaResponse, RunCommand, ServerNameNet},
};
use serde_json::Value;

pub(crate) const EDITOR_HOST_NAME: &str = "gdtf-editor-net-qa";

pub(crate) const CANNED_COMMAND: &str = "app.phase";
pub(crate) const CANNED_ARG_SCHEMA: &str =
    r#"(root:Named("AppPhaseArgs"),defs:[("AppPhaseArgs",Record([]))])"#;
pub(crate) const CANNED_REPLY_SCHEMA: &str = r#"(root:Named("AppPhaseReply"),defs:[("AppPhaseReply",Record([("phase",Named("AppPhaseNet"))]))])"#;
pub(crate) const CANNED_REPLY: &str =
    "(phase:(app:Running,running:Some(Menu),game:None,battlescape:None,aftermath:None))";

fn canned_catalogue() -> CommandCatalogue {
    CommandCatalogue::new(
        ServerNameNet::new("gdtf-net-qa".to_owned()),
        vec![CommandEntry::new(
            CommandName::from_static(CANNED_COMMAND),
            CommandSummary::from_static("Read where the app is at every level."),
            CommandTiming::Immediate,
            ArgSchemaRon::new(CANNED_ARG_SCHEMA.to_owned()),
            ReplySchemaRon::new(CANNED_REPLY_SCHEMA.to_owned()),
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
    if run.arguments.as_str() != "()" {
        return CommandOutcome::BadArguments {
            detail: ArgumentFault::new(format!(
                "unknown field, expected no fields: {}",
                run.arguments.as_str()
            )),
            schema: ArgSchemaRon::new(CANNED_ARG_SCHEMA.to_owned()),
        };
    }
    CommandOutcome::Ran {
        reply:       CommandReplyRon::new(CANNED_REPLY.to_owned()),
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

    fn stop_instance(&mut self, _instance: &InstanceId) -> StopOutcome {
        StopOutcome::NotRunning
    }

    fn stop_owned(&mut self) -> StopOutcome {
        StopOutcome::NotRunning
    }

    fn reap_dead_child(&mut self) {}

    fn instances(&self) -> Vec<RecordedInstance> {
        Vec::new()
    }

    fn child_working_dir(&self) -> Option<WorkingDir> {
        None
    }

    fn child_output(&self, _max: TailLines) -> Option<OutputTail> {
        None
    }

    fn instance_output(&self, _instance: &InstanceId, _max: TailLines) -> Option<OutputTail> {
        None
    }
}

pub(crate) const GAME_LOG: &str = "first line from the game\nsecond line from the game";
pub(crate) const EDITOR_LOG: &str = "first line from the editor\nsecond line from the editor";

pub(crate) const GAME_PORT: u16 = 7616;
pub(crate) const GAME_PID: u32 = 4242;
pub(crate) const EDITOR_PORT: u16 = 7617;
pub(crate) const EDITOR_PID: u32 = 5150;

pub(crate) const FIRST_EDITOR_LOG: &str = "first line from editor one\nsecond line from editor one";
pub(crate) const SECOND_EDITOR_LOG: &str =
    "first line from editor two\nsecond line from editor two";

/// One instance a canned host records, with the log that instance printed.
#[derive(Clone)]
pub(crate) struct SeededInstance {
    id:   &'static str,
    port: u16,
    pid:  u32,
    log:  &'static str,
}

impl SeededInstance {
    pub(crate) const fn new(id: &'static str, port: u16, pid: u32, log: &'static str) -> Self {
        Self { id, port, pid, log }
    }

    pub(crate) const fn id(&self) -> &'static str {
        self.id
    }

    pub(crate) const fn pid(&self) -> u32 {
        self.pid
    }

    pub(crate) const fn log(&self) -> &'static str {
        self.log
    }

    fn recorded(&self) -> RecordedInstance {
        RecordedInstance::new(
            InstanceId::new(self.id.to_owned()),
            QaPort::new(self.port),
            ChildPid::new(self.pid),
        )
    }
}

pub(crate) const FIRST_EDITOR_INSTANCE: SeededInstance =
    SeededInstance::new("editor-one", EDITOR_PORT, 5201, FIRST_EDITOR_LOG);

pub(crate) const SECOND_EDITOR_INSTANCE: SeededInstance =
    SeededInstance::new("editor-two", 7618, 5202, SECOND_EDITOR_LOG);

pub(crate) const GAME_INSTANCE: SeededInstance =
    SeededInstance::new("game-one", GAME_PORT, 4301, GAME_LOG);

struct CannedLifecycle {
    port:   u16,
    pid:    u32,
    log:    &'static str,
    seeded: Vec<SeededInstance>,
    minted: u32,
}

impl CannedLifecycle {
    fn new(port: u16, pid: u32, log: &'static str, seeded: &[SeededInstance]) -> Self {
        Self {
            port,
            pid,
            log,
            seeded: seeded.to_vec(),
            minted: 0,
        }
    }

    fn at(&self, instance: &InstanceId) -> Option<&SeededInstance> {
        self.seeded
            .iter()
            .find(|seeded| seeded.id == instance.as_str())
    }
}

impl HostLifecycle for CannedLifecycle {
    fn launch(&mut self, _port: QaPort, _spec: &LaunchSpec) -> LaunchOutcome {
        self.minted += 1;
        LaunchOutcome::Launched {
            port:     QaPort::new(self.port),
            pid:      ChildPid::new(self.pid),
            instance: InstanceId::new(format!("canned-{}", self.minted)),
        }
    }

    fn stop(&mut self, _port: QaPort) -> StopOutcome {
        self.stop_owned()
    }

    fn stop_instance(&mut self, instance: &InstanceId) -> StopOutcome {
        let Some(at) = self
            .seeded
            .iter()
            .position(|seeded| seeded.id == instance.as_str())
        else {
            return StopOutcome::NotRunning;
        };
        StopOutcome::Stopped {
            pid: ChildPid::new(self.seeded.remove(at).pid),
        }
    }

    fn stop_owned(&mut self) -> StopOutcome {
        StopOutcome::Stopped {
            pid: ChildPid::new(self.pid),
        }
    }

    fn reap_dead_child(&mut self) {}

    fn instances(&self) -> Vec<RecordedInstance> {
        self.seeded.iter().map(SeededInstance::recorded).collect()
    }

    fn child_working_dir(&self) -> Option<WorkingDir> {
        None
    }

    fn child_output(&self, _max: TailLines) -> Option<OutputTail> {
        Some(OutputTail::new(self.log.to_owned()))
    }

    fn instance_output(&self, instance: &InstanceId, _max: TailLines) -> Option<OutputTail> {
        self.at(instance)
            .map(|seeded| OutputTail::new(seeded.log.to_owned()))
    }
}

pub(crate) fn dispatch_line(line: &str, lifecycles: bool) -> Option<String> {
    let (mut game_link, mut editor_link) = (CannedGame, CannedEditor);
    if lifecycles {
        let mut game_life = CannedLifecycle::new(GAME_PORT, GAME_PID, GAME_LOG, &[]);
        let mut editor_life = CannedLifecycle::new(EDITOR_PORT, EDITOR_PID, EDITOR_LOG, &[]);
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

/// Dispatch several lines against one host set seeded with these instances.
pub(crate) fn dispatch_seeded(
    lines: &[&str],
    editor_instances: &[SeededInstance],
    game_instances: &[SeededInstance],
) -> Vec<Value> {
    let (mut game_link, mut editor_link) = (CannedGame, CannedEditor);
    let mut game_life = CannedLifecycle::new(GAME_PORT, GAME_PID, GAME_LOG, game_instances);
    let mut editor_life =
        CannedLifecycle::new(EDITOR_PORT, EDITOR_PID, EDITOR_LOG, editor_instances);
    let mut hosts = HostSet::new(
        HostPair::new(&mut game_link, &mut game_life),
        HostPair::new(&mut editor_link, &mut editor_life),
    );
    lines
        .iter()
        .map(|line| {
            let Some(response) = dispatch(line, &mut hosts) else {
                unreachable!("a request with an id yields a response line");
            };
            serde_json::from_str(&response).unwrap_or(Value::Null)
        })
        .collect()
}

/// The tool reply's own JSON payload, parsed out of its text content block.
pub(crate) fn payload(response: &Value) -> Value {
    let Some(text) = response["result"]["content"][0]["text"].as_str() else {
        unreachable!("a tool reply carries a text content block: {response}");
    };
    serde_json::from_str(text).unwrap_or(Value::Null)
}
