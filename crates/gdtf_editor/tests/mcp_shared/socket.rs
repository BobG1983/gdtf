use std::{
    io::{self, ErrorKind, Read as _, Write as _},
    net::{Ipv4Addr, TcpStream},
};

use bevy::app::App;
use cobalt_mcp_protocol::{
    command::{CommandArgsRon, CommandName, RunOptions},
    framing::{FrameDecoder, encode},
    message::{McpRequest, McpResponse, RunCommand},
    ports::McpPort,
};

use crate::support::TestError;

/// Frames an unbudgeted read runs between one look at the socket and the next.
const READ_BATCH_FRAMES: u32 = 16;

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

    /// Put a request on the wire without running a frame, so a case can read the state it left.
    pub(crate) fn send(&mut self, request: &McpRequest) -> Result<(), TestError> {
        self.stream.write_all(&encode(request)?)?;
        Ok(())
    }

    /// Read what is already answered before running another frame, so a case reads back the world
    /// the answering frame left. The wait has no budget, so a busy machine is never red.
    pub(crate) fn read(&mut self, app: &mut App) -> Result<McpResponse, TestError> {
        loop {
            if let Some(response) = self.read_within(app, READ_BATCH_FRAMES)? {
                return Ok(response);
            }
        }
    }

    /// Read the answer within `frames` frames, or `None` once that many have run without one.
    ///
    /// The budget is a frame count, not a clock, so a slow machine cannot make a case flaky.
    pub(crate) fn read_within(
        &mut self,
        app: &mut App,
        frames: u32,
    ) -> Result<Option<McpResponse>, TestError> {
        for _ in 0..frames {
            if let Some(response) = self.poll_frame()? {
                return Ok(Some(response));
            }
            app.update();
        }
        self.poll_frame()
    }

    // One non-blocking look at the socket, decoding a response once a whole frame has arrived.
    fn poll_frame(&mut self) -> Result<Option<McpResponse>, TestError> {
        let mut buf = [0u8; 512];
        match self.stream.read(&mut buf) {
            Ok(0) => return Err("the editor closed before a full response arrived".into()),
            Ok(read) => self.decoder.push(&buf[..read]),
            Err(quiet) if nothing_yet(&quiet) => {}
            Err(failed) => return Err(failed.into()),
        }
        match self.decoder.next_frame()? {
            Some(frame) => Ok(Some(frame.decode::<McpResponse>()?)),
            None => Ok(None),
        }
    }

    pub(crate) fn exchange(
        &mut self,
        app: &mut App,
        request: &McpRequest,
    ) -> Result<McpResponse, TestError> {
        self.send(request)?;
        self.read(app)
    }
}

// The socket had nothing to hand over this poll, rather than a connection that broke.
fn nothing_yet(fault: &io::Error) -> bool {
    matches!(fault.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)
}

pub(crate) fn run_editor(command: &'static str, arguments: &str) -> McpRequest {
    run_editor_with(command, arguments, RunOptions::default())
}

/// A run request carrying explicit options, so a case can hang a rider on the call.
pub(crate) fn run_editor_with(
    command: &'static str,
    arguments: &str,
    options: RunOptions,
) -> McpRequest {
    McpRequest::Run(RunCommand::with_options(
        CommandName::from_static(command),
        CommandArgsRon::new(arguments.to_owned()),
        options,
    ))
}
