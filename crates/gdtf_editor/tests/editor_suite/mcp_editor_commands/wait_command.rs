use core::time::Duration;

use cobalt_mcp_host::dispatch::DeferredBudget;
use cobalt_mcp_protocol::message::{McpRequest, McpResponse, McpSessionError, ProtocolVersion};
use gdtf_assets::ContentChecksComplete;
use gdtf_battle_sim::terrain::def::TerrainDefRegistry;
use gdtf_editor::{EditorState, shorten_editor_wait_budget};

use crate::{
    mcp_editor_commands::{
        names::{EDITOR_SET_MODE, WAIT},
        rows::{ContentFamilyRow, PhaseRow, SetModeReplyRow, WaitConditionRow, WaitReplyRow},
    },
    mcp_shared::{
        harness::{advance_to_editing, editing_app_and_client, editor_app_listening, editor_state},
        hello::assert_hello_ok,
        mirror::ModeRow,
        outcome::ran_body,
        socket::{Client, run_editor},
        support::TestResult,
    },
};

/// The condition both halves of the parking case name.
const CHECKS_COMPLETE: &str = "(condition: ChecksComplete)";

/// A family whose registry the editor's own Load pass fills, so its change does come.
const TERRAIN_REARMED: &str = "(condition: RegistryRearmed(family: Terrain))";

/// A family whose registry nothing writes once the editor is already up.
const NEVER_REARMED: &str = "(condition: RegistryRearmed(family: Sprite))";

#[test]
fn a_wait_for_the_content_checks_is_held_through_the_load_pass() -> TestResult {
    let (mut app, port) = editor_app_listening()?;
    shorten_editor_wait_budget(&mut app, DeferredBudget::new(Duration::MAX));
    let mut client = Client::connect(port)?;
    client.send(&McpRequest::Hello(ProtocolVersion::CURRENT))?;
    client.send(&run_editor(WAIT, CHECKS_COMPLETE))?;
    assert_eq!(
        editor_state(&app),
        Some(EditorState::Load),
        "the app has run no frames yet, so the wait goes out during the editor's Load pass",
    );
    assert!(
        app.world()
            .get_resource::<ContentChecksComplete>()
            .is_none(),
        "the reference checks need the registries, which no frame has loaded yet, so the \
         condition the wait names is false when it is admitted",
    );

    let negotiated = client.read(&mut app)?;
    assert_hello_ok(&negotiated);
    let reply = client.read(&mut app)?;

    let body: WaitReplyRow = ran_body(&reply, WAIT)?;
    assert_eq!(
        body.condition,
        WaitConditionRow::ChecksComplete,
        "the reply names the condition that came true, so a client can tell which of its waits \
         was released: {body:?}",
    );
    assert!(
        app.world()
            .get_resource::<ContentChecksComplete>()
            .is_some(),
        "the reply is held until the condition holds, so the marker the condition reads must be \
         in the world by the frame that answered — a wait that answered on admission would find \
         it absent",
    );

    advance_to_editing(&mut app);
    let settled = client.exchange(&mut app, &run_editor(WAIT, CHECKS_COMPLETE))?;
    let settled: WaitReplyRow = ran_body(&settled, WAIT)?;
    assert_eq!(
        settled.phase,
        PhaseRow::Editing,
        "a condition that already holds is answered on the frame it is claimed in, and the \
         reply reports the phase that frame was in: {settled:?}",
    );
    Ok(())
}

#[test]
fn a_wait_on_a_registry_is_released_by_the_frame_that_loads_it() -> TestResult {
    let (mut app, port) = editor_app_listening()?;
    shorten_editor_wait_budget(&mut app, DeferredBudget::new(Duration::MAX));
    let mut client = Client::connect(port)?;
    client.send(&McpRequest::Hello(ProtocolVersion::CURRENT))?;
    client.send(&run_editor(WAIT, TERRAIN_REARMED))?;
    assert!(
        app.world().get_resource::<TerrainDefRegistry>().is_none(),
        "the app has run no frames yet, so the registry this wait watches is not in the world \
         and nothing has changed it",
    );

    let negotiated = client.read(&mut app)?;
    assert_hello_ok(&negotiated);
    let reply = client.read(&mut app)?;

    let body: WaitReplyRow = ran_body(&reply, WAIT)?;
    assert_eq!(
        body.condition,
        WaitConditionRow::RegistryRearmed {
            family: ContentFamilyRow::Terrain,
        },
        "the reply names the family whose registry moved, so a client holding several waits \
         can tell which one was released: {body:?}",
    );
    assert!(
        app.world().get_resource::<TerrainDefRegistry>().is_some(),
        "the wait is released by the frame that counted the change, so the registry it named \
         is in the world by then — a handler that never counted would still be parked",
    );
    Ok(())
}

#[test]
fn a_wait_on_a_registry_that_never_changes_expires_and_leaves_the_socket_open() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    shorten_editor_wait_budget(&mut app, DeferredBudget::new(Duration::ZERO));

    let expired = client.exchange(&mut app, &run_editor(WAIT, NEVER_REARMED))?;

    assert_eq!(
        expired,
        McpResponse::Error(McpSessionError::Timeout),
        "a condition nothing will make true must expire against the parking budget rather than \
         be refused or answered unsatisfied — there is no third reply shape",
    );

    let reopened = client.exchange(&mut app, &run_editor(EDITOR_SET_MODE, "(mode: Sprite)"))?;
    let opened: SetModeReplyRow = ran_body(&reopened, EDITOR_SET_MODE)?;
    assert_eq!(
        opened.mode,
        ModeRow::Sprite,
        "the expiry answers on the connection rather than dropping it, so the next command sent \
         down the same socket is still served",
    );
    Ok(())
}
