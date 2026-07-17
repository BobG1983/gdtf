//! The readiness probe — one loopback `Hello` round-trip against the game's port
//! (GTW-745).
//!
//! `probe_ready` is how the launch loop learns the game is up: it opens a fresh loopback
//! connection, sends a [`Hello`](QaRequest::Hello), and waits for one decodable reply. A
//! refused connection, a timeout, or garbage back all read as [`NotYet`](Readiness::NotYet)
//! — only a well-formed [`QaResponse`] (whether the handshake succeeded or reported a
//! version mismatch) proves the listener is answering, i.e. [`Ready`](Readiness::Ready).
//! It reuses the protocol crate's framing rather than reimplementing it.

use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpStream},
};

use gdtf_qa_protocol::{
    envelope::{ProtocolVersion, QaRequest, QaResponse},
    framing::{FrameDecoder, encode},
};

use super::values::{ProbeTimeout, Readiness};
use crate::game::GamePort;

/// The protocol version the probe's `Hello` carries. Readiness does not depend on a match
/// — a version-mismatch reply still proves the listener is up — so any valid version does.
const PROBE_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::new(1);

/// The size of one read chunk while waiting for the reply frame.
const READ_CHUNK: usize = 1024;

/// Probe whether the game's `net_qa` listener on `port` is answering yet.
///
/// Opens a fresh loopback connection (with `timeout` bounding connect, read, and write),
/// sends a `Hello`, and returns [`Ready`](Readiness::Ready) once one decodable
/// [`QaResponse`] comes back; any failure along the way is [`NotYet`](Readiness::NotYet),
/// which the launch loop retries until its own boot timeout.
#[must_use]
pub(super) fn probe_ready(port: GamePort, timeout: ProbeTimeout) -> Readiness {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, *port));
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, *timeout) else {
        return Readiness::NotYet;
    };
    if stream.set_read_timeout(Some(*timeout)).is_err()
        || stream.set_write_timeout(Some(*timeout)).is_err()
    {
        return Readiness::NotYet;
    }
    let Ok(frame) = encode(&QaRequest::Hello(PROBE_PROTOCOL_VERSION)) else {
        return Readiness::NotYet;
    };
    if stream.write_all(&frame).is_err() {
        return Readiness::NotYet;
    }
    read_reply(&mut stream)
}

/// Read the connection until one whole frame decodes to a [`QaResponse`].
fn read_reply(stream: &mut TcpStream) -> Readiness {
    let mut decoder = FrameDecoder::new();
    let mut buf = [0u8; READ_CHUNK];
    loop {
        match decoder.next_frame() {
            Ok(Some(frame)) => {
                return match frame.decode::<QaResponse>() {
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
