//! Probe a port: whether anything holds it, and whether a host answers the handshake.
//!
//! Any decodable answer, a version mismatch included, proves the listener is answering.
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpStream},
};

use cobalt_mcp_protocol::{
    framing::{FrameDecoder, encode},
    message::{McpRequest, McpResponse, ProtocolVersion},
    ports::McpPort,
};

use super::values::{PortListening, ProbeTimeout, Readiness};

const PROBE_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::new(1);

const READ_CHUNK: usize = 1024;

// Whether anything accepts a connection on the port, with no handshake asked for.
#[must_use]
pub(super) fn probe_listening(port: McpPort, timeout: ProbeTimeout) -> PortListening {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, *port));
    match TcpStream::connect_timeout(&addr, *timeout) {
        Ok(_) => PortListening::Listening,
        Err(_) => PortListening::Silent,
    }
}

#[must_use]
pub(super) fn probe_ready(port: McpPort, timeout: ProbeTimeout) -> Readiness {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, *port));
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, *timeout) else {
        return Readiness::NotYet;
    };
    if stream.set_read_timeout(Some(*timeout)).is_err()
        || stream.set_write_timeout(Some(*timeout)).is_err()
    {
        return Readiness::NotYet;
    }
    let Ok(frame) = encode(&McpRequest::Hello(PROBE_PROTOCOL_VERSION)) else {
        return Readiness::NotYet;
    };
    if stream.write_all(&frame).is_err() {
        return Readiness::NotYet;
    }
    read_reply(&mut stream)
}

fn read_reply(stream: &mut TcpStream) -> Readiness {
    let mut decoder = FrameDecoder::new();
    let mut buf = [0u8; READ_CHUNK];
    loop {
        match decoder.next_frame() {
            Ok(Some(frame)) => {
                return match frame.decode::<McpResponse>() {
                    Ok(_) => Readiness::Ready,
                    Err(_) => Readiness::NotYet,
                };
            }
            Ok(None) => {}
            Err(_) => return Readiness::NotYet,
        }
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => return Readiness::NotYet,
            Ok(count) => decoder.push(&buf[..count]),
        }
    }
}
