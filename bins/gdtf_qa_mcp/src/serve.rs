use std::io::{self, BufRead, Write};

use crate::{
    hosts::{HostPair, HostSet, QaHost},
    lifecycle::{CargoSpawner, HostLifecycle, HostManager},
    link::QaClient,
    rpc::dispatch,
};

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
    let _ = game_lifecycle.stop_owned();
    let _ = editor_lifecycle.stop_owned();
}

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

    use gdtf_qa_protocol::message::{QaRequest, QaResponse};
    use serde_json::Value;

    use super::run_loop;
    use crate::{
        error::McpError,
        hosts::{HostPair, HostSet},
        lifecycle::{
            HostLifecycle, LaunchOutcome, LaunchSpec, OutputTail, StopOutcome, TailLines,
            WorkingDir,
        },
        link::{QaLink, QaPort},
    };

            struct DeadLink;

    impl QaLink for DeadLink {
        fn request(&mut self, _request: QaRequest) -> Result<QaResponse, McpError> {
            Err(McpError::Disconnected)
        }
    }

            struct DeadLifecycle;

    impl HostLifecycle for DeadLifecycle {
        fn launch(&mut self, _port: QaPort, _spec: &LaunchSpec) -> LaunchOutcome {
            unreachable!("the transport test never launches");
        }

        fn stop(&mut self, _port: QaPort) -> StopOutcome {
            StopOutcome::NotRunning
        }

        fn stop_owned(&mut self) -> StopOutcome {
            StopOutcome::NotRunning
        }

        fn child_working_dir(&self) -> Option<WorkingDir> {
            None
        }

        fn child_output(&self, _max: TailLines) -> Option<OutputTail> {
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
        let (mut game_link, mut editor_link) = (DeadLink, DeadLink);
        let (mut game_life, mut editor_life) = (DeadLifecycle, DeadLifecycle);
        let mut hosts = HostSet::new(
            HostPair::new(&mut game_link, &mut game_life),
            HostPair::new(&mut editor_link, &mut editor_life),
        );
        run_loop(Cursor::new(input.as_bytes()), &mut out, &mut hosts);
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
