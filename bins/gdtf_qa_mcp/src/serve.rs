//! The blocking stdio transport loop (GTW-741, GTW-745, GTW-808).
//!
//! MCP over stdio is newline-delimited JSON-RPC 2.0: each message is one line of JSON on
//! stdin (no embedded newlines), and each response is one line on stdout. [`run_stdio`]
//! reads lines, hands each to [`dispatch()`], and writes back any
//! response line — blocking `std` I/O, no async runtime. It stops on stdin EOF.
//!
//! Stdin EOF is the MCP host's normal shutdown (the client closed the pipe). Because the
//! host may have launched BOTH children — the game and the content editor — [`run_stdio`]
//! stops each of them gracefully before returning, so neither outlives the MCP; and each
//! [`HostManager`]'s `Drop` force-kills any child that somehow survives that (e.g. on an
//! unwind).

use std::io::{self, BufRead, Write};

use crate::{
    hosts::{HostPair, HostSet, QaHost},
    lifecycle::{CargoSpawner, HostLifecycle, HostManager},
    link::QaClient,
    rpc::dispatch,
};

/// Run the MCP stdio server against both hosts' real links + lifecycles until stdin
/// closes, then stop any children the host launched.
///
/// Each child's connection is opened lazily on the first forwarding `tools/call` for that
/// host, so `initialize` and `tools/list` answer before either process is up. The
/// `launch_game` / `stop_game` and `launch_editor` / `stop_editor` tools drive the two
/// [`HostManager`]s, which own the child processes — separately, so a game and an editor
/// can be running at the same time.
pub fn run_stdio() {
    let mut game_link = QaClient::for_host(QaHost::Game);
    let mut editor_link = QaClient::for_host(QaHost::Editor);
    let mut game_lifecycle = HostManager::with_config(
        Box::new(CargoSpawner::new()),
        QaHost::Game.lifecycle_config(),
    );
    let mut editor_lifecycle = HostManager::with_config(
        Box::new(CargoSpawner::new()),
        QaHost::Editor.lifecycle_config(),
    );
    {
        let mut hosts = HostSet::new(
            HostPair::new(&mut game_link, &mut game_lifecycle),
            HostPair::new(&mut editor_link, &mut editor_lifecycle),
        );
        let stdin = io::stdin();
        let stdout = io::stdout();
        let reader = stdin.lock();
        let mut writer = stdout.lock();
        run_loop(reader, &mut writer, &mut hosts);
    }
    // Stdin closed: stop the children this host launched, gracefully, so neither outlives
    // us. BOTH are stopped — a game left running because only the editor's stop ran is
    // exactly the orphan this exists to prevent.
    let _ = game_lifecycle.stop();
    let _ = editor_lifecycle.stop();
}

/// The transport core, generic over its byte streams so it is testable without real
/// stdin/stdout.
///
/// Reads one line at a time; for each, dispatches and — when there is a response — writes
/// it followed by a newline and flushes (so the peer sees each message immediately).
/// Returns on EOF, a read error, or a write error.
pub fn run_loop<R: BufRead, W: Write>(mut reader: R, writer: &mut W, hosts: &mut HostSet<'_>) {
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let Some(response) = dispatch(&line, hosts) else {
            continue;
        };
        if writeln!(writer, "{response}").is_err() || writer.flush().is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use gdtf_qa_protocol::envelope::{QaRequest, QaResponse};
    use serde_json::Value;

    use super::run_loop;
    use crate::{
        error::McpError,
        hosts::{HostPair, HostSet},
        lifecycle::{HostLifecycle, LaunchOutcome, LaunchSpec, StopOutcome},
        link::{QaLink, QaPort},
    };

    /// A link that always fails — the loop's `initialize` / `tools/list` handling never
    /// touches it, so the transport can be exercised with no socket.
    struct DeadLink;

    impl QaLink for DeadLink {
        fn request(&mut self, _request: QaRequest) -> Result<QaResponse, McpError> {
            Err(McpError::Disconnected)
        }
    }

    /// A lifecycle the transport test never invokes (no launch / stop tool in the
    /// fixture) — it only exists so `run_loop` has its argument.
    struct DeadLifecycle;

    impl HostLifecycle for DeadLifecycle {
        fn launch(&mut self, _port: QaPort, _spec: &LaunchSpec) -> LaunchOutcome {
            unreachable!("the transport test never launches");
        }

        fn stop(&mut self) -> StopOutcome {
            StopOutcome::NotRunning
        }
    }

    /// Two newline-delimited requests in produce two newline-delimited responses out, in
    /// order, each echoing its own id — the MCP stdio framing.
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
        let (mut game_link, mut editor_link) = (DeadLink, DeadLink);
        let (mut game_life, mut editor_life) = (DeadLifecycle, DeadLifecycle);
        let mut hosts = HostSet::new(
            HostPair::new(&mut game_link, &mut game_life),
            HostPair::new(&mut editor_link, &mut editor_life),
        );
        run_loop(Cursor::new(input.as_bytes()), &mut out, &mut hosts);
        let text = String::from_utf8(out).unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        // The notification (no id) produced no line; the two requests produced one each.
        assert_eq!(lines.len(), 2, "got: {text}");
        let first: Value = serde_json::from_str(lines[0]).unwrap_or(Value::Null);
        let second: Value = serde_json::from_str(lines[1]).unwrap_or(Value::Null);
        assert_eq!(first["id"], serde_json::json!(1));
        assert!(first["result"]["protocolVersion"].is_string());
        assert_eq!(second["id"], serde_json::json!(2));
        assert!(second["result"]["tools"].is_array());
    }
}
