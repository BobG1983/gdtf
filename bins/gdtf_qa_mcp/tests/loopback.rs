//! Loopback tests for catalogue requests against a fake TCP game.

use std::{
    io::{Read, Write},
    net::{Ipv4Addr, TcpListener},
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use gdtf_qa_mcp::{
    HostLifecycle, HostPair, HostSet, LaunchOutcome, LaunchSpec, McpError, OutputTail, QaClient,
    QaLink, QaPort, StopOutcome, TailLines, WorkingDir, dispatch,
};
use gdtf_qa_protocol::{
    command::CommandCatalogue,
    framing::{FrameDecoder, encode},
    message::{HelloFacts, ProtocolVersion, QaError, QaRequest, QaResponse, ServerNameNet},
};
use serde_json::Value;

struct NoLifecycle;

impl HostLifecycle for NoLifecycle {
    fn launch(&mut self, _port: QaPort, _spec: &LaunchSpec) -> LaunchOutcome {
        unreachable!("the loopback test never launches");
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

struct DeadLink;

impl QaLink for DeadLink {
    fn request(&mut self, _request: QaRequest) -> Result<QaResponse, McpError> {
        Err(McpError::Disconnected)
    }
}

fn spawn_fake_game() -> (u16, Receiver<QaRequest>) {
    spawn_fake_game_with(answer)
}

fn spawn_fake_game_with(answerer: fn(&QaRequest) -> QaResponse) -> (u16, Receiver<QaRequest>) {
    let listener = match TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) {
        Ok(listener) => listener,
        Err(err) => unreachable!("the test can bind a loopback listener: {err}"),
    };
    let port = match listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(err) => unreachable!("the listener has a local address: {err}"),
    };
    let (seen_tx, seen_rx) = mpsc::channel();
    thread::spawn(move || serve_one(&listener, &seen_tx, answerer));
    (port, seen_rx)
}

fn serve_one(
    listener: &TcpListener,
    seen: &Sender<QaRequest>,
    answerer: fn(&QaRequest) -> QaResponse,
) {
    let Ok((mut stream, _)) = listener.accept() else {
        return;
    };
    let mut decoder = FrameDecoder::new();
    let mut buf = [0u8; 1024];
    loop {
        let read = match stream.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(count) => count,
        };
        decoder.push(&buf[..read]);
        loop {
            let frame = match decoder.next_frame() {
                Ok(Some(frame)) => frame,
                Ok(None) => break,
                Err(_) => return,
            };
            let Ok(request) = frame.decode::<QaRequest>() else {
                return;
            };
            drop(seen.send(request.clone()));
            let response = answerer(&request);
            if let Ok(out) = encode(&response)
                && stream.write_all(&out).is_err()
            {
                return;
            }
        }
    }
}

fn answer(request: &QaRequest) -> QaResponse {
    match request {
        QaRequest::Hello(version) if *version == ProtocolVersion::CURRENT => {
            QaResponse::HelloOk(HelloFacts::new(
                ProtocolVersion::CURRENT,
                ServerNameNet::new("fake-game".to_owned()),
            ))
        }
        QaRequest::Hello(_) => QaResponse::Error(QaError::VersionMismatch),
        QaRequest::Catalogue => QaResponse::Catalogue(CommandCatalogue::new(
            ServerNameNet::new("fake-game".to_owned()),
            Vec::new(),
        )),
        QaRequest::Run(_) => QaResponse::Error(QaError::Malformed),
    }
}

#[test]
fn a_catalogue_round_trips_through_the_real_client() {
    let (port, seen) = spawn_fake_game();
    let mut client = QaClient::new(QaPort::new(port));
    let mut unused_editor = DeadLink;
    let (mut game_life, mut editor_life) = (NoLifecycle, NoLifecycle);
    let mut hosts = HostSet::new(
        HostPair::new(&mut client, &mut game_life),
        HostPair::new(&mut unused_editor, &mut editor_life),
    );
    let line = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"commands","arguments":{}}}"#;
    let Some(response) = dispatch(line, &mut hosts) else {
        unreachable!("tools/call yields a response");
    };
    let parsed: Value = serde_json::from_str(&response).unwrap_or(Value::Null);
    assert_eq!(parsed["result"]["isError"], serde_json::json!(false));
    let Some(text) = parsed["result"]["content"][0]["text"].as_str() else {
        unreachable!("the catalogue reply renders as text content: {parsed}");
    };
    assert!(text.contains("fake-game"), "rendered: {text}");
    assert!(text.contains("commands"), "rendered: {text}");

    let first = seen.recv().ok();
    assert!(
        matches!(first, Some(QaRequest::Hello(version)) if version == ProtocolVersion::CURRENT),
        "the client's FIRST frame on a fresh connection must be the handshake, got {first:?}"
    );
    let second = seen.recv().ok();
    assert!(
        matches!(second, Some(QaRequest::Catalogue)),
        "the tool's request must follow the handshake, got {second:?}"
    );
}

const fn refuse_everything(_request: &QaRequest) -> QaResponse {
    QaResponse::Error(QaError::VersionMismatch)
}

#[test]
fn a_refused_handshake_fails_the_request_and_sends_nothing_else() {
    let (port, seen) = spawn_fake_game_with(refuse_everything);
    let mut client = QaClient::new(QaPort::new(port));

    let result = client.request(QaRequest::Catalogue);
    assert!(
        matches!(result, Err(McpError::Handshake(QaError::VersionMismatch))),
        "a version-mismatched child must surface as a handshake error, got {result:?}"
    );

    let first = seen.recv().ok();
    assert!(
        matches!(first, Some(QaRequest::Hello(_))),
        "the handshake is what the server saw, got {first:?}"
    );
    assert!(
        seen.recv().is_err(),
        "nothing may follow a refused handshake — the connection is dropped, not reused"
    );
}
