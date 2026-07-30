//! The host-lifecycle manager — owns the one running child and drives launch / stop
//! (GTW-745; made host-neutral in GTW-808).
//!
//! [`HostLifecycle`] is the surface the MCP tool layer calls; [`HostManager`] is the real
//! implementation. ONE manager owns ONE child, so the dual-target host holds two of them —
//! one for the game, one for the editor — and each enforces one child of its own kind.
//! [`launch`](HostLifecycle::launch) with
//! a child already running from the SAME recipe is an ensure-style
//! [`AlreadyRunning`](LaunchOutcome::AlreadyRunning), never a second spawn (mirroring the
//! game's own one-client transport rule); with a child running from a DIFFERENT recipe it
//! is a [`RecipeMismatch`](LaunchFailure::RecipeMismatch) failure naming what is actually
//! running, because answering "already running" to a request for another package,
//! features, or checkout would hand the caller a success for a build it never asked for
//! (GTW-875). A launch
//! that times out kills and reaps the orphan before returning the failure, and the
//! manager's [`Drop`] force-kills and reaps any surviving child so the MCP never outlives
//! its game silently.

use std::time::Instant;

use super::{
    child::ManagedChild,
    config::LifecycleConfig,
    launch::{LaunchSpec, WorkingDir},
    outcome::{LaunchFailure, LaunchOutcome, StopOutcome},
    probe::probe_ready,
    spawn::ChildSpawner,
    values::{ChildStatus, KillGrace, Readiness},
};
use crate::link::QaPort;

/// Launch and stop one host's child, one at a time.
///
/// The MCP tool layer depends on this trait, not on [`HostManager`], so the
/// `launch_game` / `stop_game` / `launch_editor` / `stop_editor` tool handlers can be
/// exercised against a manager driven by a stub spawner.
pub trait HostLifecycle {
    /// Ensure this host's child is running on `port`, launching one built to `spec` if
    /// none is (see [`LaunchOutcome`]).
    ///
    /// The recipe belongs to the call, not to the manager, so successive launches can ask
    /// for different builds — a `dev_tools` build, or a build in another checkout — and
    /// the recipe is checked on EVERY call, not only when a spawn happens: a child already
    /// running from the same recipe is kept
    /// ([`AlreadyRunning`](LaunchOutcome::AlreadyRunning), carrying that recipe), and a
    /// child running from a different one is reported as
    /// [`RecipeMismatch`](LaunchFailure::RecipeMismatch) instead of being passed off as the
    /// build that was asked for.
    fn launch(&mut self, port: QaPort, spec: &LaunchSpec) -> LaunchOutcome;

    /// Stop the running child, if any (see [`StopOutcome`]).
    fn stop(&mut self) -> StopOutcome;

    /// The directory the running child was launched in, or `None` when no child is running
    /// (or its directory cannot be named at all).
    ///
    /// Read by the render path so a screenshot the CHILD wrote at a relative path is opened
    /// relative to the child's directory instead of the MCP host's own. Before GTW-923
    /// nothing asked this question, so a capture taken by a child launched with a
    /// `working_dir` of its own — a git worktree, the whole point of GTW-875 — came back as
    /// "could not be read" while the PNG sat on disk.
    fn child_working_dir(&self) -> Option<WorkingDir>;
}

/// A running child together with the port it was launched on and the recipe that built it.
struct RunningChild {
    /// The owned, managed child process.
    child:  Box<dyn ManagedChild>,
    /// The loopback port it listens on.
    port:   QaPort,
    /// The recipe it was launched from — which package, features, and checkout.
    recipe: LaunchSpec,
}

/// The real [`HostLifecycle`] — owns the optional running child and the spawner.
pub struct HostManager {
    /// How the game child is launched (real cargo run, or a test stub).
    spawner: Box<dyn ChildSpawner>,
    /// The timing knobs the launch / stop loops wait on.
    config:  LifecycleConfig,
    /// The one running child, or `None` when no game is up.
    running: Option<RunningChild>,
}

impl HostManager {
    /// Build a manager with the production timing config.
    #[must_use]
    pub fn new(spawner: Box<dyn ChildSpawner>) -> Self {
        Self::with_config(spawner, LifecycleConfig::default())
    }

    /// Build a manager with an explicit timing config (the tests' short-fused knobs).
    #[must_use]
    pub fn with_config(spawner: Box<dyn ChildSpawner>, config: LifecycleConfig) -> Self {
        Self {
            spawner,
            config,
            running: None,
        }
    }

    /// Wait for the freshly spawned `child` to answer readiness, returning the launch
    /// outcome. On success the child is retained as the running game, together with the
    /// `recipe` it was built from; on timeout or an early exit the child is reaped before
    /// this returns.
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
                // Reap FIRST, then read the tail: reaping joins the stderr reader thread,
                // which finishes draining the closed pipe into the ring. Reading first can
                // catch the reader mid-drain and miss the child's final lines (GTW-756).
                child.reap();
                let tail = child.stderr_tail();
                return LaunchOutcome::Failed(LaunchFailure::ExitedEarly(tail));
            }
            if Instant::now() >= deadline {
                // Shut down FIRST, then read the tail: `shutdown` kills and reaps the
                // child, and reaping joins the stderr reader thread, so the pipe is fully
                // drained before it is read. Reading first can catch the reader mid-drain
                // and truncate the diagnosis this failure exists to carry (GTW-756).
                shutdown(child.as_mut(), self.config.kill_grace());
                let tail = child.stderr_tail();
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
        match self.spawner.spawn(port, spec) {
            Ok(child) => self.await_readiness(child, port, spec),
            Err(err) => LaunchOutcome::Failed(LaunchFailure::Spawn(
                super::values::SpawnError::new(err.to_string()),
            )),
        }
    }

    fn stop(&mut self) -> StopOutcome {
        let Some(mut running) = self.running.take() else {
            return StopOutcome::NotRunning;
        };
        let pid = running.child.pid();
        shutdown(running.child.as_mut(), self.config.kill_grace());
        StopOutcome::Stopped { pid }
    }

    fn child_working_dir(&self) -> Option<WorkingDir> {
        // The RESOLVED directory, not the recipe's own: a launch that named no directory
        // still ran somewhere — the host's own current directory — and a caller resolving a
        // child-written path needs that answer, not `None` (GTW-875 drew the same
        // distinction for reporting which tree is under test).
        self.running
            .as_ref()
            .and_then(|running| running.recipe.resolved_working_dir())
    }
}

impl Drop for HostManager {
    fn drop(&mut self) {
        // Last-resort orphan guard on ANY exit path (including an unwind): force-kill and
        // reap a still-owned child so it never outlives the MCP. The graceful SIGTERM path
        // is the explicit `stop` at normal shutdown; by then `running` is already `None`.
        if let Some(mut running) = self.running.take() {
            running.child.kill();
            running.child.reap();
        }
    }
}

/// Stop a child cleanly: SIGTERM, wait the grace period, escalate to SIGKILL if it is
/// still up, then reap it so it never lingers as a zombie.
fn shutdown(child: &mut dyn ManagedChild, grace: KillGrace) {
    child.terminate();
    if matches!(child.wait_until_exit(grace), ChildStatus::Running) {
        child.kill();
    }
    child.reap();
}
