//! Reconnect and retarget behaviour for the QA client.

use std::{
    io::{Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};

use cobalt_mcp_protocol::{
    command::CommandCatalogue,
    framing::{FrameDecoder, encode},
    message::{
        HelloFacts, McpRequest, McpResponse, McpSessionError, ProtocolVersion, ServerNameNet,
    },
};
use cobalt_mcp_server::{EnvVarName, McpClient, McpLink, McpPort};

fn bind_loopback() -> (TcpListener, u16) {
    let listener = match TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) {
        Ok(listener) => listener,
        Err(err) => unreachable!("the test can bind a loopback listener: {err}"),
    };
    let port = match listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(err) => unreachable!("the listener has a local address: {err}"),
    };
    (listener, port)
}

fn catalogue_reply() -> McpResponse {
    McpResponse::Catalogue(CommandCatalogue::new(
        ServerNameNet::new("fake-game".to_owned()),
        Vec::new(),
    ))
}

fn answer(request: &McpRequest) -> McpResponse {
    match request {
        McpRequest::Hello(_) => McpResponse::HelloOk(HelloFacts::new(
            ProtocolVersion::CURRENT,
            ServerNameNet::new("reconnect-stub".to_owned()),
        )),
        _ => catalogue_reply(),
    }
}

struct StubConn<'stream> {
    stream:  &'stream mut TcpStream,
    decoder: FrameDecoder,
}

impl StubConn<'_> {
    fn answer_one(&mut self) -> bool {
        let mut buf = [0u8; 1024];
        loop {
            match self.decoder.next_frame() {
                Ok(Some(frame)) => {
                    let Ok(request) = frame.decode::<McpRequest>() else {
                        return false;
                    };
                    let Ok(out) = encode(&answer(&request)) else {
                        return false;
                    };
                    return self.stream.write_all(&out).is_ok();
                }
                Ok(None) => {}
                Err(_) => return false,
            }
            let read = match self.stream.read(&mut buf) {
                Ok(0) | Err(_) => return false,
                Ok(count) => count,
            };
            self.decoder.push(&buf[..read]);
        }
    }
}

fn answer_frames(stream: &mut TcpStream, frames: usize) {
    let mut conn = StubConn {
        stream,
        decoder: FrameDecoder::new(),
    };
    for _ in 0..frames {
        if !conn.answer_one() {
            return;
        }
    }
}

fn answer_until_gone(stream: &mut TcpStream) {
    let mut conn = StubConn {
        stream,
        decoder: FrameDecoder::new(),
    };
    while conn.answer_one() {}
}

fn request_catalogue(client: &mut McpClient) -> Result<McpResponse, cobalt_mcp_server::McpError> {
    client.request(McpRequest::Catalogue)
}

#[test]
fn a_reused_connection_the_game_closed_reconnects_and_succeeds() {
    let (listener, port) = bind_loopback();
    let server = thread::spawn(move || {
        for _ in 0..2 {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            answer_frames(&mut stream, 2);
        }
    });

    let mut client = McpClient::new(
        McpPort::new(port),
        EnvVarName::new("SAMPLE_CHANNEL".to_owned()),
    );

    let first = request_catalogue(&mut client);
    assert!(
        matches!(first, Ok(McpResponse::Catalogue(_))),
        "the first request opens a fresh connection and succeeds: {first:?}"
    );

    let second = request_catalogue(&mut client);
    assert!(
        matches!(second, Ok(McpResponse::Catalogue(_))),
        "a request on a connection the game already closed must reconnect and succeed, \
         not surface the dead socket: {second:?}"
    );

    drop(client);
    drop(server.join());
}

#[test]
fn retarget_on_the_same_port_invalidates_the_connection() {
    let (listener, port) = bind_loopback();
    let connections = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&connections);
    let server = thread::spawn(move || {
        for _ in 0..2 {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            seen.fetch_add(1, Ordering::SeqCst);
            answer_until_gone(&mut stream);
        }
    });

    let mut client = McpClient::new(
        McpPort::new(port),
        EnvVarName::new("SAMPLE_CHANNEL".to_owned()),
    );

    let first = request_catalogue(&mut client);
    assert!(
        matches!(first, Ok(McpResponse::Catalogue(_))),
        "the first request succeeds on connection 1: {first:?}"
    );
    assert_eq!(
        connections.load(Ordering::SeqCst),
        1,
        "one connection so far"
    );

    client.retarget(McpPort::new(port));

    let second = request_catalogue(&mut client);
    assert!(
        matches!(second, Ok(McpResponse::Catalogue(_))),
        "the post-relaunch request succeeds: {second:?}"
    );
    assert_eq!(
        connections.load(Ordering::SeqCst),
        2,
        "retarget must have forced a fresh connection to the relaunched process, not \
         reused the old socket"
    );

    drop(client);
    drop(server.join());
}

#[test]
fn an_unreachable_game_fails_promptly_without_retrying_forever() {
    let (listener, port) = bind_loopback();
    drop(listener);

    // Returning at all is the proof: an unbounded reconnect loop would hang right here.
    let mut client = McpClient::new(
        McpPort::new(port),
        EnvVarName::new("SAMPLE_CHANNEL".to_owned()),
    );
    let result = request_catalogue(&mut client);

    assert!(
        result.is_err(),
        "a request to a port with no listener must error, got: {result:?}"
    );
    assert!(
        !matches!(result, Ok(McpResponse::Error(McpSessionError::Malformed))),
        "an unreachable game is a link error, not a protocol rejection"
    );
}
