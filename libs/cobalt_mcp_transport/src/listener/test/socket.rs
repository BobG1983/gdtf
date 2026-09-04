use std::{
    error::Error,
    io::{Read, Write},
    net::{Ipv4Addr, TcpStream},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::Duration,
};

use cobalt_mcp_protocol::{
    framing::{FrameDecoder, encode},
    message::{HelloFacts, McpRequest, McpResponse, ProtocolVersion, ServerNameNet},
    ports::McpPort,
    timeouts::{NetIoTimeout, NetReplyTimeout, NetTimeouts},
};

use crate::{
    channel::IncomingRequest,
    listener::{bind_listener, run_listener},
};

pub(super) type TestResult = Result<(), Box<dyn Error>>;

// Never fire during a test; reaping has its own suite.
const TEST_TIMEOUTS: NetTimeouts = NetTimeouts::new(
    NetIoTimeout::new(Duration::MAX),
    NetReplyTimeout::new(Duration::MAX),
);

pub(super) fn host_facts() -> HelloFacts {
    HelloFacts::new(
        ProtocolVersion::new(*ProtocolVersion::CURRENT + 41),
        ServerNameNet::new("transport-session-test".to_owned()),
    )
}

pub(super) fn foreign_version() -> ProtocolVersion {
    ProtocolVersion::CURRENT
}

pub(super) fn connected_client() -> Result<(TcpStream, Receiver<IncomingRequest>), Box<dyn Error>> {
    let (tx, rx) = mpsc::channel::<IncomingRequest>();
    let (listener, port) = bind_listener(McpPort::new(0))?;
    thread::spawn(move || run_listener(listener, tx, TEST_TIMEOUTS, host_facts()));
    let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, *port))?;
    Ok((stream, rx))
}

pub(super) fn send(stream: &mut TcpStream, request: &McpRequest) -> TestResult {
    stream.write_all(&encode(request)?)?;
    Ok(())
}

pub(super) fn send_raw(stream: &mut TcpStream, frame: &[u8]) -> TestResult {
    stream.write_all(frame)?;
    Ok(())
}

pub(super) fn read_response(stream: &mut TcpStream) -> Result<McpResponse, Box<dyn Error>> {
    let mut decoder = FrameDecoder::new();
    let mut buf = [0u8; 512];
    loop {
        if let Some(frame) = decoder.next_frame()? {
            return Ok(frame.decode::<McpResponse>()?);
        }
        let read = stream.read(&mut buf)?;
        if read == 0 {
            return Err("the listener closed before a full response arrived".into());
        }
        decoder.push(&buf[..read]);
    }
}

pub(super) fn assert_inbox_empty(inbox: &Receiver<IncomingRequest>, what: &str) {
    match inbox.try_recv() {
        Err(TryRecvError::Empty) => {}
        Err(TryRecvError::Disconnected) => {
            unreachable!("the listener holds the sender for the whole test, so it is connected")
        }
        Ok(incoming) => unreachable!(
            "{what} must never reach the host inbox, but the host received {:?}",
            incoming.request()
        ),
    }
}
