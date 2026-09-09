//! Loopback tests for catalogue requests against a fake TCP game.

use core::time::Duration;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use cobalt_mcp_protocol::{
    command::CommandCatalogue,
    framing::{FrameDecoder, encode},
    message::{
        HelloFacts, McpRequest, McpResponse, McpSessionError, ProtocolVersion, ServerNameNet,
    },
};
use cobalt_mcp_server::{
    CargoPackage, FeatureList, HostLifecycle, HostName, HostPair, HostRegistry, HostSet,
    InstanceId, LaunchOutcome, LaunchPolicy, LaunchSpec, LifecycleConfig, LinkTimeout, McpClient,
    McpError, McpHostSpec, McpLink, McpPort, OutputTail, RecordedInstance, ServerIdentity,
    ServerName, ServerVersion, StopOutcome, TailLines, WorkingDir, dispatch,
};
use serde_json::Value;

use crate::ports::{bind_loopback, port_of};

/// A read wait no run reaches, so a loaded machine cannot cut a reply short.
const NO_READ_DEADLINE: LinkTimeout = LinkTimeout::new(Duration::MAX);

// One host registered under `name`, listening on `port`.
fn registered(name: &str, port: u16) -> McpHostSpec {
    McpHostSpec::new(
        HostName::new(name.to_owned()),
        CargoPackage::new(format!("{name}_package")),
        FeatureList::default(),
        McpPort::new(port),
        None,
        LifecycleConfig::defaults_with_policy(LaunchPolicy::Reuse),
    )
}

fn identity() -> ServerIdentity {
    ServerIdentity::new(
        ServerName::new("sample-bridge".to_owned()),
        ServerVersion::new("9.9.9".to_owned()),
    )
}

struct NoLifecycle;

impl HostLifecycle for NoLifecycle {
    fn launch(&mut self, _port: McpPort, _spec: &LaunchSpec) -> LaunchOutcome {
        unreachable!("the loopback test never launches");
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

struct DeadLink;

impl McpLink for DeadLink {
    fn request(&mut self, _request: McpRequest) -> Result<McpResponse, McpError> {
        Err(McpError::Disconnected)
    }
}

fn spawn_fake_game() -> (u16, Receiver<McpRequest>) {
    spawn_fake_game_with(answer)
}

fn spawn_fake_game_with(answerer: fn(&McpRequest) -> McpResponse) -> (u16, Receiver<McpRequest>) {
    let listener = bind_loopback();
    let port = port_of(&listener);
    let (seen_tx, seen_rx) = mpsc::channel();
    thread::spawn(move || serve_one(&listener, &seen_tx, answerer));
    (port, seen_rx)
}

fn serve_one(
    listener: &TcpListener,
    seen: &Sender<McpRequest>,
    answerer: fn(&McpRequest) -> McpResponse,
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
            let Ok(request) = frame.decode::<McpRequest>() else {
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

fn answer(request: &McpRequest) -> McpResponse {
    match request {
        McpRequest::Hello(version) if *version == ProtocolVersion::CURRENT => {
            McpResponse::HelloOk(HelloFacts::new(
                ProtocolVersion::CURRENT,
                ServerNameNet::new("fake-game".to_owned()),
            ))
        }
        McpRequest::Hello(_) => McpResponse::Error(McpSessionError::VersionMismatch),
        McpRequest::Catalogue => McpResponse::Catalogue(CommandCatalogue::new(
            ServerNameNet::new("fake-game".to_owned()),
            Vec::new(),
        )),
        McpRequest::Run(_) => McpResponse::Error(McpSessionError::Malformed),
    }
}

#[test]
fn a_catalogue_round_trips_through_the_real_client() {
    let (port, seen) = spawn_fake_game();
    let mut client = McpClient::with_timeout(McpPort::new(port), NO_READ_DEADLINE);
    let mut unused_second = DeadLink;
    let (mut first_life, mut second_life) = (NoLifecycle, NoLifecycle);
    let mut hosts = HostSet::new(
        HostRegistry::new(vec![registered("alpha", port), registered("beta", 0)]),
        vec![
            (
                HostName::new("alpha".to_owned()),
                HostPair::new(&mut client, &mut first_life),
            ),
            (
                HostName::new("beta".to_owned()),
                HostPair::new(&mut unused_second, &mut second_life),
            ),
        ],
    );
    let line = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"commands","arguments":{}}}"#;
    let Some(response) = dispatch(line, &identity(), &mut hosts) else {
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
        matches!(first, Some(McpRequest::Hello(version)) if version == ProtocolVersion::CURRENT),
        "the client's FIRST frame on a fresh connection must be the handshake, got {first:?}"
    );
    let second = seen.recv().ok();
    assert!(
        matches!(second, Some(McpRequest::Catalogue)),
        "the tool's request must follow the handshake, got {second:?}"
    );
}

const fn refuse_everything(_request: &McpRequest) -> McpResponse {
    McpResponse::Error(McpSessionError::VersionMismatch)
}

#[test]
fn a_refused_handshake_fails_the_request_and_sends_nothing_else() {
    let (port, seen) = spawn_fake_game_with(refuse_everything);
    let mut client = McpClient::with_timeout(McpPort::new(port), NO_READ_DEADLINE);

    let result = client.request(McpRequest::Catalogue);
    assert!(
        matches!(
            result,
            Err(McpError::Handshake(McpSessionError::VersionMismatch))
        ),
        "a version-mismatched child must surface as a handshake error, got {result:?}"
    );

    let first = seen.recv().ok();
    assert!(
        matches!(first, Some(McpRequest::Hello(_))),
        "the handshake is what the server saw, got {first:?}"
    );
    assert!(
        seen.recv().is_err(),
        "nothing may follow a refused handshake — the connection is dropped, not reused"
    );
}
