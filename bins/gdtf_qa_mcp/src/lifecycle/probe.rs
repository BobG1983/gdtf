//! version mismatch) proves the listener is answering, i.e. [`Ready`](Readiness::Ready).
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpStream},
};

use gdtf_qa_protocol::{
    framing::{FrameDecoder, encode},
    message::{ProtocolVersion, QaRequest, QaResponse},
};

use super::values::{ProbeTimeout, Readiness};
use crate::link::QaPort;

const PROBE_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::new(1);

const READ_CHUNK: usize = 1024;

#[must_use]
pub(super) fn probe_ready(port: QaPort, timeout: ProbeTimeout) -> Readiness {
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
