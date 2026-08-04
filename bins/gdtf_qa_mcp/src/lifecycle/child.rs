//! Managed OS child process with output capture.

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

/// Max lines kept in the output ring for failure tails.
pub const OUTPUT_TAIL_LINES: TailLines = TailLines::new(512);

const EXIT_POLL_STEP: Duration = Duration::from_millis(50);

/// Process we launched and can signal, wait on, and read tails from.
pub trait ManagedChild {
    /// Process id.
    fn pid(&self) -> ChildPid;

    /// Non-blocking status check.
    fn poll(&mut self) -> ChildStatus;

    /// Ask the process (group) to exit gracefully.
    fn terminate(&mut self);

    /// Force-kill the process (group).
    fn kill(&mut self);

    /// Wait up to `within` for exit; return current status.
    fn wait_until_exit(&mut self, within: KillGrace) -> ChildStatus;

    /// Block until the process has been reaped and readers joined.
    fn reap(&mut self);

    /// Full captured output for failure reporting.
    fn failure_tail(&self) -> FailureTail;

    /// Trailing `max` lines of output.
    fn output_tail(&self, max: TailLines) -> OutputTail;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StopSignal {
    Terminate,
    Kill,
}

impl StopSignal {
    const fn flag(self) -> &'static str {
        match self {
            Self::Terminate => "-TERM",
            Self::Kill => "-KILL",
        }
    }
}

struct OutputRing {
    lines: VecDeque<String>,
}

impl OutputRing {
    const fn new() -> Self {
        Self {
            lines: VecDeque::new(),
        }
    }

    fn push_line(&mut self, line: String) {
        if self.lines.len() >= *OUTPUT_TAIL_LINES {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    fn render(&self) -> String {
        self.lines.iter().cloned().collect::<Vec<_>>().join("\n")
    }

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

/// Real OS child with threaded stdout/stderr capture.
pub struct ProcessChild {
    child:   Child,
    pid:     ChildPid,
    tail:    Arc<Mutex<OutputRing>>,
    readers: Vec<JoinHandle<()>>,
}

impl ProcessChild {
    /// Spawn `command` with piped stdout/stderr.
    ///
    /// # Errors
    ///
    /// Returns I/O errors from the underlying spawn.
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
