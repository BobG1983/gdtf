//! The host manager: it records the children it starts and stops them again.

use std::time::Instant;

use super::{lifecycle::HostLifecycle, port::pick_port};
use crate::{
    lifecycle::{
        child::ManagedChild,
        config::{LaunchPolicy, LifecycleConfig},
        launch::{LaunchSpec, WorkingDir},
        liveness::{ChildLiveness, SystemLiveness},
        orphan::{OrphanPid, OrphanStop, OrphanTarget, OrphanWatch, PortHold, SystemOrphanWatch},
        outcome::{LaunchFailure, LaunchOutcome, StopOutcome},
        probe::probe_ready,
        spawn::ChildSpawner,
        values::{
            ChildStatus, InstanceId, KillGrace, OutputTail, Readiness, RecordedInstance,
            SpawnError, TailLines,
        },
    },
    link::QaPort,
};

struct RunningChild {
    child:  Box<dyn ManagedChild>,
    id:     InstanceId,
    port:   QaPort,
    recipe: LaunchSpec,
}

// Instance ids for one manager, counted up from the first launch.
#[derive(Default)]
struct InstanceIds(u32);

impl InstanceIds {
    fn mint(&mut self) -> InstanceId {
        self.0 = self.0.saturating_add(1);
        InstanceId::new(format!("instance-{}", self.0))
    }
}

/// Default host lifecycle implementation.
pub struct HostManager {
    spawner:  Box<dyn ChildSpawner>,
    config:   LifecycleConfig,
    running:  Vec<RunningChild>,
    ids:      InstanceIds,
    orphans:  Box<dyn OrphanWatch>,
    liveness: Box<dyn ChildLiveness>,
}

impl HostManager {
    /// Manager with default config and system orphan watch.
    #[must_use]
    pub fn new(spawner: Box<dyn ChildSpawner>) -> Self {
        Self::with_config(spawner, LifecycleConfig::default())
    }

    /// Manager with explicit config.
    #[must_use]
    pub fn with_config(spawner: Box<dyn ChildSpawner>, config: LifecycleConfig) -> Self {
        Self::with_orphan_watch(spawner, config, Box::new(SystemOrphanWatch::new()))
    }

    /// Manager with explicit config and orphan watch.
    #[must_use]
    pub fn with_orphan_watch(
        spawner: Box<dyn ChildSpawner>,
        config: LifecycleConfig,
        orphans: Box<dyn OrphanWatch>,
    ) -> Self {
        Self::with_probes(spawner, config, orphans, Box::new(SystemLiveness::new()))
    }

    /// Manager with explicit config, orphan watch, and liveness probe.
    #[must_use]
    pub fn with_probes(
        spawner: Box<dyn ChildSpawner>,
        config: LifecycleConfig,
        orphans: Box<dyn OrphanWatch>,
        liveness: Box<dyn ChildLiveness>,
    ) -> Self {
        Self {
            spawner,
            config,
            running: Vec::new(),
            ids: InstanceIds::default(),
            orphans,
            liveness,
        }
    }

    /// Whether this manager reuses its recorded child or starts another.
    #[must_use]
    pub const fn launch_policy(&self) -> LaunchPolicy {
        self.config.launch_policy()
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

    // The launch a full record answers with, or None when this launch may spawn.
    fn reuse(&self, spec: &LaunchSpec) -> Option<LaunchOutcome> {
        if !matches!(self.config.launch_policy(), LaunchPolicy::Reuse) {
            return None;
        }
        let running = self.running.first()?;
        if !running.recipe.is_same_launch_as(spec) {
            return Some(LaunchOutcome::Failed(LaunchFailure::RecipeMismatch(
                Box::new(running.recipe.clone()),
            )));
        }
        Some(LaunchOutcome::AlreadyRunning {
            port:   running.port,
            pid:    running.child.pid(),
            recipe: Box::new(running.recipe.clone()),
        })
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
                let instance = self.ids.mint();
                self.running.push(RunningChild {
                    child,
                    id: instance.clone(),
                    port,
                    recipe: recipe.clone(),
                });
                return LaunchOutcome::Launched {
                    port,
                    pid,
                    instance,
                };
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
        if let Some(reused) = self.reuse(spec) {
            return reused;
        }
        let taken: Vec<QaPort> = self.running.iter().map(|running| running.port).collect();
        let Some(chosen) = pick_port(
            port,
            &taken,
            self.orphans.as_ref(),
            self.config.probe_timeout(),
        ) else {
            return LaunchOutcome::Failed(LaunchFailure::NoFreePort { requested: port });
        };
        if let PortHold::Orphan(pid) = self.hold_on(chosen) {
            return LaunchOutcome::Failed(LaunchFailure::PortHeldByOrphan { port: chosen, pid });
        }
        match self.spawner.spawn(chosen, spec) {
            Ok(child) => self.await_readiness(child, chosen, spec),
            Err(err) => {
                LaunchOutcome::Failed(LaunchFailure::Spawn(SpawnError::new(err.to_string())))
            }
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

    fn stop_instance(&mut self, instance: &InstanceId) -> StopOutcome {
        let Some(at) = self
            .running
            .iter()
            .position(|running| running.id == *instance)
        else {
            return StopOutcome::NotRunning;
        };
        let mut stopped = self.running.remove(at);
        let pid = stopped.child.pid();
        shutdown(stopped.child.as_mut(), self.config.kill_grace());
        StopOutcome::Stopped { pid }
    }

    fn stop_owned(&mut self) -> StopOutcome {
        let grace = self.config.kill_grace();
        let mut outcome = StopOutcome::NotRunning;
        for mut running in self.running.drain(..) {
            let pid = running.child.pid();
            shutdown(running.child.as_mut(), grace);
            outcome = StopOutcome::Stopped { pid };
        }
        outcome
    }

    fn reap_dead_child(&mut self) {
        let liveness = self.liveness.as_ref();
        self.running.retain_mut(|running| {
            if matches!(liveness.status(running.child.pid()), ChildStatus::Running) {
                return true;
            }
            running.child.reap();
            false
        });
    }

    fn instances(&self) -> Vec<RecordedInstance> {
        self.running
            .iter()
            .map(|running| {
                RecordedInstance::new(running.id.clone(), running.port, running.child.pid())
            })
            .collect()
    }

    fn child_working_dir(&self) -> Option<WorkingDir> {
        self.running
            .last()
            .and_then(|running| running.recipe.resolved_working_dir())
    }

    fn instance_working_dir(&self, instance: &InstanceId) -> Option<WorkingDir> {
        self.running
            .iter()
            .find(|running| running.id == *instance)
            .and_then(|running| running.recipe.resolved_working_dir())
    }

    fn child_output(&self, max: TailLines) -> Option<OutputTail> {
        self.running
            .last()
            .map(|running| running.child.output_tail(max))
    }

    fn instance_output(&self, instance: &InstanceId, max: TailLines) -> Option<OutputTail> {
        self.running
            .iter()
            .find(|running| running.id == *instance)
            .map(|running| running.child.output_tail(max))
    }
}

impl Drop for HostManager {
    fn drop(&mut self) {
        for mut running in self.running.drain(..) {
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
