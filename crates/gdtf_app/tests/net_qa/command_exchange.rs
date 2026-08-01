//! The client half the GAME command-layer cases share (GTW-942).
//!
//! [`socket_support`](super::socket_support) owns the socket and the frame pump; this owns the
//! one step above it — negotiate, then send a list of command-layer requests and collect their
//! replies. It stands apart from the cases for the usual reason: what a frame LOOKS like on the
//! way out changes with the protocol, and what a reply MEANS changes with the command, and
//! those are two different reasons to edit a file.

use std::{sync::mpsc, thread};

use gdtf_app::test_support::NET_QA_PROTOCOL_VERSION;
use gdtf_qa_protocol::{
    command::{CommandArgsJson, CommandName, RunOptions},
    envelope::{QaRequest, QaResponse, RunCommand},
};

use super::socket_support::{Client, TestError, drive_until_reported, game_app_listening};

/// The one command the game's `GAME_COMMANDS` publishes today.
pub(crate) const APP_PHASE: &str = "app.phase";

/// Drive the real app while a client negotiates and then sends `requests` in order, and hand
/// back one reply per request.
///
/// The handshake is sent first because the listener refuses everything else until it has
/// (GTW-940), and its reply is dropped: what these cases measure is what comes after it.
///
/// # Errors
///
/// Any socket or codec failure on the client half, or a frame budget spent without a reply.
pub(crate) fn exchange_all(requests: Vec<QaRequest>) -> Result<Vec<QaResponse>, TestError> {
    let (mut app, port) = game_app_listening()?;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let collected = (|| -> Result<Vec<QaResponse>, TestError> {
            let mut client = Client::connect(port)?;
            let hello = client.exchange(&QaRequest::Hello(NET_QA_PROTOCOL_VERSION))?;
            if !matches!(hello, QaResponse::HelloOk(_)) {
                return Err(format!("the handshake must succeed first, got {hello:?}").into());
            }
            let mut replies = Vec::with_capacity(requests.len());
            for request in &requests {
                replies.push(client.exchange(request)?);
            }
            Ok(replies)
        })();
        let _sent = tx.send(collected);
    });
    drive_until_reported(&mut app, &rx)
}

/// Send one request over the real path and return its reply.
///
/// # Errors
///
/// Any socket or codec failure on the client half, or a frame budget spent without a reply.
pub(crate) fn exchange(request: QaRequest) -> Result<QaResponse, TestError> {
    let mut replies = exchange_all(vec![request])?;
    replies.pop().ok_or_else(|| "no reply arrived".into())
}

/// A `Run` frame with the given name, arguments and riders.
pub(crate) fn run(name: &'static str, arguments: &str, options: RunOptions) -> QaRequest {
    QaRequest::Run(RunCommand::with_options(
        CommandName::from_static(name),
        CommandArgsJson::new(arguments.to_owned()),
        options,
    ))
}
