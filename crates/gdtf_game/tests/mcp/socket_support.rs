use std::{
    error::Error,
    io::{self, ErrorKind, Read as _, Write as _},
    net::{Ipv4Addr, TcpStream},
};

use bevy::{
    app::App,
    state::state::{NextState, State},
    time::TimeUpdateStrategy,
};
use cobalt_mcp_protocol::{
    framing::{FrameDecoder, encode},
    message::{QaRequest, QaResponse},
    ports::McpPort,
};
use cobalt_screenshot::{PollCap, SettleFrames};
use gdtf_battle_sim::rng::BattleSeed;
use gdtf_game::test_support::{AppState, BattleScapeState, McpPlugin, RunningState};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until};

pub(crate) type TestError = Box<dyn Error + Send + Sync>;

pub(crate) type TestResult = Result<(), TestError>;

/// Builds a listening app and returns it with the port its listener bound.
pub(crate) type SocketFixture = fn() -> Result<(App, McpPort), TestError>;

/// Fixed steps a frame runs while the descent is held still.
const STOPPED: u32 = 0;

/// Fixed steps a frame runs once the descent is let go.
const ONE_STEP_A_FRAME: u32 = 1;

/// Pins procgen so every battle fixture generates the same map on every run.
const FIXTURE_SEED: BattleSeed = BattleSeed::new(20_260_805);

pub(crate) struct Client {
    stream:  TcpStream,
    decoder: FrameDecoder,
}

impl Client {
    pub(crate) fn connect(port: McpPort) -> Result<Self, TestError> {
        let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, *port))?;
        stream.set_nonblocking(true)?;
        Ok(Self {
            stream,
            decoder: FrameDecoder::new(),
        })
    }

    /// Send `request`, then run frames until the game answers it.
    /// The wait has no budget, so a busy machine makes a case slower and never red.
    pub(crate) fn exchange(
        &mut self,
        app: &mut App,
        request: &QaRequest,
    ) -> Result<QaResponse, TestError> {
        self.stream.write_all(&encode(request)?)?;
        self.answer(app)
    }

    fn answer(&mut self, app: &mut App) -> Result<QaResponse, TestError> {
        let mut buf = [0u8; 512];
        loop {
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

fn listening_menu_app() -> Result<(App, McpPort), TestError> {
    let (plugin, port) = McpPlugin::listening(McpPort::new(0))?;
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();
    app.add_plugins(plugin);
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(ONE_STEP_A_FRAME));
    advance_until(&mut app, |app| {
        running_state(app) == Some(RunningState::Menu)
    });
    Ok((app, port))
}

pub(crate) fn game_app_listening() -> Result<(App, McpPort), TestError> {
    listening_menu_app()
}

/// The menu app with both capture tunables small, so a capture that cannot land gives up fast.
pub(crate) fn capture_app_listening() -> Result<(App, McpPort), TestError> {
    let (mut app, port) = listening_menu_app()?;
    app.insert_resource(SettleFrames::new(2));
    app.insert_resource(PollCap::new(4));
    Ok((app, port))
}

// Queue the descent into a battle without advancing a single frame.
fn open_the_battle(app: &mut App) {
    app.insert_resource(FIXTURE_SEED);
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
}

/// The menu app with the descent queued and the fixed clock stopped, so it cannot advance.
///
/// Every step of the descent runs in `FixedUpdate`, so it moves only when a case calls
/// [`let_the_descent_run`] — never because the machine was slow between two frames.
pub(crate) fn opening_battle_app_listening() -> Result<(App, McpPort), TestError> {
    let (mut app, port) = listening_menu_app()?;
    open_the_battle(&mut app);
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(STOPPED));
    Ok((app, port))
}

/// Let the queued descent advance one fixed step per frame.
pub(crate) fn let_the_descent_run(app: &mut App) {
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(ONE_STEP_A_FRAME));
}

pub(crate) fn battle_app_listening() -> Result<(App, McpPort), TestError> {
    let (mut app, port) = listening_menu_app()?;
    open_the_battle(&mut app);
    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::BattleRunning)
    });
    Ok((app, port))
}
