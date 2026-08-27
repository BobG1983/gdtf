use bevy::app::App;
use gdtf_qa_protocol::{
    command::CommandOutcome,
    message::{ProtocolVersion, QaError, QaRequest, QaResponse},
};
use gdtf_screenshot::{PollCap, SettleFrames};

use crate::{
    harness::{advance_to_editing, editor_app_listening},
    hello::assert_hello_ok,
    names::CAPTURE_SCREENSHOT,
    socket::{Client, run_editor},
    support::{TestError, TestResult},
};

/// Frames the pump settles for before it spawns the readback.
const SHORT_SETTLE: u32 = 2;

/// Frames the pump polls for the PNG before it gives up.
const SHORT_POLL: u32 = 4;

// An editing app with both capture tunables small, so a shot that cannot land gives up fast.
// Nothing renders in this harness, so nothing ever writes a PNG.
fn capture_app_and_client() -> Result<(App, Client), TestError> {
    let (mut app, port) = editor_app_listening()?;
    app.insert_resource(SettleFrames::new(SHORT_SETTLE));
    app.insert_resource(PollCap::new(SHORT_POLL));
    advance_to_editing(&mut app);
    let mut client = Client::connect(port)?;
    let hello = client.exchange(&mut app, &QaRequest::Hello(ProtocolVersion::CURRENT))?;
    assert_hello_ok(&hello);
    Ok((app, client))
}

#[test]
fn a_capture_the_headless_editor_cannot_land_reaches_the_pump_and_times_out() -> TestResult {
    let (mut app, mut client) = capture_app_and_client()?;

    let reply = client.exchange(&mut app, &run_editor(CAPTURE_SCREENSHOT, "()"))?;

    assert!(
        matches!(reply, QaResponse::Error(QaError::Timeout)),
        "with nothing writing a PNG the editor's capture must reach the pump and give up there, \
         answering Timeout rather than refusing; got {reply:?}",
    );
    assert!(
        !matches!(reply, QaResponse::Outcome(CommandOutcome::Unknown { .. })),
        "capture.screenshot must be a REGISTERED name on the editor host, not Unknown: {reply:?}",
    );
    assert!(
        !matches!(
            reply,
            QaResponse::Outcome(CommandOutcome::BadArguments { .. })
        ),
        "`()` must deserialize into the command's args, whose only field is optional: {reply:?}",
    );
    assert!(
        !matches!(
            reply,
            QaResponse::Outcome(CommandOutcome::Unavailable { .. })
        ),
        "the command needs neither the authoring scene nor a tab, so nothing may refuse it: \
         {reply:?}",
    );
    Ok(())
}
