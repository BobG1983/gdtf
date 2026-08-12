use std::{
    error::Error,
    io::{self, ErrorKind, Read as _, Write as _},
    net::{Ipv4Addr, TcpStream},
    time::Duration,
};

use bevy::{
    app::App,
    state::state::{NextState, State},
};
use gdtf_app::test_support::{AppState, BattleScapeState, NetQaPlugin, RunningState};
use gdtf_battle_sim::rng::BattleSeed;
use gdtf_qa_protocol::{
    framing::{FrameDecoder, encode},
    message::{QaRequest, QaResponse},
    ports::NetQaPort,
};
use gdtf_screenshot::{PollCap, SettleFrames};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until};

pub(crate) type TestError = Box<dyn Error + Send + Sync>;

pub(crate) type TestResult = Result<(), TestError>;

/// Builds a listening app and returns it with the port its listener bound.
pub(crate) type SocketFixture = fn() -> Result<(App, NetQaPort), TestError>;

/// How long one look at the socket waits before the game gets another frame.
/// Its expiry is the pause between frames, never a failure.
const READ_POLL: Duration = Duration::from_millis(10);

/// Frames the game may take to answer one request. A deferred act waits out a whole enemy turn.
const REPLY_BUDGET: u32 = 1024;

const DRIVE_BUDGET: u32 = 512;

/// Pins procgen so every battle fixture generates the same map on every run.
const FIXTURE_SEED: BattleSeed = BattleSeed::new(20_260_805);

pub(crate) struct Client {
    stream:  TcpStream,
    decoder: FrameDecoder,
}

impl Client {
    pub(crate) fn connect(port: NetQaPort) -> Result<Self, TestError> {
        let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, *port))?;
        stream.set_read_timeout(Some(READ_POLL))?;
        Ok(Self {
            stream,
            decoder: FrameDecoder::new(),
        })
    }

    /// Send `request`, then run frames until the game answers it.
    /// The wait is counted in frames, so a busy machine makes a case slower and never red.
    pub(crate) fn exchange(
        &mut self,
        app: &mut App,
        request: &QaRequest,
    ) -> Result<QaResponse, TestError> {
        self.stream.write_all(&encode(request)?)?;
        self.answer(app, request)
    }

    fn answer(&mut self, app: &mut App, request: &QaRequest) -> Result<QaResponse, TestError> {
        let mut buf = [0u8; 512];
        for _ in 0..REPLY_BUDGET {
            app.update();
            match self.stream.read(&mut buf) {
                Ok(0) => {
                    return Err(
                        "the game closed the connection before a full response arrived".into(),
                    );
                }
                Ok(read) => self.decoder.push(&buf[..read]),
                Err(quiet) if nothing_yet(&quiet) => {}
                Err(failed) => return Err(failed.into()),
            }
            if let Some(frame) = self.decoder.next_frame()? {
                return Ok(frame.decode::<QaResponse>()?);
            }
        }
        Err(format!("the game never answered {request:?} within {REPLY_BUDGET} frames").into())
    }
}

// The socket had nothing to hand over this poll, rather than a connection that broke.
fn nothing_yet(fault: &io::Error) -> bool {
    matches!(fault.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)
}

fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

fn listening_menu_app() -> Result<(App, NetQaPort), TestError> {
    let (plugin, port) = NetQaPlugin::listening(NetQaPort::new(0))?;
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();
    app.add_plugins(plugin);
    if !advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        DRIVE_BUDGET,
    ) {
        return Err(format!(
            "the real Load state never rested at RunningState::Menu within {DRIVE_BUDGET} \
             frames; the live state was {:?}",
            running_state(&app)
        )
        .into());
    }
    Ok((app, port))
}

pub(crate) fn game_app_listening() -> Result<(App, NetQaPort), TestError> {
    listening_menu_app()
}

/// The menu app with both capture tunables small enough for a reply inside `REPLY_BUDGET`.
pub(crate) fn capture_app_listening() -> Result<(App, NetQaPort), TestError> {
    let (mut app, port) = listening_menu_app()?;
    app.insert_resource(SettleFrames::new(2));
    app.insert_resource(PollCap::new(4));
    Ok((app, port))
}

pub(crate) fn battle_app_listening() -> Result<(App, NetQaPort), TestError> {
    let (mut app, port) = listening_menu_app()?;
    app.insert_resource(FIXTURE_SEED);
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    if !advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        DRIVE_BUDGET,
    ) {
        return Err(format!(
            "the descent never reached BattleScapeState::BattleRunning within {DRIVE_BUDGET} \
             frames; the live state was {:?}",
            battlescape_state(&app)
        )
        .into());
    }
    Ok((app, port))
}
