use std::{
    io::{self, ErrorKind, Read as _, Write as _},
    net::{Ipv4Addr, TcpStream},
};

use bevy::app::App;
use gdtf_qa_protocol::{
    command::{CommandArgsRon, CommandName},
    framing::{FrameDecoder, encode},
    message::{QaRequest, QaResponse, RunCommand},
    ports::NetQaPort,
};

use crate::support::TestError;

pub(crate) struct Client {
    stream:  TcpStream,
    decoder: FrameDecoder,
}

impl Client {
    pub(crate) fn connect(port: NetQaPort) -> Result<Self, TestError> {
        let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, *port))?;
        stream.set_nonblocking(true)?;
        Ok(Self {
            stream,
            decoder: FrameDecoder::new(),
        })
    }

    /// Put a request on the wire without running a frame, so a case can read the state it left.
    pub(crate) fn send(&mut self, request: &QaRequest) -> Result<(), TestError> {
        self.stream.write_all(&encode(request)?)?;
        Ok(())
    }

    /// Read what is already answered before running another frame, so a case reads back the world
    /// the answering frame left. The wait has no budget, so a busy machine is never red.
    pub(crate) fn read(&mut self, app: &mut App) -> Result<QaResponse, TestError> {
        let mut buf = [0u8; 512];
        loop {
            match self.stream.read(&mut buf) {
                Ok(0) => return Err("the editor closed before a full response arrived".into()),
                Ok(read) => self.decoder.push(&buf[..read]),
                Err(quiet) if nothing_yet(&quiet) => {}
                Err(failed) => return Err(failed.into()),
            }
            if let Some(frame) = self.decoder.next_frame()? {
                return Ok(frame.decode::<QaResponse>()?);
            }
            app.update();
        }
    }

    pub(crate) fn exchange(
        &mut self,
        app: &mut App,
        request: &QaRequest,
    ) -> Result<QaResponse, TestError> {
        self.send(request)?;
        self.read(app)
    }
}

// The socket had nothing to hand over this poll, rather than a connection that broke.
fn nothing_yet(fault: &io::Error) -> bool {
    matches!(fault.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)
}

pub(crate) fn run_editor(command: &'static str, arguments: &str) -> QaRequest {
    QaRequest::Run(RunCommand::new(
        CommandName::from_static(command),
        CommandArgsRon::new(arguments.to_owned()),
    ))
}
