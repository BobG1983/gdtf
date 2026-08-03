use std::{sync::mpsc, thread};

use gdtf_app::test_support::NET_QA_PROTOCOL_VERSION;
use gdtf_qa_protocol::{
    command::{CommandArgsJson, CommandName, RunOptions},
    message::{QaRequest, QaResponse, RunCommand},
};

use super::socket_support::{Client, TestError, drive_until_reported, game_app_listening};

pub(crate) const APP_PHASE: &str = "app.phase";

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

pub(crate) fn exchange(request: QaRequest) -> Result<QaResponse, TestError> {
    let mut replies = exchange_all(vec![request])?;
    replies.pop().ok_or_else(|| "no reply arrived".into())
}

pub(crate) fn run(name: &'static str, arguments: &str, options: RunOptions) -> QaRequest {
    QaRequest::Run(RunCommand::with_options(
        CommandName::from_static(name),
        CommandArgsJson::new(arguments.to_owned()),
        options,
    ))
}
