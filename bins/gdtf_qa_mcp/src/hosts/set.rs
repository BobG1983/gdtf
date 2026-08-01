//! [`HostSet`] and [`HostPair`] — the two hosts' links and lifecycles, keyed by
//! [`QaHost`] (GTW-808).
//!
//! Before this the MCP host held exactly one link and one manager, so `serve.rs`
//! constructed one of each and every handler took them as two arguments. The dual-target
//! host holds one of each PER HOST and hands the tool layer whichever pair the resolved
//! tool names, which is what lets a game on `7616` and an editor on `7617` be driven from
//! one process at the same time.

use super::host::QaHost;
use crate::{lifecycle::HostLifecycle, link::QaLink};

/// One host's two moving parts: the link its forwarding tools speak over, and the
/// lifecycle its launch / stop tools drive.
///
/// Borrowed rather than owned, so the owner (`serve.rs`, or a test) keeps the concrete
/// [`QaClient`](crate::link::QaClient) / [`HostManager`](crate::lifecycle::HostManager)
/// and can stop the child after the transport loop returns.
pub struct HostPair<'a> {
    /// The link a forwarding tool call for this host travels over.
    link:      &'a mut dyn QaLink,
    /// The lifecycle this host's launch / stop tools drive.
    lifecycle: &'a mut dyn HostLifecycle,
}

impl<'a> HostPair<'a> {
    /// Build a pair from one host's link and lifecycle.
    pub fn new(link: &'a mut dyn QaLink, lifecycle: &'a mut dyn HostLifecycle) -> Self {
        Self { link, lifecycle }
    }

    /// The link this host's forwarding tools speak over.
    pub fn link(&mut self) -> &mut dyn QaLink {
        self.link
    }

    /// The lifecycle this host's launch / stop tools drive.
    pub fn lifecycle(&mut self) -> &mut dyn HostLifecycle {
        self.lifecycle
    }

    /// Both halves at once, for a caller that needs to hand them to one function —
    /// a launch reads the link (to probe readiness) and drives the lifecycle, and
    /// two separate accessor calls would borrow `self` mutably twice.
    pub fn parts(&mut self) -> (&mut dyn QaLink, &mut dyn HostLifecycle) {
        (self.link, self.lifecycle)
    }
}

/// Both hosts' pairs, resolved by [`QaHost`].
///
/// The one place the "which host" question is answered for the whole dispatch path: a call
/// names its host with the `host` argument every tool takes, and this hands back that host's
/// pair, so no handler below it branches on game-versus-editor.
pub struct HostSet<'a> {
    /// The game's link + lifecycle.
    game:   HostPair<'a>,
    /// The editor's link + lifecycle.
    editor: HostPair<'a>,
}

impl<'a> HostSet<'a> {
    /// Build the set from the two hosts' pairs.
    #[must_use]
    pub const fn new(game: HostPair<'a>, editor: HostPair<'a>) -> Self {
        Self { game, editor }
    }

    /// The pair belonging to `host`.
    pub const fn pair(&mut self, host: QaHost) -> &mut HostPair<'a> {
        match host {
            QaHost::Game => &mut self.game,
            QaHost::Editor => &mut self.editor,
        }
    }
}

#[cfg(test)]
mod test {
    use gdtf_qa_protocol::message::{
        HelloFacts, ProtocolVersion, QaRequest, QaResponse, ServerNameNet,
    };

    /// The port the stub lifecycles are asked about — irrelevant to what is asserted, since
    /// each answers from its own name rather than from any real process.
    const STUB_PORT: QaPort = QaPort::new(7616);

    /// The protocol version the stub links answer with — irrelevant to what is asserted.
    const STUB_PROTOCOL: ProtocolVersion = ProtocolVersion::new(1);

    use super::{HostPair, HostSet, QaHost};
    use crate::{
        error::McpError,
        lifecycle::{
            ChildPid, HostLifecycle, LaunchOutcome, LaunchSpec, OutputTail, StopOutcome, TailLines,
            WorkingDir,
        },
        link::{QaLink, QaPort},
    };

    /// A link that names itself in every reply, so a reply proves WHICH link answered.
    struct NamedLink(&'static str);

    impl QaLink for NamedLink {
        fn request(&mut self, _request: QaRequest) -> Result<QaResponse, McpError> {
            Ok(QaResponse::HelloOk(HelloFacts::new(
                STUB_PROTOCOL,
                ServerNameNet::new(self.0.to_owned()),
            )))
        }
    }

    /// A lifecycle that reports a distinct pid, so an outcome proves WHICH lifecycle ran.
    struct NamedLifecycle(u32);

    impl HostLifecycle for NamedLifecycle {
        fn launch(&mut self, port: QaPort, _spec: &LaunchSpec) -> LaunchOutcome {
            LaunchOutcome::Launched {
                port,
                pid: ChildPid::new(self.0),
            }
        }

        fn stop(&mut self, _port: QaPort) -> StopOutcome {
            self.stop_owned()
        }

        fn stop_owned(&mut self) -> StopOutcome {
            StopOutcome::Stopped {
                pid: ChildPid::new(self.0),
            }
        }

        fn child_working_dir(&self) -> Option<WorkingDir> {
            None
        }

        fn child_output(&self, _max: TailLines) -> Option<OutputTail> {
            None
        }
    }

    /// The name a link answered with, for the assertions below.
    fn answered_by(pair: &mut HostPair<'_>) -> ServerNameNet {
        let Ok(QaResponse::HelloOk(facts)) = pair.link().request(QaRequest::Hello(STUB_PROTOCOL))
        else {
            unreachable!("the named link always answers HelloOk");
        };
        facts.server
    }

    /// The pid a lifecycle's stop reported, for the assertions below.
    fn stopped_pid(pair: &mut HostPair<'_>) -> u32 {
        let StopOutcome::Stopped { pid } = pair.lifecycle().stop(STUB_PORT) else {
            unreachable!("the named lifecycle always reports a stopped pid");
        };
        *pid
    }

    /// [`HostSet::pair`] hands back the pair that BELONGS to the named host — both its link
    /// and its lifecycle.
    ///
    /// The two hosts are given links and lifecycles that identify themselves, so swapping
    /// either match arm fails here. Without that the whole dual-host routing (`ToolName::host`
    /// → this lookup → a child process) is answerable only by fixtures that cannot tell the
    /// two apart, and an editor tool reaching the GAME's child would pass unnoticed
    /// (GTW-808 clause 5).
    #[test]
    fn each_host_resolves_to_its_own_link_and_lifecycle() {
        let (mut game_link, mut editor_link) = (NamedLink("game"), NamedLink("editor"));
        let (mut game_life, mut editor_life) = (NamedLifecycle(7616), NamedLifecycle(7617));
        let mut hosts = HostSet::new(
            HostPair::new(&mut game_link, &mut game_life),
            HostPair::new(&mut editor_link, &mut editor_life),
        );

        assert_eq!(
            answered_by(hosts.pair(QaHost::Game)),
            ServerNameNet::new("game".to_owned())
        );
        assert_eq!(
            answered_by(hosts.pair(QaHost::Editor)),
            ServerNameNet::new("editor".to_owned())
        );
        assert_eq!(stopped_pid(hosts.pair(QaHost::Game)), 7616);
        assert_eq!(stopped_pid(hosts.pair(QaHost::Editor)), 7617);
    }
}
