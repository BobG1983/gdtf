use std::time::Instant;

use super::{
    child::ManagedChild,
    config::LifecycleConfig,
    launch::{LaunchSpec, WorkingDir},
    orphan::{OrphanPid, OrphanStop, OrphanTarget, OrphanWatch, PortHold, SystemOrphanWatch},
    outcome::{LaunchFailure, LaunchOutcome, StopOutcome},
    probe::probe_ready,
    spawn::ChildSpawner,
    values::{ChildStatus, KillGrace, OutputTail, Readiness, TailLines},
};
use crate::link::QaPort;

pub trait HostLifecycle {
                                                fn launch(&mut self, port: QaPort, spec: &LaunchSpec) -> LaunchOutcome;

                                    fn stop(&mut self, port: QaPort) -> StopOutcome;

                            fn stop_owned(&mut self) -> StopOutcome;

                                    fn child_working_dir(&self) -> Option<WorkingDir>;

                                fn child_output(&self, max: TailLines) -> Option<OutputTail>;
}

struct RunningChild {
        child:  Box<dyn ManagedChild>,
        port:   QaPort,
        recipe: LaunchSpec,
}

pub struct HostManager {
        spawner: Box<dyn ChildSpawner>,
        config:  LifecycleConfig,
        running: Option<RunningChild>,
        orphans: Box<dyn OrphanWatch>,
}

impl HostManager {
        #[must_use]
    pub fn new(spawner: Box<dyn ChildSpawner>) -> Self {
        Self::with_config(spawner, LifecycleConfig::default())
    }

        #[must_use]
    pub fn with_config(spawner: Box<dyn ChildSpawner>, config: LifecycleConfig) -> Self {
        Self::with_orphan_watch(spawner, config, Box::new(SystemOrphanWatch::new()))
    }

                            #[must_use]
    pub fn with_orphan_watch(
        spawner: Box<dyn ChildSpawner>,
        config: LifecycleConfig,
        orphans: Box<dyn OrphanWatch>,
    ) -> Self {
        Self {
            spawner,
            config,
            running: None,
            orphans,
        }
    }

        fn hold_on(&self, port: QaPort) -> PortHold {
        self.orphans.inspect(port, self.config.probe_timeout())
    }

                    fn stop_orphan(&self, port: QaPort, pid: OrphanPid) -> StopOutcome {
        let OrphanPid::Known(known) = pid else {
            return StopOutcome::OrphanHeld { port, pid };
        };
        let target = OrphanTarget::from_config(port, known, self.config);
        match self.orphans.stop(target) {
            OrphanStop::Stopped => StopOutcome::OrphanStopped { port, pid },
            OrphanStop::Survived => StopOutcome::OrphanHeld { port, pid },
        }
    }

                    fn await_readiness(
        &mut self,
        mut child: Box<dyn ManagedChild>,
        port: QaPort,
        recipe: &LaunchSpec,
    ) -> LaunchOutcome {
        let deadline = Instant::now() + *self.config.boot_timeout();
        loop {
            if matches!(
                probe_ready(port, self.config.probe_timeout()),
                Readiness::Ready
            ) {
                let pid = child.pid();
                self.running = Some(RunningChild {
                    child,
                    port,
                    recipe: recipe.clone(),
                });
                return LaunchOutcome::Launched { port, pid };
            }
            if matches!(child.poll(), ChildStatus::Exited) {
                child.reap();
                let tail = child.failure_tail();
                return LaunchOutcome::Failed(LaunchFailure::ExitedEarly(tail));
            }
            if Instant::now() >= deadline {
                shutdown(child.as_mut(), self.config.kill_grace());
                let tail = child.failure_tail();
                return LaunchOutcome::Failed(LaunchFailure::Timeout {
                    tail,
                    waited: self.config.boot_timeout(),
                });
            }
            std::thread::sleep(*self.config.poll_interval());
        }
    }
}

impl HostLifecycle for HostManager {
    fn launch(&mut self, port: QaPort, spec: &LaunchSpec) -> LaunchOutcome {
        if let Some(running) = &self.running {
            if !running.recipe.is_same_launch_as(spec) {
                return LaunchOutcome::Failed(LaunchFailure::RecipeMismatch(Box::new(
                    running.recipe.clone(),
                )));
            }
            return LaunchOutcome::AlreadyRunning {
                port:   running.port,
                pid:    running.child.pid(),
                recipe: Box::new(running.recipe.clone()),
            };
        }
        if let PortHold::Orphan(pid) = self.hold_on(port) {
            return LaunchOutcome::Failed(LaunchFailure::PortHeldByOrphan { port, pid });
        }
        match self.spawner.spawn(port, spec) {
            Ok(child) => self.await_readiness(child, port, spec),
            Err(err) => LaunchOutcome::Failed(LaunchFailure::Spawn(
                super::values::SpawnError::new(err.to_string()),
            )),
        }
    }

    fn stop(&mut self, port: QaPort) -> StopOutcome {
        let owned = self.stop_owned();
        if !matches!(owned, StopOutcome::NotRunning) {
            return owned;
        }
        match self.hold_on(port) {
            PortHold::Free => StopOutcome::NotRunning,
            PortHold::Orphan(pid) => self.stop_orphan(port, pid),
        }
    }

    fn stop_owned(&mut self) -> StopOutcome {
        let Some(mut running) = self.running.take() else {
            return StopOutcome::NotRunning;
        };
        let pid = running.child.pid();
        shutdown(running.child.as_mut(), self.config.kill_grace());
        StopOutcome::Stopped { pid }
    }

    fn child_working_dir(&self) -> Option<WorkingDir> {
        self.running
            .as_ref()
            .and_then(|running| running.recipe.resolved_working_dir())
    }

    fn child_output(&self, max: TailLines) -> Option<OutputTail> {
        self.running
            .as_ref()
            .map(|running| running.child.output_tail(max))
    }
}

impl Drop for HostManager {
    fn drop(&mut self) {
        if let Some(mut running) = self.running.take() {
            running.child.kill();
            running.child.reap();
        }
    }
}

fn shutdown(child: &mut dyn ManagedChild, grace: KillGrace) {
    child.terminate();
    if matches!(child.wait_until_exit(grace), ChildStatus::Running) {
        child.kill();
    }
    child.reap();
}
