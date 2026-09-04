use core::ops::ControlFlow;
use std::{
    io::{self, Read, Write},
    net::TcpStream,
    sync::mpsc::Sender,
};

use cobalt_mcp_protocol::{
    framing::{FrameDecoder, encode},
    message::{HelloFacts, McpRequest, McpResponse, McpSessionError},
    timeouts::{NetReplyTimeout, NetTimeouts},
};

use super::session::{FrameVerdict, SessionState};
use crate::transport::channel::{IncomingRequest, Responder};

pub(super) fn handle_client(
    mut stream: TcpStream,
    request_tx: &Sender<IncomingRequest>,
    timeouts: NetTimeouts,
    facts: &HelloFacts,
) {
    drop(stream.set_read_timeout(Some(*timeouts.io())));
    drop(stream.set_write_timeout(Some(*timeouts.io())));
    let mut decoder = FrameDecoder::new();
    let mut session = SessionState::Fresh;
    let mut buf = [0u8; 4096];
    loop {
        let read = match stream.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(n) => n,
        };
        decoder.push(&buf[..read]);
        loop {
            let frame = match decoder.next_frame() {
                Ok(Some(frame)) => frame,
                Ok(None) => break,
                Err(_) => return,
            };
            if handle_frame(
                &mut stream,
                &frame,
                request_tx,
                timeouts.reply(),
                facts,
                &mut session,
            )
            .is_break()
            {
                return;
            }
        }
    }
}

fn handle_frame(
    stream: &mut TcpStream,
    frame: &cobalt_mcp_protocol::framing::Frame,
    request_tx: &Sender<IncomingRequest>,
    reply_timeout: NetReplyTimeout,
    facts: &HelloFacts,
    session: &mut SessionState,
) -> ControlFlow<()> {
    let Ok(request) = frame.decode::<McpRequest>() else {
        drop(write_frame(
            stream,
            &McpResponse::Error(McpSessionError::Malformed),
        ));
        return ControlFlow::Continue(());
    };
    match session.admit(&request, facts) {
        FrameVerdict::Answer(response) => match write_frame(stream, &response) {
            Ok(()) => ControlFlow::Continue(()),
            Err(_) => ControlFlow::Break(()),
        },
        FrameVerdict::Forward => forward_to_host(stream, request, request_tx, reply_timeout),
    }
}

fn forward_to_host(
    stream: &mut TcpStream,
    request: McpRequest,
    request_tx: &Sender<IncomingRequest>,
    reply_timeout: NetReplyTimeout,
) -> ControlFlow<()> {
    let (responder, reply_rx) = Responder::channel();
    if request_tx
        .send(IncomingRequest::new(request, responder))
        .is_err()
    {
        return ControlFlow::Break(());
    }
    let response = reply_rx
        .recv_timeout(*reply_timeout)
        .unwrap_or(McpResponse::Error(McpSessionError::Timeout));
    match write_frame(stream, &response) {
        Ok(()) => ControlFlow::Continue(()),
        Err(_) => ControlFlow::Break(()),
    }
}

fn write_frame(stream: &mut TcpStream, response: &McpResponse) -> io::Result<()> {
    let frame = encode(response).map_err(io::Error::other)?;
    stream.write_all(&frame)
}
