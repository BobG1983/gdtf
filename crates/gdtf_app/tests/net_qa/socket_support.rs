//! The socket fixtures the REAL-listener suites share: a headless game with its real
//! `net_qa` listener bound, a real client over a real [`TcpStream`], and the frame pump that
//! drives the app while that client talks to it (GTW-940; lifted here in GTW-942, when the
//! command suite became a second consumer).
//!
//! Nothing here stands in for the code under test. The listener is the one
//! [`NetQaPlugin::listening`] binds, `build` spawns the REAL accept loop through the same
//! `serve` the env-driven arm runs, and every frame goes through the protocol crate's real
//! framing codec. What the fixtures own is only the CLIENT half.

use std::{
    error::Error,
    io::{Read as _, Write as _},
    net::{Ipv4Addr, TcpStream},
    sync::mpsc::{Receiver, RecvTimeoutError},
    time::Duration,
};

use bevy::app::App;
use gdtf_app::test_support::{AppState, NetQaPlugin};
use gdtf_net_qa_transport::NetQaPort;
use gdtf_qa_protocol::{
    envelope::{QaRequest, QaResponse},
    framing::{FrameDecoder, encode},
};
use gdtf_test_utils::GdtfLoadTestAppBuilder;

/// A boxed error so a test's `?` spans [`std::io::Error`], the codec's error type and a bare
/// message. `Send + Sync` because a client half runs on its own thread and reports back over
/// an [`mpsc`](std::sync::mpsc) channel.
pub(crate) type TestError = Box<dyn Error + Send + Sync>;

/// The usual test return: nothing, or a boxed error.
pub(crate) type TestResult = Result<(), TestError>;

/// How long the client waits on its own read before reporting a failure instead of hanging.
const READ_TIMEOUT: Duration = Duration::from_secs(10);

/// How long one frame iteration waits on the client thread before driving another update.
const POLL_STEP: Duration = Duration::from_millis(10);

/// A SAFETY NET on the frame loop, not a timing budget: the loop exits as soon as the client
/// thread has reported.
const MAX_UPDATES: u32 = 400;

/// A connected client socket with its own decoder.
///
/// ONE decoder across the whole connection: a reply may arrive in the same read as the tail
/// of the previous one, and the decoder owns that leftover.
pub(crate) struct Client {
    /// The real socket to the game's real listener.
    stream:  TcpStream,
    /// The real framing decoder the replies are read through.
    decoder: FrameDecoder,
}

impl Client {
    /// Connect to the game's real listener on loopback.
    ///
    /// # Errors
    ///
    /// Any [`std::io::Error`] from the connect or from setting the read deadline.
    pub(crate) fn connect(port: NetQaPort) -> Result<Self, TestError> {
        let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, *port))?;
        stream.set_read_timeout(Some(READ_TIMEOUT))?;
        Ok(Self {
            stream,
            decoder: FrameDecoder::new(),
        })
    }

    /// Read one whole framed [`QaResponse`] off the socket (tolerating a split read).
    ///
    /// # Errors
    ///
    /// A codec failure, any read error, or the game closing before a whole reply arrived.
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

    /// Send one request and read its reply.
    ///
    /// # Errors
    ///
    /// A codec failure, any socket error, or a truncated reply.
    pub(crate) fn exchange(&mut self, request: &QaRequest) -> Result<QaResponse, TestError> {
        self.stream.write_all(&encode(request)?)?;
        self.read()
    }
}

/// Build a headless app resting in [`AppState::Running`] with the game's REAL `net_qa`
/// listener already bound, returning the app and the port the OS assigned.
///
/// Port `0` so parallel test binaries never collide, and bound BEFORE the app is built so the
/// client knows where to connect without racing `build`.
///
/// The `DefaultPlugins` tier (no GPU, no winit) rather than the `MinimalPlugins` one the
/// injected-inbox suite uses: the listener arm these tests exist to reach also adds the
/// GTW-764 offscreen-capture present path, whose target system takes `ResMut<Assets<Image>>` —
/// a resource `MinimalPlugins` does not have, so a driven frame there dies on parameter
/// validation. No window is spawned, so that system finds no primary window and returns
/// without creating a target; nothing else here depends on it.
///
/// # Errors
///
/// Any [`std::io::Error`] from binding the loopback listener.
pub(crate) fn game_app_listening() -> Result<(App, NetQaPort), TestError> {
    let (plugin, port) = NetQaPlugin::listening(NetQaPort::new(0))?;
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Running)
        .build();
    app.add_plugins(plugin);
    Ok((app, port))
}

/// Drive one frame per iteration until the client half reports, then hand back what it
/// reported.
///
/// # Errors
///
/// The client half's own failure, or a frame budget spent without it reporting at all.
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
