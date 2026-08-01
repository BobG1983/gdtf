//! The managed child process — the [`ManagedChild`] trait and its real [`ProcessChild`]
//! (GTW-745).
//!
//! [`ManagedChild`] is the small surface the launch / stop logic drives: read the pid, poll
//! whether it has exited, ask for a graceful (SIGTERM) or forceful (SIGKILL) stop, wait a
//! bounded time for it to go, reap it (no zombie), and read the tail of what it wrote.
//! [`ProcessChild`] is the one real implementation over [`std::process::Child`];
//! injecting the trait is what lets a test drive this exact logic against a stub process
//! instead of the real game binary.
//!
//! BOTH of the child's output streams are piped and drained into one ring (GTW-943). Before
//! that only stderr was, and only a FAILED launch ever read it — stdout went to
//! `Stdio::null()`, so a child that came up fine and then logged something interesting had
//! nowhere to say it. The `logs` tool reads [`ManagedChild::output_tail`], which is that ring
//! and both streams, interleaved in the order the reader threads observed them.
//!
//! The child is spawned into its OWN process group (on Unix) so a stop signals the WHOLE
//! group — the `cargo run` launcher AND the game it spawns as a grandchild — rather than
//! leaving the game orphaned when only the launcher is killed.

use core::time::Duration;
use std::{
    collections::VecDeque,
    io::{self, BufRead, BufReader},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    time::Instant,
};

use super::values::{ChildPid, ChildStatus, FailureTail, KillGrace, OutputTail, TailLines};

/// How many trailing output lines a child's ring keeps.
///
/// Deep enough that a `logs` call after a few minutes of play still holds the interesting
/// part, and bounded so a chatty child cannot grow the host's memory without limit. Public
/// so the eviction test can drive the ring PAST it without pinning the number — the depth is
/// a tuning knob, the bound is the behaviour.
pub const OUTPUT_TAIL_LINES: TailLines = TailLines::new(512);

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

    /// The whole retained tail of what the child has written — a failed launch's diagnosis.
    ///
    /// Both pipes feed one ring (see [`output_tail`](Self::output_tail)), so this interleaves
    /// stdout with stderr. That is deliberate: a boot failure's cause is often the last normal
    /// line before the error, and splitting the two loses the ordering between them.
    fn failure_tail(&self) -> FailureTail;

    /// The last `max` lines the child wrote to stdout or stderr, newest last.
    ///
    /// One ring over BOTH streams: a caller reading a child's output wants what the process
    /// said, in order, not two lists it has to merge itself.
    fn output_tail(&self, max: TailLines) -> OutputTail;
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

/// A bounded ring of the child's most recent output lines, from both streams.
struct OutputRing {
    /// The retained trailing lines, oldest first.
    lines: VecDeque<String>,
}

impl OutputRing {
    /// An empty ring.
    const fn new() -> Self {
        Self {
            lines: VecDeque::new(),
        }
    }

    /// Append one line, dropping the oldest once the cap is reached.
    fn push_line(&mut self, line: String) {
        if self.lines.len() >= *OUTPUT_TAIL_LINES {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    /// Render the retained lines as one newline-joined string.
    fn render(&self) -> String {
        self.lines.iter().cloned().collect::<Vec<_>>().join("\n")
    }

    /// Render the LAST `max` retained lines as one newline-joined string.
    fn render_tail(&self, max: TailLines) -> String {
        let keep = self.lines.len().saturating_sub(*max);
        self.lines
            .iter()
            .skip(keep)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Read one of the child's pipes line by line into the shared ring until it closes.
fn drain_pipe<R: std::io::Read>(pipe: R, sink: &Arc<Mutex<OutputRing>>) {
    let reader = BufReader::new(pipe);
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
    child:   Child,
    /// The child's process id, captured once at spawn (stable across reaping).
    pid:     ChildPid,
    /// The shared tail of the child's output, filled by the reader threads.
    tail:    Arc<Mutex<OutputRing>>,
    /// The two output reader threads, joined on reap.
    readers: Vec<JoinHandle<()>>,
}

impl ProcessChild {
    /// Spawn `command` as a managed child.
    ///
    /// The caller sets the program, arguments and environment; this forces BOTH output
    /// streams to pipes (so the tail can be captured — the host's own stdout is the JSON-RPC
    /// channel and must never carry the child's) and, on Unix, puts the child in its own
    /// process group (so a later stop can signal the whole group).
    ///
    /// # Errors
    ///
    /// The underlying [`io::Error`] if the process cannot be spawned.
    pub fn spawn(mut command: Command) -> io::Result<Box<dyn ManagedChild>> {
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let mut child = command.spawn()?;
        let pid = ChildPid::new(child.id());
        let tail = Arc::new(Mutex::new(OutputRing::new()));
        let mut readers = Vec::with_capacity(2);
        if let Some(stdout) = child.stdout.take() {
            let sink = Arc::clone(&tail);
            readers.push(thread::spawn(move || drain_pipe(stdout, &sink)));
        }
        if let Some(stderr) = child.stderr.take() {
            let sink = Arc::clone(&tail);
            readers.push(thread::spawn(move || drain_pipe(stderr, &sink)));
        }
        Ok(Box::new(Self {
            child,
            pid,
            tail,
            readers,
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
        for handle in self.readers.drain(..) {
            drop(handle.join());
        }
    }

    fn failure_tail(&self) -> FailureTail {
        let text = self
            .tail
            .lock()
            .map(|ring| ring.render())
            .unwrap_or_default();
        FailureTail::new(text)
    }

    fn output_tail(&self, max: TailLines) -> OutputTail {
        let text = self
            .tail
            .lock()
            .map(|ring| ring.render_tail(max))
            .unwrap_or_default();
        OutputTail::new(text)
    }
}
