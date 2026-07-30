//! Loopback integration — the real [`QaClient`] against a bevy-free fake game server
//! (GTW-741).
//!
//! A small test-only TCP listener speaks the protocol crate's REAL framing (reusing
//! [`encode`] + [`FrameDecoder`], never reimplementing it). The real [`QaClient`],
//! driven through the real [`dispatch`], connects to it and round-trips one
//! request/response pair — proving the client + framing path end-to-end with NO Bevy in
//! the test.

use std::{
    io::{Read, Write},
    net::{Ipv4Addr, TcpListener},
    thread,
};

use gdtf_qa_mcp::{
    HostLifecycle, HostPair, HostSet, LaunchOutcome, LaunchSpec, McpError, QaClient, QaLink,
    QaPort, StopOutcome, WorkingDir, dispatch,
};
use gdtf_qa_protocol::{
    envelope::{QaError, QaRequest, QaResponse},
    framing::{FrameDecoder, encode},
    view::{AppFlowView, AppStateNet, BattleActiveNet, CaughtUpNet},
};
use serde_json::Value;

/// A lifecycle the loopback test never invokes — only present so `dispatch` has its
/// argument.
struct NoLifecycle;

impl HostLifecycle for NoLifecycle {
    fn launch(&mut self, _port: QaPort, _spec: &LaunchSpec) -> LaunchOutcome {
        unreachable!("the loopback test never launches");
    }

    fn stop(&mut self) -> StopOutcome {
        StopOutcome::NotRunning
    }

    fn child_working_dir(&self) -> Option<WorkingDir> {
        None
    }
}

/// An editor link the loopback test never reaches — the game tool it calls resolves to the
/// game's pair, so this one only fills the set's other half.
struct DeadLink;

impl QaLink for DeadLink {
    fn request(&mut self, _request: QaRequest) -> Result<QaResponse, McpError> {
        Err(McpError::Disconnected)
    }
}

/// Bind a loopback listener on an OS-assigned port and serve exactly one framed request
/// on a background thread, replying with a framed `QaResponse`. Returns the bound port.
fn spawn_fake_game() -> u16 {
    let listener = match TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) {
        Ok(listener) => listener,
        Err(err) => unreachable!("the test can bind a loopback listener: {err}"),
    };
    let port = match listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(err) => unreachable!("the listener has a local address: {err}"),
    };
    thread::spawn(move || serve_one(&listener));
    port
}

/// Accept one client, decode one framed [`QaRequest`], and frame back a [`QaResponse`].
fn serve_one(listener: &TcpListener) {
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
        let frame = match decoder.next_frame() {
            Ok(Some(frame)) => frame,
            Ok(None) => continue,
            Err(_) => return,
        };
        let response = match frame.decode::<QaRequest>() {
            Ok(QaRequest::GetAppFlow) => QaResponse::AppFlow(AppFlowView::new(
                AppStateNet::Running,
                BattleActiveNet::new(true),
                Vec::new(),
                CaughtUpNet::new(true),
                None,
                None,
            )),
            Ok(_) => QaResponse::Error(QaError::BadRequest),
            Err(_) => return,
        };
        if let Ok(out) = encode(&response) {
            drop(stream.write_all(&out));
        }
        return;
    }
}

/// The real client + real framing carry a `tools/call app_flow` to the fake server and
/// back, and the reply renders as the expected content.
#[test]
fn app_flow_round_trips_through_the_real_client() {
    let port = spawn_fake_game();
    let mut client = QaClient::new(QaPort::new(port));
    let mut unused_editor = DeadLink;
    let (mut game_life, mut editor_life) = (NoLifecycle, NoLifecycle);
    let mut hosts = HostSet::new(
        HostPair::new(&mut client, &mut game_life),
        HostPair::new(&mut unused_editor, &mut editor_life),
    );
    let line = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"app_flow","arguments":{}}}"#;
    let Some(response) = dispatch(line, &mut hosts) else {
        unreachable!("tools/call yields a response");
    };
    let parsed: Value = serde_json::from_str(&response).unwrap_or(Value::Null);
    assert_eq!(parsed["result"]["isError"], serde_json::json!(false));
    let Some(text) = parsed["result"]["content"][0]["text"].as_str() else {
        unreachable!("the app_flow reply renders as text content: {parsed}");
    };
    assert!(text.contains("Running"), "rendered: {text}");
    assert!(text.contains("battle_active"), "rendered: {text}");
}
