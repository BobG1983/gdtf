use bevy::app::App;
use gdtf_app::test_support::NET_QA_PROTOCOL_VERSION;
use gdtf_qa_protocol::{
    command::{CommandArgsRon, CommandName, CommandOutcome, RunOptions, UnavailableCode},
    message::{QaError, QaRequest, QaResponse, RunCommand},
    ports::NetQaPort,
};

use super::socket_support::{Client, SocketFixture, TestError, battle_app_listening};

pub(crate) const APP_PHASE: &str = "app.phase";

pub(crate) const CAPTURE_SCREENSHOT: &str = "capture.screenshot";

pub(crate) const SETTINGS_READ: &str = "settings.read";

pub(crate) const UI_FOCUS: &str = "ui.focus";

pub(crate) const PLAYBACK_STATE: &str = "playback.state";

pub(crate) const BATTLE_ROSTER: &str = "battle.roster";

pub(crate) const BATTLE_TURN: &str = "battle.turn";

pub(crate) const BATTLE_SELECTION: &str = "battle.selection";

pub(crate) const BATTLE_OFFERS: &str = "battle.offers";

pub(crate) const BATTLE_INSPECT: &str = "battle.inspect";

pub(crate) const BATTLE_SIGHTLINE: &str = "battle.sightline";

pub(crate) const BATTLE_VISIBLE: &str = "battle.visible";

pub(crate) const BATTLE_COST: &str = "battle.cost";

pub(crate) const LOG_READ: &str = "log.read";

pub(crate) const BATTLE_START: &str = "battle.start";

pub(crate) const BATTLE_FLEE: &str = "battle.flee";

pub(crate) const PROCGEN_STEP: &str = "procgen.step";

pub(crate) const WAIT: &str = "wait";

pub(crate) const ACT_SELECT: &str = "act.select";

pub(crate) const ACT_SELECT_NEXT: &str = "act.select_next";

pub(crate) const ACT_SELECT_PREV: &str = "act.select_prev";

pub(crate) const ACT_SELECT_CLEAR: &str = "act.select_clear";

pub(crate) const ACT_MOVE: &str = "act.move";

pub(crate) const ACT_FIRE: &str = "act.fire";

pub(crate) const ACT_RELOAD: &str = "act.reload";

pub(crate) const ACT_SET_STANCE: &str = "act.set_stance";

pub(crate) const ACT_SET_AIMING: &str = "act.set_aiming";

pub(crate) const ACT_SET_FACING: &str = "act.set_facing";

pub(crate) const ACT_END_TURN: &str = "act.end_turn";

pub(crate) const ACT_MELEE: &str = "act.melee";

pub(crate) const ACT_SHOVE: &str = "act.shove";

pub(crate) const ACT_STABILIZE: &str = "act.stabilize";

pub(crate) const ACT_EXECUTE: &str = "act.execute";

pub(crate) const ACT_THROW_GRENADE: &str = "act.throw_grenade";

pub(crate) const ACT_OPEN_DOOR: &str = "act.open_door";

pub(crate) const ACT_ENTER_EMPLACEMENT: &str = "act.enter_emplacement";

pub(crate) const ACT_EXIT_EMPLACEMENT: &str = "act.exit_emplacement";

pub(crate) const INPUT_PRESS_KEY: &str = "input.press_key";

pub(crate) const INPUT_HOVER: &str = "input.hover";

pub(crate) const INPUT_SET_FOCUS: &str = "input.set_focus";

pub(crate) const INPUT_FOCUS_STEP: &str = "input.focus_step";

pub(crate) const INPUT_ACTIVATE: &str = "input.activate";

pub(crate) const INPUT_CLICK_CELL: &str = "input.click_cell";

pub(crate) const VIEW_LEVEL_UP: &str = "view.level_up";

pub(crate) const VIEW_LEVEL_DOWN: &str = "view.level_down";

pub(crate) const VIEW_TOGGLE_FULL_VIEW: &str = "view.toggle_full_view";

pub(crate) const VIEW_PAN: &str = "view.pan";

pub(crate) const VIEW_LOOK_AT: &str = "view.look_at";

pub(crate) const BATTLE_SET_FIRE_MODE: &str = "battle.set_fire_mode";

/// Every command the game publishes, in declaration order.
pub(crate) fn published_names() -> Vec<CommandName> {
    [
        APP_PHASE,
        CAPTURE_SCREENSHOT,
        SETTINGS_READ,
        UI_FOCUS,
        PLAYBACK_STATE,
        BATTLE_ROSTER,
        BATTLE_TURN,
        BATTLE_SELECTION,
        BATTLE_OFFERS,
        BATTLE_INSPECT,
        BATTLE_SIGHTLINE,
        BATTLE_VISIBLE,
        BATTLE_COST,
        LOG_READ,
        BATTLE_START,
        BATTLE_FLEE,
        PROCGEN_STEP,
        WAIT,
        ACT_SELECT,
        ACT_SELECT_NEXT,
        ACT_SELECT_PREV,
        ACT_SELECT_CLEAR,
        ACT_MOVE,
        ACT_FIRE,
        ACT_RELOAD,
        ACT_SET_STANCE,
        ACT_SET_AIMING,
        ACT_SET_FACING,
        ACT_END_TURN,
        ACT_MELEE,
        ACT_SHOVE,
        ACT_STABILIZE,
        ACT_EXECUTE,
        ACT_THROW_GRENADE,
        ACT_OPEN_DOOR,
        ACT_ENTER_EMPLACEMENT,
        ACT_EXIT_EMPLACEMENT,
        INPUT_PRESS_KEY,
        INPUT_HOVER,
        INPUT_SET_FOCUS,
        INPUT_FOCUS_STEP,
        INPUT_ACTIVATE,
        INPUT_CLICK_CELL,
        VIEW_LEVEL_UP,
        VIEW_LEVEL_DOWN,
        VIEW_TOGGLE_FULL_VIEW,
        VIEW_PAN,
        VIEW_LOOK_AT,
        BATTLE_SET_FIRE_MODE,
    ]
    .into_iter()
    .map(CommandName::from_static)
    .collect()
}

/// Build the fixture, let `plan` read the live world to shape the requests, then exchange.
pub(crate) fn exchange_planned(
    fixture: SocketFixture,
    plan: impl FnOnce(&App) -> Vec<QaRequest>,
) -> Result<Vec<QaResponse>, TestError> {
    let (mut app, port) = fixture()?;
    let requests = plan(&app);
    exchange_over(&mut app, port, requests)
}

/// Build the battle fixture, let `prepare` write the world and shape the requests, then exchange.
///
/// The app comes back so a case can read what the sim actually holds after the replies landed.
pub(crate) fn exchange_in_battle(
    prepare: impl FnOnce(&mut App) -> Vec<QaRequest>,
) -> Result<(App, Vec<QaResponse>), TestError> {
    let (mut app, port) = battle_app_listening()?;
    let requests = prepare(&mut app);
    let replies = exchange_over(&mut app, port, requests)?;
    Ok((app, replies))
}

/// Build a fixture that reports what it set up, shape the requests from it, then exchange.
pub(crate) fn exchange_expected<T>(
    fixture: impl FnOnce() -> Result<(App, NetQaPort, T), TestError>,
    plan: impl FnOnce(&T) -> Vec<QaRequest>,
) -> Result<(Vec<QaResponse>, T), TestError> {
    let (_app, replies, expected) = exchange_inspecting(fixture, plan)?;
    Ok((replies, expected))
}

/// Exchange as `exchange_expected` does, and keep the live app so a case can read what it left.
///
/// A reply says what the QA layer answered; the app says what the sim actually holds. A case
/// that has to tell one value from another reads the app.
pub(crate) fn exchange_inspecting<T>(
    fixture: impl FnOnce() -> Result<(App, NetQaPort, T), TestError>,
    plan: impl FnOnce(&T) -> Vec<QaRequest>,
) -> Result<(App, Vec<QaResponse>, T), TestError> {
    let (mut app, port, expected) = fixture()?;
    let requests = plan(&expected);
    let replies = exchange_over(&mut app, port, requests)?;
    Ok((app, replies, expected))
}

/// Exchange `first`, let `between` write the live world, then exchange `second`.
///
/// The world changes with no request in flight, so what each call sees is fixed by the case.
pub(crate) fn exchange_around(
    fixture: SocketFixture,
    first: QaRequest,
    between: impl FnOnce(&mut App),
    second: QaRequest,
) -> Result<(QaResponse, QaResponse), TestError> {
    let (mut app, port) = fixture()?;
    let mut client = greet(&mut app, port)?;
    let before = client.exchange(&mut app, &first)?;
    between(&mut app);
    let after = client.exchange(&mut app, &second)?;
    Ok((before, after))
}

fn greet(app: &mut App, port: NetQaPort) -> Result<Client, TestError> {
    let mut client = Client::connect(port)?;
    let hello = client.exchange(app, &QaRequest::Hello(NET_QA_PROTOCOL_VERSION))?;
    if !matches!(hello, QaResponse::HelloOk(_)) {
        return Err(format!("the handshake must succeed first, got {hello:?}").into());
    }
    Ok(client)
}

fn exchange_over(
    app: &mut App,
    port: NetQaPort,
    requests: Vec<QaRequest>,
) -> Result<Vec<QaResponse>, TestError> {
    let mut client = greet(app, port)?;
    let mut replies = Vec::with_capacity(requests.len());
    for request in &requests {
        replies.push(client.exchange(app, request)?);
    }
    Ok(replies)
}

pub(crate) fn exchange_all(
    fixture: SocketFixture,
    requests: Vec<QaRequest>,
) -> Result<Vec<QaResponse>, TestError> {
    exchange_planned(fixture, move |_app| requests)
}

pub(crate) fn exchange(
    fixture: SocketFixture,
    request: QaRequest,
) -> Result<QaResponse, TestError> {
    let mut replies = exchange_all(fixture, vec![request])?;
    replies.pop().ok_or_else(|| "no reply arrived".into())
}

/// Exchange `request` repeatedly until the reply is not `Timeout`.
///
/// A capture answers Timeout while its readback is still in flight; a loaded machine
/// makes that happen more often, never differently.
pub(crate) fn exchange_until_not_timeout(
    fixture: SocketFixture,
    request: QaRequest,
) -> Result<QaResponse, TestError> {
    let (mut app, port) = fixture()?;
    let mut client = greet(&mut app, port)?;
    loop {
        let reply = client.exchange(&mut app, &request)?;
        if !matches!(reply, QaResponse::Error(QaError::Timeout)) {
            return Ok(reply);
        }
    }
}

pub(crate) fn run(name: &'static str, arguments: &str, options: RunOptions) -> QaRequest {
    QaRequest::Run(RunCommand::with_options(
        CommandName::from_static(name),
        CommandArgsRon::new(arguments.to_owned()),
        options,
    ))
}

/// The RON body of a reply that ran, or a failure naming what came back instead.
pub(crate) fn ran_body(name: &str, reply: QaResponse) -> Result<String, TestError> {
    match reply {
        QaResponse::Outcome(CommandOutcome::Ran { reply, .. }) => Ok(reply.as_str().to_owned()),
        other => Err(format!("`{name}` must RUN in this fixture, got {other:?}").into()),
    }
}

/// Assert a command refuses with `WrongState` on a host that is not in a battle.
pub(crate) fn assert_refused_off_the_battle_screen(
    fixture: SocketFixture,
    name: &'static str,
    arguments: &str,
) -> Result<(), TestError> {
    let reply = exchange(fixture, run(name, arguments, RunOptions::default()))?;
    let QaResponse::Outcome(CommandOutcome::Unavailable { code, note }) = reply else {
        return Err(format!("`{name}` must refuse outside a battle, got {reply:?}").into());
    };
    if code != UnavailableCode::WrongState {
        return Err(
            format!("`{name}` must refuse with WrongState, got {code:?} — {note:?}").into(),
        );
    }
    Ok(())
}
