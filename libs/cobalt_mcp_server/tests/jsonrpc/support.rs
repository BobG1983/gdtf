use cobalt_mcp_protocol::{
    command::{
        ArgSchemaRon, ArgumentFault, CommandAvailability, CommandCatalogue, CommandEntry,
        CommandName, CommandOutcome, CommandReplyRon, CommandSummary, CommandTiming, RefusalNote,
        ReplySchemaRon, RunOptions, UnavailableCode,
    },
    message::{McpRequest, McpResponse, McpSessionError, RunCommand, ServerNameNet},
};
use cobalt_mcp_server::{
    ChildPid, HostLifecycle, InstanceId, LaunchOutcome, LaunchSpec, McpError, McpLink, McpPort,
    OutputTail, RecordedInstance, StopOutcome, TailLines, WorkingDir, dispatch,
};
use serde_json::Value;

use crate::hosts::{test_identity, two_host_set};

pub(crate) const BRAMBLE_HOST_NAME: &str = "sample-bramble-mcp";

pub(crate) const CANNED_COMMAND: &str = "app.phase";
pub(crate) const CANNED_ARG_SCHEMA: &str =
    r#"(root:Named("AppPhaseArgs"),defs:[("AppPhaseArgs",Record([]))])"#;
pub(crate) const CANNED_REPLY_SCHEMA: &str = r#"(root:Named("AppPhaseReply"),defs:[("AppPhaseReply",Record([("phase",Named("AppPhaseNet"))]))])"#;
pub(crate) const CANNED_REPLY: &str =
    "(phase:(app:Running,running:Some(Menu),game:None,battlescape:None,aftermath:None))";

fn canned_catalogue() -> CommandCatalogue {
    CommandCatalogue::new(
        ServerNameNet::new("sample-mcp".to_owned()),
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

pub(crate) struct CannedThistle;

impl McpLink for CannedThistle {
    fn request(&mut self, request: McpRequest) -> Result<McpResponse, McpError> {
        match request {
            McpRequest::Catalogue => Ok(McpResponse::Catalogue(canned_catalogue())),
            McpRequest::Run(run) => Ok(McpResponse::Outcome(canned_run(&run))),
            McpRequest::Hello(_) => Ok(McpResponse::Error(McpSessionError::Malformed)),
        }
    }
}

struct CannedBramble;

impl McpLink for CannedBramble {
    fn request(&mut self, request: McpRequest) -> Result<McpResponse, McpError> {
        match request {
            McpRequest::Catalogue => Ok(McpResponse::Catalogue(CommandCatalogue::new(
                ServerNameNet::new(BRAMBLE_HOST_NAME.to_owned()),
                Vec::new(),
            ))),
            McpRequest::Run(_) => Ok(McpResponse::Outcome(CommandOutcome::Unknown {
                known: Vec::new(),
            })),
            McpRequest::Hello(_) => Ok(McpResponse::Error(McpSessionError::Malformed)),
        }
    }
}

struct NoLifecycle;

impl HostLifecycle for NoLifecycle {
    fn launch(&mut self, _port: McpPort, _spec: &LaunchSpec) -> LaunchOutcome {
        unreachable!("the forwarding-tool fixtures never launch");
    }

    fn stop(&mut self, _port: McpPort) -> StopOutcome {
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

    fn instance_working_dir(&self, _instance: &InstanceId) -> Option<WorkingDir> {
        None
    }

    fn child_output(&self, _max: TailLines) -> Option<OutputTail> {
        None
    }

    fn instance_output(&self, _instance: &InstanceId, _max: TailLines) -> Option<OutputTail> {
        None
    }
}

pub(crate) const THISTLE_LOG: &str = "first line from the thistle\nsecond line from the thistle";
pub(crate) const BRAMBLE_LOG: &str = "first line from the bramble\nsecond line from the bramble";

pub(crate) const THISTLE_PORT: u16 = 4100;
pub(crate) const THISTLE_PID: u32 = 4242;
pub(crate) const BRAMBLE_PORT: u16 = 4200;
pub(crate) const BRAMBLE_PID: u32 = 5150;

pub(crate) const FIRST_BRAMBLE_LOG: &str =
    "first line from bramble one\nsecond line from bramble one";
pub(crate) const SECOND_BRAMBLE_LOG: &str =
    "first line from bramble two\nsecond line from bramble two";

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

    pub(crate) const fn port(&self) -> McpPort {
        McpPort::new(self.port)
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
            McpPort::new(self.port),
            ChildPid::new(self.pid),
        )
    }
}

pub(crate) const FIRST_BRAMBLE_INSTANCE: SeededInstance =
    SeededInstance::new("bramble-one", BRAMBLE_PORT, 5201, FIRST_BRAMBLE_LOG);

pub(crate) const SECOND_BRAMBLE_INSTANCE: SeededInstance =
    SeededInstance::new("bramble-two", 4300, 5202, SECOND_BRAMBLE_LOG);

pub(crate) const THISTLE_INSTANCE: SeededInstance =
    SeededInstance::new("thistle-one", THISTLE_PORT, 4301, THISTLE_LOG);

pub(crate) struct CannedLifecycle {
    port:   u16,
    pid:    u32,
    log:    &'static str,
    seeded: Vec<SeededInstance>,
    minted: u32,
}

impl CannedLifecycle {
    pub(crate) fn new(port: u16, pid: u32, log: &'static str, seeded: &[SeededInstance]) -> Self {
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
    fn launch(&mut self, _port: McpPort, _spec: &LaunchSpec) -> LaunchOutcome {
        self.minted += 1;
        LaunchOutcome::Launched {
            port:     McpPort::new(self.port),
            pid:      ChildPid::new(self.pid),
            instance: InstanceId::new(format!("canned-{}", self.minted)),
        }
    }

    fn stop(&mut self, _port: McpPort) -> StopOutcome {
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

    fn instance_working_dir(&self, _instance: &InstanceId) -> Option<WorkingDir> {
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
    let (mut thistle_link, mut bramble_link) = (CannedThistle, CannedBramble);
    if lifecycles {
        let mut thistle_life = CannedLifecycle::new(THISTLE_PORT, THISTLE_PID, THISTLE_LOG, &[]);
        let mut bramble_life = CannedLifecycle::new(BRAMBLE_PORT, BRAMBLE_PID, BRAMBLE_LOG, &[]);
        let mut hosts = two_host_set(
            &mut thistle_link,
            &mut thistle_life,
            &mut bramble_link,
            &mut bramble_life,
        );
        return dispatch(line, &test_identity(), &mut hosts);
    }
    let (mut thistle_life, mut bramble_life) = (NoLifecycle, NoLifecycle);
    let mut hosts = two_host_set(
        &mut thistle_link,
        &mut thistle_life,
        &mut bramble_link,
        &mut bramble_life,
    );
    dispatch(line, &test_identity(), &mut hosts)
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
    bramble_instances: &[SeededInstance],
    thistle_instances: &[SeededInstance],
) -> Vec<Value> {
    let (mut thistle_link, mut bramble_link) = (CannedThistle, CannedBramble);
    let mut thistle_life =
        CannedLifecycle::new(THISTLE_PORT, THISTLE_PID, THISTLE_LOG, thistle_instances);
    let mut bramble_life =
        CannedLifecycle::new(BRAMBLE_PORT, BRAMBLE_PID, BRAMBLE_LOG, bramble_instances);
    let mut hosts = two_host_set(
        &mut thistle_link,
        &mut thistle_life,
        &mut bramble_link,
        &mut bramble_life,
    );
    let identity = test_identity();
    lines
        .iter()
        .map(|line| {
            let Some(response) = dispatch(line, &identity, &mut hosts) else {
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
