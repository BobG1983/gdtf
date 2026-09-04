//! Registries, stub links and stub lifecycles the tool tests dispatch against.

use cobalt_mcp_protocol::{
    command::{
        ArgSchemaRon, CommandAvailability, CommandCatalogue, CommandEntry, CommandName,
        CommandOutcome, CommandReplyRon, CommandSummary, CommandTiming, ReplySchemaRon,
    },
    message::{QaError, QaRequest, QaResponse, ServerNameNet},
};
use cobalt_mcp_server::{
    CargoPackage, ChildPid, EnvVarName, FeatureList, FeatureName, HostLifecycle, HostName,
    HostPair, HostRegistry, HostSet, InstanceId, LaunchOutcome, LaunchPolicy, LaunchSpec,
    LifecycleConfig, McpError, OutputTail, QaChannel, QaHostSpec, QaLink, QaPort, RecordedInstance,
    ServerIdentity, ServerName, ServerVersion, StopOutcome, TailLines, WorkingDir,
};

/// A host registered under `name`, with every field derived from that name.
pub(crate) fn registered(name: &str, port: u16, policy: LaunchPolicy) -> QaHostSpec {
    let upper = name.to_uppercase();
    QaHostSpec::new(
        HostName::new(name.to_owned()),
        CargoPackage::new(format!("{name}_package")),
        FeatureList::new(vec![FeatureName::new(format!("{name}_feature"))]),
        QaPort::new(port),
        None,
        QaChannel::new(
            EnvVarName::new(format!("{upper}_CHANNEL")),
            EnvVarName::new(format!("{upper}_CHANNEL_PORT")),
        ),
        LifecycleConfig::defaults_with_policy(policy),
    )
}

/// Two hosts under names no shipped string in the crate uses.
pub(crate) fn two_registered_hosts() -> HostRegistry {
    HostRegistry::new(vec![
        registered("thistle", 4100, LaunchPolicy::Reuse),
        registered("bramble", 4200, LaunchPolicy::AlwaysSpawn),
    ])
}

/// What the tool tests advertise as the server's identity.
pub(crate) fn test_identity() -> ServerIdentity {
    ServerIdentity::new(
        ServerName::new("sample-bridge".to_owned()),
        ServerVersion::new("9.9.9".to_owned()),
    )
}

pub(crate) const CANNED_COMMAND: &str = "app.phase";

/// Link that answers a catalogue and one command without touching a socket.
pub(crate) struct CannedLink;

impl QaLink for CannedLink {
    fn request(&mut self, request: QaRequest) -> Result<QaResponse, McpError> {
        match request {
            QaRequest::Catalogue => Ok(QaResponse::Catalogue(CommandCatalogue::new(
                ServerNameNet::new("canned-host".to_owned()),
                vec![CommandEntry::new(
                    CommandName::from_static(CANNED_COMMAND),
                    CommandSummary::from_static("Read where the app is at every level."),
                    CommandTiming::Immediate,
                    ArgSchemaRon::new("(root:Named(\"Args\"),defs:[])".to_owned()),
                    ReplySchemaRon::new("(root:Named(\"Reply\"),defs:[])".to_owned()),
                    CommandAvailability::Available,
                )],
            ))),
            QaRequest::Run(_) => Ok(QaResponse::Outcome(CommandOutcome::Ran {
                reply:       CommandReplyRon::new("(ran:true)".to_owned()),
                attachments: Vec::new(),
            })),
            QaRequest::Hello(_) => Ok(QaResponse::Error(QaError::Malformed)),
        }
    }
}

/// Lifecycle that records nothing on disk and answers every call from memory.
pub(crate) struct CannedLifecycle {
    launches: u32,
}

impl CannedLifecycle {
    pub(crate) const fn new() -> Self {
        Self { launches: 0 }
    }

    pub(crate) const fn launches(&self) -> u32 {
        self.launches
    }
}

impl HostLifecycle for CannedLifecycle {
    fn launch(&mut self, port: QaPort, _spec: &LaunchSpec) -> LaunchOutcome {
        self.launches += 1;
        LaunchOutcome::Launched {
            port,
            pid: ChildPid::new(4242),
            instance: InstanceId::new(format!("canned-{}", self.launches)),
        }
    }

    fn stop(&mut self, _port: QaPort) -> StopOutcome {
        self.stop_owned()
    }

    fn stop_instance(&mut self, _instance: &InstanceId) -> StopOutcome {
        StopOutcome::NotRunning
    }

    fn stop_owned(&mut self) -> StopOutcome {
        StopOutcome::Stopped {
            pid: ChildPid::new(4242),
        }
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
        Some(OutputTail::new("a line the child printed".to_owned()))
    }

    fn instance_output(&self, _instance: &InstanceId, _max: TailLines) -> Option<OutputTail> {
        None
    }
}

/// A host set holding one registered host's stub link and lifecycle.
pub(crate) fn one_host_set<'a>(
    name: &str,
    link: &'a mut dyn QaLink,
    lifecycle: &'a mut dyn HostLifecycle,
) -> HostSet<'a> {
    HostSet::new(
        HostRegistry::new(vec![registered(name, 4100, LaunchPolicy::Reuse)]),
        vec![(
            HostName::new(name.to_owned()),
            HostPair::new(link, lifecycle),
        )],
    )
}
