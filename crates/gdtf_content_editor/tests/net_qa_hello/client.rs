use std::{
    io::{Read as _, Write as _},
    net::{Ipv4Addr, TcpStream},
    sync::mpsc::Sender,
    time::Duration,
};

use gdtf_net_qa_transport::NetQaPort;
use gdtf_qa_protocol::{
    command::{CommandArgsRon, CommandName},
    framing::{FrameDecoder, encode},
    message::{ProtocolVersion, QaRequest, QaResponse, RunCommand},
};

use crate::support::{ClientResult, TestError};

const READ_TIMEOUT: Duration = Duration::from_secs(10);

struct Client {
    stream:  TcpStream,
    decoder: FrameDecoder,
}

impl Client {
    fn connect(port: NetQaPort) -> Result<Self, TestError> {
        let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, *port))?;
        stream.set_read_timeout(Some(READ_TIMEOUT))?;
        Ok(Self {
            stream,
            decoder: FrameDecoder::new(),
        })
    }

    fn send(&mut self, request: &QaRequest) -> Result<(), TestError> {
        self.stream.write_all(&encode(request)?)?;
        Ok(())
    }

    fn read(&mut self) -> Result<QaResponse, TestError> {
        let mut buf = [0u8; 512];
        loop {
            if let Some(frame) = self.decoder.next_frame()? {
                return Ok(frame.decode::<QaResponse>()?);
            }
            let read = self.stream.read(&mut buf)?;
            assert!(read > 0, "the editor closed before a full response arrived");
            self.decoder.push(&buf[..read]);
        }
    }

    fn exchange(&mut self, request: &QaRequest) -> Result<QaResponse, TestError> {
        self.send(request)?;
        self.read()
    }
}

pub(crate) fn run_a_command_the_editor_has_not_built() -> QaRequest {
    QaRequest::Run(RunCommand::new(
        CommandName::from_static("editor.phase"),
        CommandArgsRon::new("()".to_owned()),
    ))
}

pub(crate) fn wrong_version() -> ProtocolVersion {
    ProtocolVersion::new(*ProtocolVersion::CURRENT + 1)
}

pub(crate) fn exchange_while_editing(port: NetQaPort) -> ClientResult {
    let mut client = Client::connect(port)?;
    Ok(vec![
        client.exchange(&QaRequest::Hello(ProtocolVersion::CURRENT))?,
        client.exchange(&QaRequest::Hello(wrong_version()))?,
        client.exchange(&run_a_command_the_editor_has_not_built())?,
    ])
}

pub(crate) fn exchange_during_load(
    port: NetQaPort,
    opened: &Sender<()>,
    request: QaRequest,
) -> Result<QaResponse, TestError> {
    let mut client = Client::connect(port)?;
    let negotiated = client.exchange(&QaRequest::Hello(ProtocolVersion::CURRENT))?;
    assert!(
        matches!(negotiated, QaResponse::HelloOk(_)),
        "the connection must negotiate before the measured request goes out, got {negotiated:?}",
    );
    client.send(&request)?;
    opened.send(())?;
    client.read()
}
