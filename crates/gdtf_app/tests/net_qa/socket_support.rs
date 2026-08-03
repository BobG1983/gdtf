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
    framing::{FrameDecoder, encode},
    message::{QaRequest, QaResponse},
};
use gdtf_test_utils::GdtfLoadTestAppBuilder;

pub(crate) type TestError = Box<dyn Error + Send + Sync>;

pub(crate) type TestResult = Result<(), TestError>;

const READ_TIMEOUT: Duration = Duration::from_secs(10);

const POLL_STEP: Duration = Duration::from_millis(10);

const MAX_UPDATES: u32 = 400;

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

pub(crate) fn game_app_listening() -> Result<(App, NetQaPort), TestError> {
    let (plugin, port) = NetQaPlugin::listening(NetQaPort::new(0))?;
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Running)
        .build();
    app.add_plugins(plugin);
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
