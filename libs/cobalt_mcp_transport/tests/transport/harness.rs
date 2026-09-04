use std::{
    error::Error,
    io::{Read, Write},
    net::TcpStream,
    sync::mpsc::{self, Receiver},
    thread,
};

use cobalt_mcp_protocol::{
    framing::{FrameDecoder, encode},
    message::{HelloFacts, ProtocolVersion, QaRequest, QaResponse, ServerNameNet},
    ports::McpPort,
    timeouts::NetTimeouts,
};
use cobalt_mcp_transport::{IncomingRequest, bind_listener, run_listener};

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

pub(crate) fn spawn_listener(
    timeouts: NetTimeouts,
) -> Result<(McpPort, Receiver<IncomingRequest>), Box<dyn Error>> {
    let (tx, rx) = mpsc::channel::<IncomingRequest>();
    let (listener, port) = bind_listener(McpPort::new(0))?;
    thread::spawn(move || run_listener(listener, tx, timeouts, test_facts()));
    Ok((port, rx))
}

pub(crate) fn test_facts() -> HelloFacts {
    HelloFacts::new(
        ProtocolVersion::CURRENT,
        ServerNameNet::new("test".to_owned()),
    )
}

pub(crate) fn host_reply_facts() -> HelloFacts {
    HelloFacts::new(
        ProtocolVersion::CURRENT,
        ServerNameNet::new("test-host-inbox".to_owned()),
    )
}

pub(crate) fn spawn_fake_host_side(inbox: Receiver<IncomingRequest>) {
    thread::spawn(move || {
        while let Ok(incoming) = inbox.recv() {
            incoming.respond(QaResponse::HelloOk(host_reply_facts()));
        }
    });
}

pub(crate) fn write_request(stream: &mut TcpStream, request: &QaRequest) -> TestResult {
    let frame = encode(request)?;
    stream.write_all(&frame)?;
    Ok(())
}

pub(crate) fn read_response(stream: &mut TcpStream) -> Result<QaResponse, Box<dyn Error>> {
    let mut decoder = FrameDecoder::new();
    let mut buf = [0u8; 512];
    loop {
        if let Some(frame) = decoder.next_frame()? {
            return Ok(frame.decode::<QaResponse>()?);
        }
        let read = stream.read(&mut buf)?;
        assert!(read > 0, "server closed before a full response arrived");
        decoder.push(&buf[..read]);
    }
}
