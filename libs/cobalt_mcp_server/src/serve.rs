//! Stdio serve loop for the MCP bridge.

use std::{
    io::{self, BufRead, BufReader, Write},
    sync::mpsc::{self, RecvTimeoutError, Sender},
    thread,
    time::Instant,
};

use crate::{
    hosts::{
        HostRegistry, HostSet,
        runtime::{pairs, runtimes},
    },
    lifecycle::{SweepClock, SweepDue, SweepSchedule},
    mcp::ServerIdentity,
    rpc::dispatch,
};

// One newline-delimited JSON-RPC request read from input.
struct RequestLine(String);

impl core::ops::Deref for RequestLine {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Whether the serve loop keeps reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answered {
    Continue,
    Stop,
}

/// Run the MCP server for these hosts on stdin/stdout until EOF.
pub fn run_stdio(identity: &ServerIdentity, registry: HostRegistry) {
    let mut runtimes = runtimes(&registry);
    let mut sweep = SweepClock::started(SweepSchedule::from_registry(&registry));
    {
        let mut hosts = HostSet::new(registry, pairs(&mut runtimes));
        let stdout = io::stdout();
        let mut writer = stdout.lock();
        run_loop(
            BufReader::new(io::stdin()),
            &mut writer,
            identity,
            &mut hosts,
            &mut sweep,
        );
    }
    for runtime in &mut runtimes {
        let _ = runtime.stop_children();
    }
}

/// Answer newline-delimited JSON-RPC requests, sweeping dead children between them.
pub fn run_loop<R, W>(
    reader: R,
    writer: &mut W,
    identity: &ServerIdentity,
    hosts: &mut HostSet<'_>,
    sweep: &mut SweepClock,
) where
    R: BufRead + Send + 'static,
    W: Write,
{
    let (sender, requests) = mpsc::channel();
    thread::spawn(move || read_requests(reader, &sender));
    loop {
        for host in sweep.take_due(SweepDue::new(Instant::now())) {
            if let Some(pair) = hosts.pair(&host) {
                pair.lifecycle().reap_dead_child();
            }
        }
        match requests.recv_timeout(*sweep.tick()) {
            Ok(line) => {
                if matches!(answer(&line, writer, identity, hosts), Answered::Stop) {
                    return;
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

// Read lines off the blocking input so the serve loop keeps its own clock.
fn read_requests<R: BufRead>(mut reader: R, sender: &Sender<RequestLine>) {
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        if sender.send(RequestLine(line.clone())).is_err() {
            return;
        }
    }
}

// Dispatch one request and write its response.
fn answer<W: Write>(
    line: &RequestLine,
    writer: &mut W,
    identity: &ServerIdentity,
    hosts: &mut HostSet<'_>,
) -> Answered {
    let Some(response) = dispatch(line, identity, hosts) else {
        return Answered::Continue;
    };
    if writeln!(writer, "{response}").is_err() || writer.flush().is_err() {
        return Answered::Stop;
    }
    Answered::Continue
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use cobalt_mcp_protocol::{
        message::{McpRequest, McpResponse},
        ports::McpPort,
    };
    use serde_json::Value;

    use super::run_loop;
    use crate::{
        error::McpError,
        hosts::{HostName, HostPair, HostSet, test_support::two_hosts},
        lifecycle::{
            HostLifecycle, InstanceId, LaunchOutcome, LaunchSpec, OutputTail, RecordedInstance,
            StopOutcome, SweepClock, SweepSchedule, TailLines, WorkingDir,
        },
        link::McpLink,
        mcp::{ServerIdentity, ServerName, ServerVersion},
    };

    struct DeadLink;

    impl McpLink for DeadLink {
        fn request(&mut self, _request: McpRequest) -> Result<McpResponse, McpError> {
            Err(McpError::Disconnected)
        }
    }

    struct DeadLifecycle;

    impl HostLifecycle for DeadLifecycle {
        fn launch(&mut self, _port: McpPort, _spec: &LaunchSpec) -> LaunchOutcome {
            unreachable!("the transport test never launches");
        }

        fn stop(&mut self, _port: McpPort) -> StopOutcome {
            StopOutcome::NotRunning
        }

        fn stop_instance(&mut self, _instance: &InstanceId) -> StopOutcome {
            StopOutcome::NotRunning
        }

        fn stop_owned(&mut self) -> StopOutcome {
            StopOutcome::NotRunning
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
            None
        }

        fn instance_output(&self, _instance: &InstanceId, _max: TailLines) -> Option<OutputTail> {
            None
        }
    }

    #[test]
    fn loops_over_newline_delimited_requests() {
        let input = concat!(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
            "\n",
        );
        let mut out: Vec<u8> = Vec::new();
        let (mut alpha_link, mut beta_link) = (DeadLink, DeadLink);
        let (mut alpha_life, mut beta_life) = (DeadLifecycle, DeadLifecycle);
        let registry = two_hosts();
        let mut sweep = SweepClock::started(SweepSchedule::from_registry(&registry));
        let mut hosts = HostSet::new(
            registry,
            vec![
                (
                    HostName::new("alpha".to_owned()),
                    HostPair::new(&mut alpha_link, &mut alpha_life),
                ),
                (
                    HostName::new("beta".to_owned()),
                    HostPair::new(&mut beta_link, &mut beta_life),
                ),
            ],
        );
        let identity = ServerIdentity::new(
            ServerName::new("sample-bridge".to_owned()),
            ServerVersion::new("9.9.9".to_owned()),
        );
        run_loop(
            Cursor::new(input.as_bytes()),
            &mut out,
            &identity,
            &mut hosts,
            &mut sweep,
        );
        let text = String::from_utf8(out).unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2, "got: {text}");
        let first: Value = serde_json::from_str(lines[0]).unwrap_or(Value::Null);
        let second: Value = serde_json::from_str(lines[1]).unwrap_or(Value::Null);
        assert_eq!(first["id"], serde_json::json!(1));
        assert!(first["result"]["protocolVersion"].is_string());
        assert_eq!(second["id"], serde_json::json!(2));
        assert!(second["result"]["tools"].is_array());
    }
}
