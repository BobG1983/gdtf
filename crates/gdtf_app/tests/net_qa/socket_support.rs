use std::{
    error::Error,
    io::{Read as _, Write as _},
    net::{Ipv4Addr, TcpStream},
    sync::mpsc::{Receiver, RecvTimeoutError},
    time::Duration,
};

use bevy::{
    app::App,
    state::state::{NextState, State},
};
use gdtf_app::test_support::{AppState, BattleScapeState, NetQaPlugin, RunningState};
use gdtf_net_qa_transport::NetQaPort;
use gdtf_qa_protocol::{
    framing::{FrameDecoder, encode},
    message::{QaRequest, QaResponse},
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until};

pub(crate) type TestError = Box<dyn Error + Send + Sync>;

pub(crate) type TestResult = Result<(), TestError>;

/// Builds a listening app and returns it with the port its listener bound.
pub(crate) type SocketFixture = fn() -> Result<(App, NetQaPort), TestError>;

const READ_TIMEOUT: Duration = Duration::from_secs(10);

const POLL_STEP: Duration = Duration::from_millis(10);

const MAX_UPDATES: u32 = 400;

const DRIVE_BUDGET: u32 = 512;

pub(crate) struct Client {
    stream:  TcpStream,
    decoder: FrameDecoder,
}

impl Client {
    pub(crate) fn connect(port: NetQaPort) -> Result<Self, TestError> {
        let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, *port))?;
        stream.set_read_timeout(Some(READ_TIMEOUT))?;
        Ok(Self {
            stream,
            decoder: FrameDecoder::new(),
        })
    }

    pub(crate) fn read(&mut self) -> Result<QaResponse, TestError> {
        let mut buf = [0u8; 512];
        loop {
            if let Some(frame) = self.decoder.next_frame()? {
                return Ok(frame.decode::<QaResponse>()?);
            }
            let read = self.stream.read(&mut buf)?;
            if read == 0 {
                return Err("the game closed the connection before a full response arrived".into());
            }
            self.decoder.push(&buf[..read]);
        }
    }

    pub(crate) fn exchange(&mut self, request: &QaRequest) -> Result<QaResponse, TestError> {
        self.stream.write_all(&encode(request)?)?;
        self.read()
    }
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

pub(crate) fn battle_app_listening() -> Result<(App, NetQaPort), TestError> {
    let (mut app, port) = listening_menu_app()?;
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

pub(crate) fn drive_until_reported<T>(
    app: &mut App,
    rx: &Receiver<Result<T, TestError>>,
) -> Result<T, TestError> {
    for _ in 0..MAX_UPDATES {
        app.update();
        match rx.recv_timeout(POLL_STEP) {
            Ok(reported) => return reported,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    Err("the client thread never reported its reply".into())
}
