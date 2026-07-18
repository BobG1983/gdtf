//! The game-lifecycle manager — owns the one running child and drives launch / stop
//! (GTW-745).
//!
//! [`GameLifecycle`] is the surface the MCP tool layer calls; [`GameManager`] is the real
//! implementation. It enforces ONE game at a time: [`launch`](GameLifecycle::launch) with
//! a child already running is an ensure-style [`AlreadyRunning`](LaunchOutcome::AlreadyRunning),
//! never a second spawn (mirroring the game's own one-client transport rule). A launch
//! that times out kills and reaps the orphan before returning the failure, and the
//! manager's [`Drop`] force-kills and reaps any surviving child so the MCP never outlives
//! its game silently.

use std::time::Instant;

use super::{
    child::GameChild,
    config::LifecycleConfig,
    outcome::{LaunchFailure, LaunchOutcome, StopOutcome},
    probe::probe_ready,
    spawn::GameSpawner,
    values::{ChildStatus, KillGrace, Readiness},
};
use crate::game::GamePort;

/// Launch and stop the game child, one at a time.
///
/// The MCP tool layer depends on this trait, not on [`GameManager`], so the `launch_game`
/// / `stop_game` tool handlers can be exercised against a manager driven by a stub
/// spawner.
pub trait GameLifecycle {
    /// Ensure a game is running on `port`, launching one if none is (see
    /// [`LaunchOutcome`]).
    fn launch(&mut self, port: GamePort) -> LaunchOutcome;

    /// Stop the running game child, if any (see [`StopOutcome`]).
    fn stop(&mut self) -> StopOutcome;
}

/// A running child together with the port it was launched on.
struct RunningChild {
    /// The owned, managed child process.
    child: Box<dyn GameChild>,
    /// The loopback port it listens on.
    port:  GamePort,
}

/// The real [`GameLifecycle`] — owns the optional running child and the spawner.
pub struct GameManager {
    /// How the game child is launched (real cargo run, or a test stub).
    spawner: Box<dyn GameSpawner>,
    /// The timing knobs the launch / stop loops wait on.
    config:  LifecycleConfig,
    /// The one running child, or `None` when no game is up.
    running: Option<RunningChild>,
}

impl GameManager {
    /// Build a manager with the production timing config.
    #[must_use]
    pub fn new(spawner: Box<dyn GameSpawner>) -> Self {
        Self::with_config(spawner, LifecycleConfig::default())
    }

    /// Build a manager with an explicit timing config (the tests' short-fused knobs).
    #[must_use]
    pub fn with_config(spawner: Box<dyn GameSpawner>, config: LifecycleConfig) -> Self {
        Self {
            spawner,
            config,
            running: None,
        }
    }

    /// Wait for the freshly spawned `child` to answer readiness, returning the launch
    /// outcome. On success the child is retained as the running game; on timeout or an
    /// early exit the child is reaped before this returns.
    fn await_readiness(&mut self, mut child: Box<dyn GameChild>, port: GamePort) -> LaunchOutcome {
        let deadline = Instant::now() + *self.config.boot_timeout();
        loop {
            if matches!(
                probe_ready(port, self.config.probe_timeout()),
                Readiness::Ready
            ) {
                let pid = child.pid();
                self.running = Some(RunningChild { child, port });
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
                return LaunchOutcome::Failed(LaunchFailure::Timeout(tail));
            }
            std::thread::sleep(*self.config.poll_interval());
        }
    }
}

impl GameLifecycle for GameManager {
    fn launch(&mut self, port: GamePort) -> LaunchOutcome {
        if let Some(running) = &self.running {
            return LaunchOutcome::AlreadyRunning {
                port: running.port,
                pid:  running.child.pid(),
            };
        }
        match self.spawner.spawn(port) {
            Ok(child) => self.await_readiness(child, port),
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
}

impl Drop for GameManager {
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
fn shutdown(child: &mut dyn GameChild, grace: KillGrace) {
    child.terminate();
    if matches!(child.wait_until_exit(grace), ChildStatus::Running) {
        child.kill();
    }
    child.reap();
}
