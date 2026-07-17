//! The blocking stdio transport loop (GTW-741, GTW-745).
//!
//! MCP over stdio is newline-delimited JSON-RPC 2.0: each message is one line of JSON on
//! stdin (no embedded newlines), and each response is one line on stdout. [`run_stdio`]
//! reads lines, hands each to [`dispatch()`], and writes back any
//! response line — blocking `std` I/O, no async runtime. It stops on stdin EOF.
//!
//! Stdin EOF is the MCP host's normal shutdown (the client closed the pipe). Because the
//! host may have launched the game as a child, [`run_stdio`] stops that child gracefully
//! before returning, so the game never outlives the MCP — and the [`GameManager`]'s
//! `Drop` force-kills any child that somehow survives that (e.g. on an unwind).

use std::io::{self, BufRead, Write};

use crate::{
    game::{GameClient, GameLink},
    lifecycle::{CargoSpawner, GameLifecycle, GameManager},
    rpc::dispatch,
};

/// Run the MCP stdio server against the real game link + lifecycle until stdin closes,
/// then stop any child the host launched.
///
/// The game connection is opened lazily on the first forwarding `tools/call`, so
/// `initialize` and `tools/list` answer even before the game is up. The `launch_game` /
/// `stop_game` tools drive the [`GameManager`], which owns the child process.
pub fn run_stdio() {
    let mut game = GameClient::from_env();
    let mut lifecycle = GameManager::new(Box::new(CargoSpawner::new()));
    {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let reader = stdin.lock();
        let mut writer = stdout.lock();
        run_loop(reader, &mut writer, &mut game, &mut lifecycle);
    }
    // Stdin closed: stop a child this host launched, gracefully, so it does not outlive us.
    let _ = lifecycle.stop();
}

/// The transport core, generic over its byte streams so it is testable without real
/// stdin/stdout.
///
/// Reads one line at a time; for each, dispatches and — when there is a response — writes
/// it followed by a newline and flushes (so the peer sees each message immediately).
/// Returns on EOF, a read error, or a write error.
pub fn run_loop<R: BufRead, W: Write>(
    mut reader: R,
    writer: &mut W,
    game: &mut dyn GameLink,
    lifecycle: &mut dyn GameLifecycle,
) {
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let Some(response) = dispatch(&line, game, lifecycle) else {
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
        game::{GameLink, GamePort},
        lifecycle::{GameLifecycle, LaunchOutcome, StopOutcome},
    };

    /// A link that always fails — the loop's `initialize` / `tools/list` handling never
    /// touches it, so the transport can be exercised with no socket.
    struct DeadLink;

    impl GameLink for DeadLink {
        fn request(&mut self, _request: QaRequest) -> Result<QaResponse, McpError> {
            Err(McpError::Disconnected)
        }
    }

    /// A lifecycle the transport test never invokes (no `launch_game` / `stop_game` in the
    /// fixture) — it only exists so `run_loop` has its argument.
    struct DeadLifecycle;

    impl GameLifecycle for DeadLifecycle {
        fn launch(&mut self, _port: GamePort) -> LaunchOutcome {
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
        run_loop(
            Cursor::new(input.as_bytes()),
            &mut out,
            &mut DeadLink,
            &mut DeadLifecycle,
        );
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
