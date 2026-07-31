//! GTW-940: the GAME's handshake, over its REAL loopback listener and a REAL socket.
//!
//! The routing suite next door drives the router through an injected inbox, so it can say
//! nothing about the listener the game actually spawns. This file closes that gap the way the
//! editor's `tests/net_qa_hello/` does: [`NetQaPlugin::listening`] binds the REAL listener on
//! an OS-assigned port, `build` spawns the REAL accept loop through the same `serve` the
//! env-driven arm runs, and the client is a REAL [`TcpStream`] speaking the REAL framing
//! codec.
//!
//! What that pins is the half no in-process test could: the facts the GAME hands
//! [`run_listener`](gdtf_net_qa_transport::run_listener). Change that call site to the
//! editor's server name, or to a neighbouring protocol version, and
//! [`hello_over_the_real_listener_answers_the_games_own_facts`] fails — where before it was a
//! line no test reached at all.

use std::{
    error::Error,
    io::{Read as _, Write as _},
    net::{Ipv4Addr, TcpStream},
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread,
    time::Duration,
};

use bevy::app::App;
use gdtf_app::test_support::{AppState, NET_QA_PROTOCOL_VERSION, NET_QA_SERVER_NAME, NetQaPlugin};
use gdtf_net_qa_transport::NetQaPort;
use gdtf_qa_protocol::{
    envelope::{QaRequest, QaResponse},
    framing::{FrameDecoder, encode},
    view::AppStateNet,
};
use gdtf_test_utils::GdtfLoadTestAppBuilder;

/// A boxed error so a test's `?` spans [`std::io::Error`], the codec's error type and a bare
/// message. `Send + Sync` because the client half of the second case crosses a thread
/// boundary back to the test body over an [`mpsc`] channel.
type TestError = Box<dyn Error + Send + Sync>;

/// The usual test return: nothing, or a boxed error.
type TestResult = Result<(), TestError>;

/// How long the client waits on its own read before reporting a failure instead of hanging.
const READ_TIMEOUT: Duration = Duration::from_secs(10);

/// How long one frame iteration waits on the client thread before driving another update.
const POLL_STEP: Duration = Duration::from_millis(10);

/// A SAFETY NET on the frame loop, not a timing budget: the loop exits as soon as the client
/// thread has reported.
const MAX_UPDATES: u32 = 400;

/// A connected client socket with its own decoder.
///
/// ONE decoder across the whole connection: a reply may arrive in the same read as the tail of
/// the previous one, and the decoder owns that leftover.
struct Client {
    /// The real socket to the game's real listener.
    stream:  TcpStream,
    /// The real framing decoder the replies are read through.
    decoder: FrameDecoder,
}

impl Client {
    /// Connect to the game's real listener on loopback.
    fn connect(port: NetQaPort) -> Result<Self, TestError> {
        let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, *port))?;
        stream.set_read_timeout(Some(READ_TIMEOUT))?;
        Ok(Self {
            stream,
            decoder: FrameDecoder::new(),
        })
    }

    /// Read one whole framed [`QaResponse`] off the socket (tolerating a split read).
    fn read(&mut self) -> Result<QaResponse, TestError> {
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
    fn exchange(&mut self, request: &QaRequest) -> Result<QaResponse, TestError> {
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
/// injected-inbox suite uses: the listener arm this test exists to reach also adds the GTW-764
/// offscreen-capture present path, whose target system takes `ResMut<Assets<Image>>` — a
/// resource `MinimalPlugins` does not have, so a
/// driven frame there dies on parameter validation. No window is spawned, so that system
/// finds no primary window and returns without creating a target; nothing else here depends
/// on it.
fn game_app_listening() -> Result<(App, NetQaPort), TestError> {
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
fn drive_until_reported<T>(
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

/// The handshake the GAME answers over its own listener carries the GAME's facts — the
/// version it declares it speaks AND its own server name.
///
/// No frame is driven, deliberately: since GTW-940 the listener thread answers `Hello`
/// itself, so a reply that needed the app's drain would be a regression this case would hang
/// on rather than pass.
///
/// The one test that fails if the facts handed to `run_listener` in
/// `crate::dev::net_qa::plugin` are swapped for the editor's name or a neighbouring version.
#[test]
fn hello_over_the_real_listener_answers_the_games_own_facts() -> TestResult {
    let (_app, port) = game_app_listening()?;
    let mut client = Client::connect(port)?;
    let reply = client.exchange(&QaRequest::Hello(NET_QA_PROTOCOL_VERSION))?;
    assert!(
        matches!(
            &reply,
            QaResponse::HelloOk(facts)
                if facts.protocol == NET_QA_PROTOCOL_VERSION
                    && *facts.server == NET_QA_SERVER_NAME
        ),
        "the game's listener must answer HelloOk with the game's OWN facts (version \
         {NET_QA_PROTOCOL_VERSION:?}, server {NET_QA_SERVER_NAME}), so a client can tell which \
         host it reached — got {reply:?}",
    );
    Ok(())
}

/// Once negotiated, a request crosses the same listener into the game's REAL router and comes
/// back framed — the inbox half of the wiring, over the same socket.
///
/// Both replies are collected on the client thread while the test body drives frames: the
/// handshake costs no frame (the listener thread answers it), but the `GetAppFlow` reply only
/// exists once the router's drain has run.
#[test]
fn a_negotiated_request_reaches_the_games_real_router() -> TestResult {
    let (mut app, port) = game_app_listening()?;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let collected = (|| -> Result<[QaResponse; 2], TestError> {
            let mut client = Client::connect(port)?;
            Ok([
                client.exchange(&QaRequest::Hello(NET_QA_PROTOCOL_VERSION))?,
                client.exchange(&QaRequest::GetAppFlow)?,
            ])
        })();
        let _sent = tx.send(collected);
    });

    let [hello, app_flow] = drive_until_reported(&mut app, &rx)?;
    assert!(
        matches!(&hello, QaResponse::HelloOk(_)),
        "the connection must negotiate before the measured request goes out, got {hello:?}",
    );
    assert!(
        matches!(
            &app_flow,
            QaResponse::AppFlow(view)
                if view.state == AppStateNet::Running && !*view.battle_active
        ),
        "a negotiated GetAppFlow must be answered by the game's real router with \
         AppFlow(Running, battle_active=false), got {app_flow:?}",
    );
    Ok(())
}
