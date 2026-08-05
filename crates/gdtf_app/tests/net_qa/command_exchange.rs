use std::{sync::mpsc, thread};

use gdtf_app::test_support::NET_QA_PROTOCOL_VERSION;
use gdtf_qa_protocol::{
    command::{CommandArgsRon, CommandName, RunOptions},
    message::{QaRequest, QaResponse, RunCommand},
};

use super::socket_support::{Client, SocketFixture, TestError, drive_until_reported};

pub(crate) const APP_PHASE: &str = "app.phase";

pub(crate) const CAPTURE_SCREENSHOT: &str = "capture.screenshot";

pub(crate) const SETTINGS_READ: &str = "settings.read";

pub(crate) const UI_FOCUS: &str = "ui.focus";

pub(crate) const PLAYBACK_STATE: &str = "playback.state";

pub(crate) const BATTLE_START: &str = "battle.start";

pub(crate) const BATTLE_FLEE: &str = "battle.flee";

pub(crate) const PROCGEN_STEP: &str = "procgen.step";

pub(crate) const WAIT: &str = "wait";

pub(crate) fn exchange_all(
    fixture: SocketFixture,
    requests: Vec<QaRequest>,
) -> Result<Vec<QaResponse>, TestError> {
    let (mut app, port) = fixture()?;
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

pub(crate) fn exchange(
    fixture: SocketFixture,
    request: QaRequest,
) -> Result<QaResponse, TestError> {
    let mut replies = exchange_all(fixture, vec![request])?;
    replies.pop().ok_or_else(|| "no reply arrived".into())
}

pub(crate) fn run(name: &'static str, arguments: &str, options: RunOptions) -> QaRequest {
    QaRequest::Run(RunCommand::with_options(
        CommandName::from_static(name),
        CommandArgsRon::new(arguments.to_owned()),
        options,
    ))
}
