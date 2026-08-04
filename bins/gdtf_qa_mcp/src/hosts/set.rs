//! Borrowed link + lifecycle pair for each host.

use super::host::QaHost;
use crate::{lifecycle::HostLifecycle, link::QaLink};

/// Link and lifecycle for one host.
pub struct HostPair<'a> {
    link:      &'a mut dyn QaLink,
    lifecycle: &'a mut dyn HostLifecycle,
}

impl<'a> HostPair<'a> {
    /// Pair a link with its lifecycle manager.
    pub fn new(link: &'a mut dyn QaLink, lifecycle: &'a mut dyn HostLifecycle) -> Self {
        Self { link, lifecycle }
    }

    /// Mutable link.
    pub fn link(&mut self) -> &mut dyn QaLink {
        self.link
    }

    /// Mutable lifecycle manager.
    pub fn lifecycle(&mut self) -> &mut dyn HostLifecycle {
        self.lifecycle
    }

    /// Both handles at once.
    pub fn parts(&mut self) -> (&mut dyn QaLink, &mut dyn HostLifecycle) {
        (self.link, self.lifecycle)
    }
}

/// Game and editor pairs together.
pub struct HostSet<'a> {
    game:   HostPair<'a>,
    editor: HostPair<'a>,
}

impl<'a> HostSet<'a> {
    /// Build from both pairs.
    #[must_use]
    pub const fn new(game: HostPair<'a>, editor: HostPair<'a>) -> Self {
        Self { game, editor }
    }

    /// Select the pair for a host.
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

    const STUB_PORT: QaPort = QaPort::new(7616);

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

    struct NamedLink(&'static str);

    impl QaLink for NamedLink {
        fn request(&mut self, _request: QaRequest) -> Result<QaResponse, McpError> {
            Ok(QaResponse::HelloOk(HelloFacts::new(
                STUB_PROTOCOL,
                ServerNameNet::new(self.0.to_owned()),
            )))
        }
    }

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

    fn answered_by(pair: &mut HostPair<'_>) -> ServerNameNet {
        let Ok(QaResponse::HelloOk(facts)) = pair.link().request(QaRequest::Hello(STUB_PROTOCOL))
        else {
            unreachable!("the named link always answers HelloOk");
        };
        facts.server
    }

    fn stopped_pid(pair: &mut HostPair<'_>) -> u32 {
        let StopOutcome::Stopped { pid } = pair.lifecycle().stop(STUB_PORT) else {
            unreachable!("the named lifecycle always reports a stopped pid");
        };
        *pid
    }

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
