//! The managed child process — the [`ManagedChild`] trait and its real [`ProcessChild`]
//! (GTW-745).
//!
//! [`ManagedChild`] is the small surface the launch / stop logic drives: read the pid, poll
//! whether it has exited, ask for a graceful (SIGTERM) or forceful (SIGKILL) stop, wait a
//! bounded time for it to go, reap it (no zombie), and read the tail of what it wrote to
//! stderr. [`ProcessChild`] is the one real implementation over [`std::process::Child`];
//! injecting the trait is what lets a test drive this exact logic against a stub process
//! instead of the real game binary.
//!
//! The child is spawned into its OWN process group (on Unix) so a stop signals the WHOLE
//! group — the `cargo run` launcher AND the game it spawns as a grandchild — rather than
//! leaving the game orphaned when only the launcher is killed.

use core::time::Duration;
use std::{
    collections::VecDeque,
    io::{self, BufRead, BufReader},
    process::{Child, ChildStderr, Command, Stdio},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    time::Instant,
};

use super::values::{ChildPid, ChildStatus, KillGrace, StderrTail};

/// How many trailing stderr lines a failed child's tail keeps.
const STDERR_TAIL_LINES: usize = 64;

/// The step the bounded exit-wait sleeps between polls.
const EXIT_POLL_STEP: Duration = Duration::from_millis(50);

/// A spawned child process the lifecycle manager owns and can stop cleanly.
///
/// The manager depends on this trait, not on [`ProcessChild`], so the launch / stop logic
/// can be exercised against a stub process without launching the real game.
pub trait ManagedChild {
    /// The child's operating-system process id.
    fn pid(&self) -> ChildPid;

    /// Whether the child is still running or has already exited (reaping it if it has).
    fn poll(&mut self) -> ChildStatus;

    /// Ask the child to stop gracefully — SIGTERM to its process group (on Unix; a
    /// forceful kill elsewhere, which has no gentler signal).
    fn terminate(&mut self);

    /// Force the child to stop — SIGKILL to its process group.
    fn kill(&mut self);

    /// Poll until the child exits or `within` elapses, returning what happened.
    fn wait_until_exit(&mut self, within: KillGrace) -> ChildStatus;

    /// Block until the child is reaped, so it never lingers as a zombie.
    fn reap(&mut self);

    /// The tail of what the child has written to stderr.
    fn stderr_tail(&self) -> StderrTail;
}

/// Which stop signal to deliver to the child's process group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StopSignal {
    /// SIGTERM — the graceful "please exit" request.
    Terminate,
    /// SIGKILL — the un-catchable forceful stop.
    Kill,
}

impl StopSignal {
    /// The `kill(1)` flag that names this signal.
    const fn flag(self) -> &'static str {
        match self {
            Self::Terminate => "-TERM",
            Self::Kill => "-KILL",
        }
    }
}

/// A bounded ring of the child's most recent stderr lines.
struct StderrRing {
    /// The retained trailing lines, oldest first.
    lines: VecDeque<String>,
}

impl StderrRing {
    /// An empty ring.
    const fn new() -> Self {
        Self {
            lines: VecDeque::new(),
        }
    }

    /// Append one line, dropping the oldest once the cap is reached.
    fn push_line(&mut self, line: String) {
        if self.lines.len() == STDERR_TAIL_LINES {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    /// Render the retained lines as one newline-joined string.
    fn render(&self) -> String {
        self.lines.iter().cloned().collect::<Vec<_>>().join("\n")
    }
}

/// Read the child's stderr line by line into the shared ring until the pipe closes.
fn drain_stderr(stderr: ChildStderr, sink: &Arc<Mutex<StderrRing>>) {
    let reader = BufReader::new(stderr);
    for line in reader.lines() {
        let Ok(text) = line else {
            return;
        };
        if let Ok(mut ring) = sink.lock() {
            ring.push_line(text);
        }
    }
}

/// The real [`ManagedChild`] over a [`std::process::Child`].
pub struct ProcessChild {
    /// The owned child handle.
    child:  Child,
    /// The child's process id, captured once at spawn (stable across reaping).
    pid:    ChildPid,
    /// The shared tail of the child's stderr, filled by the reader thread.
    tail:   Arc<Mutex<StderrRing>>,
    /// The stderr reader thread, joined on reap.
    reader: Option<JoinHandle<()>>,
}

impl ProcessChild {
    /// Spawn `command` as a managed child.
    ///
    /// The caller sets the program, arguments, environment, and stdout; this forces
    /// stderr to a pipe (so the tail can be captured) and, on Unix, puts the child in its
    /// own process group (so a later stop can signal the whole group).
    ///
    /// # Errors
    ///
    /// The underlying [`io::Error`] if the process cannot be spawned.
    pub fn spawn(mut command: Command) -> io::Result<Box<dyn ManagedChild>> {
        command.stderr(Stdio::piped());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let mut child = command.spawn()?;
        let pid = ChildPid::new(child.id());
        let tail = Arc::new(Mutex::new(StderrRing::new()));
        let reader = child.stderr.take().map(|stderr| {
            let sink = Arc::clone(&tail);
            thread::spawn(move || drain_stderr(stderr, &sink))
        });
        Ok(Box::new(Self {
            child,
            pid,
            tail,
            reader,
        }))
    }

    /// Deliver a stop signal to the child's process group.
    ///
    /// On Unix the child was spawned as a process-group leader (its pgid equals its pid),
    /// so `kill -SIG -pgid` reaches the launcher AND any grandchild it spawned — killing
    /// only the `cargo run` launcher would orphan the game it spawned. The workspace
    /// denies `unsafe_code`, so this shells out to `kill(1)` rather than calling `kill(2)`
    /// directly; the short-lived `kill` helper is waited on so it does not become a
    /// zombie. The signal is only ever sent while the child is un-reaped, so its pid
    /// cannot have been recycled onto an unrelated process.
    #[cfg(unix)]
    fn signal_group(&self, signal: StopSignal) {
        let group_target = format!("-{}", *self.pid);
        let mut command = Command::new("kill");
        command
            .args([signal.flag(), group_target.as_str()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Ok(mut helper) = command.spawn() {
            drop(helper.wait());
        }
    }

    /// Non-Unix fallback: there is no SIGTERM, so both requests forcefully kill the direct
    /// child (no process-group escalation).
    #[cfg(not(unix))]
    fn signal_group(&mut self, _signal: StopSignal) {
        drop(self.child.kill());
    }
}

impl ManagedChild for ProcessChild {
    fn pid(&self) -> ChildPid {
        self.pid
    }

    fn poll(&mut self) -> ChildStatus {
        match self.child.try_wait() {
            Ok(None) => ChildStatus::Running,
            Ok(Some(_)) | Err(_) => ChildStatus::Exited,
        }
    }

    fn terminate(&mut self) {
        self.signal_group(StopSignal::Terminate);
    }

    fn kill(&mut self) {
        self.signal_group(StopSignal::Kill);
    }

    fn wait_until_exit(&mut self, within: KillGrace) -> ChildStatus {
        let deadline = Instant::now() + *within;
        loop {
            if matches!(self.poll(), ChildStatus::Exited) {
                return ChildStatus::Exited;
            }
            if Instant::now() >= deadline {
                return ChildStatus::Running;
            }
            thread::sleep(EXIT_POLL_STEP);
        }
    }

    fn reap(&mut self) {
        drop(self.child.wait());
        if let Some(handle) = self.reader.take() {
            drop(handle.join());
        }
    }

    fn stderr_tail(&self) -> StderrTail {
        let text = self
            .tail
            .lock()
            .map(|ring| ring.render())
            .unwrap_or_default();
        StderrTail::new(text)
    }
}
