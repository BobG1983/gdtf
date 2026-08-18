use std::{
    io::{self, ErrorKind, Read as _, Write as _},
    net::{Ipv4Addr, TcpStream},
};

use bevy::app::App;
use gdtf_qa_protocol::{
    command::{CommandArgsRon, CommandName},
    framing::{FrameDecoder, encode},
    message::{ProtocolVersion, QaRequest, QaResponse, RunCommand},
    ports::NetQaPort,
};

use crate::support::{EDITING_EXCHANGES, TestError};

/// The lifecycle read the editor host publishes.
pub(crate) const EDITOR_PHASE: &str = "editor.phase";

/// The newest-save read the editor host publishes.
pub(crate) const EDITOR_LAST_SAVE: &str = "editor.last_save";

/// The mode-tab write the editor host publishes.
pub(crate) const EDITOR_SET_MODE: &str = "editor.set_mode";

/// The blank-draft write the editor host publishes.
pub(crate) const EDITOR_NEW: &str = "editor.new";

/// The registry-load write the editor host publishes.
pub(crate) const EDITOR_LOAD: &str = "editor.load";

/// The draft-save write the editor host publishes.
pub(crate) const EDITOR_SAVE: &str = "editor.save";

/// The single-field write the editor host publishes.
pub(crate) const EDITOR_SET_FIELD: &str = "editor.set_field";

/// The list-field write the editor host publishes.
pub(crate) const EDITOR_LIST_OP: &str = "editor.list_op";

/// Every command name the editor host publishes today.
pub(crate) const EDITOR_COMMAND_NAMES: [&str; 8] = [
    EDITOR_PHASE,
    EDITOR_LAST_SAVE,
    EDITOR_SET_MODE,
    EDITOR_NEW,
    EDITOR_LOAD,
    EDITOR_SAVE,
    EDITOR_SET_FIELD,
    EDITOR_LIST_OP,
];

/// Command names that need the authoring scene, so they refuse the editor's Load pass.
pub(crate) const EDITOR_EDITING_ONLY: [&str; 6] = [
    EDITOR_SET_MODE,
    EDITOR_NEW,
    EDITOR_LOAD,
    EDITOR_SAVE,
    EDITOR_SET_FIELD,
    EDITOR_LIST_OP,
];

/// Command names that also need the Terrain tab, so they refuse every other tab.
pub(crate) const EDITOR_TERRAIN_TAB_ONLY: [&str; 2] = [EDITOR_SET_FIELD, EDITOR_LIST_OP];

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

pub(crate) fn run_editor_phase(arguments: &str) -> QaRequest {
    run_editor(EDITOR_PHASE, arguments)
}

pub(crate) fn run_editor(command: &'static str, arguments: &str) -> QaRequest {
    QaRequest::Run(RunCommand::new(
        CommandName::from_static(command),
        CommandArgsRon::new(arguments.to_owned()),
    ))
}

pub(crate) fn run_a_misspelled_command_name() -> QaRequest {
    QaRequest::Run(RunCommand::new(
        CommandName::from_static("editor.phasee"),
        CommandArgsRon::new("()".to_owned()),
    ))
}

pub(crate) fn wrong_version() -> ProtocolVersion {
    ProtocolVersion::new(*ProtocolVersion::CURRENT + 1)
}

pub(crate) fn exchange_while_editing(
    app: &mut App,
    port: NetQaPort,
) -> Result<[QaResponse; EDITING_EXCHANGES], TestError> {
    let mut client = Client::connect(port)?;
    Ok([
        client.exchange(app, &QaRequest::Hello(ProtocolVersion::CURRENT))?,
        client.exchange(app, &QaRequest::Hello(wrong_version()))?,
        client.exchange(app, &run_a_misspelled_command_name())?,
    ])
}
