use core::{ops::Deref, time::Duration};
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, TcpStream},
};

use gdtf_qa_protocol::{
    framing::{FrameDecoder, encode},
    message::{ProtocolVersion, QaRequest, QaResponse},
};

use crate::{error::McpError, hosts::QaHost};

pub const LINK_TIMEOUT: LinkTimeout = LinkTimeout::new(Duration::from_secs(10));

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QaPort(u16);

impl QaPort {
        #[must_use]
    pub const fn new(port: u16) -> Self {
        Self(port)
    }
}

impl Deref for QaPort {
    type Target = u16;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LinkTimeout(Duration);

impl LinkTimeout {
        #[must_use]
    pub const fn new(timeout: Duration) -> Self {
        Self(timeout)
    }
}

impl Deref for LinkTimeout {
    type Target = Duration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub trait QaLink {
                            fn request(&mut self, request: QaRequest) -> Result<QaResponse, McpError>;

                    fn retarget(&mut self, _port: QaPort) {}
}

struct Connection {
        stream:  TcpStream,
        decoder: FrameDecoder,
}

impl Connection {
                            fn exchange(&mut self, frame: &[u8]) -> Result<QaResponse, McpError> {
        self.stream.write_all(frame).map_err(McpError::Io)?;
        let mut buf = [0u8; 4096];
        loop {
            if let Some(frame) = self.decoder.next_frame().map_err(McpError::Wire)? {
                return frame.decode::<QaResponse>().map_err(McpError::Wire);
            }
            let read = match self.stream.read(&mut buf) {
                Ok(0) => return Err(McpError::Disconnected),
                Ok(count) => count,
                Err(err) => return Err(McpError::Io(err)),
            };
            self.decoder.push(&buf[..read]);
        }
    }

                                                    fn negotiate(&mut self) -> Result<(), McpError> {
        let frame = encode(&QaRequest::Hello(ProtocolVersion::CURRENT)).map_err(McpError::Wire)?;
        match self.exchange(&frame)? {
            QaResponse::HelloOk(_) => Ok(()),
            QaResponse::Error(err) => Err(McpError::Handshake(err)),
            _ => Err(McpError::UnexpectedResponse),
        }
    }
}

pub struct QaClient {
        port:    QaPort,
        timeout: LinkTimeout,
        conn:    Option<Connection>,
}

impl QaClient {
            #[must_use]
    pub const fn new(port: QaPort) -> Self {
        Self::with_timeout(port, LINK_TIMEOUT)
    }

                        #[must_use]
    pub const fn with_timeout(port: QaPort, timeout: LinkTimeout) -> Self {
        Self {
            port,
            timeout,
            conn: None,
        }
    }

            #[must_use]
    pub fn for_host(host: QaHost) -> Self {
        Self::new(host.port_from_env())
    }

                                                    fn ensure_connected(&mut self) -> Result<(), McpError> {
        if self.conn.is_some() {
            return Ok(());
        }
        let stream =
            TcpStream::connect((Ipv4Addr::LOCALHOST, *self.port)).map_err(McpError::Connect)?;
        stream
            .set_read_timeout(Some(*self.timeout))
            .map_err(McpError::Io)?;
        stream
            .set_write_timeout(Some(*self.timeout))
            .map_err(McpError::Io)?;
        let mut conn = Connection {
            stream,
            decoder: FrameDecoder::new(),
        };
        conn.negotiate()?;
        self.conn = Some(conn);
        Ok(())
    }
}

impl QaLink for QaClient {
    fn retarget(&mut self, port: QaPort) {
        self.port = port;
        self.conn = None;
    }

    fn request(&mut self, request: QaRequest) -> Result<QaResponse, McpError> {
        let frame = encode(&request).map_err(McpError::Wire)?;
        let reused = self.conn.is_some();
        match self.try_exchange(&frame) {
            Err(McpError::Io(_) | McpError::Disconnected) if reused => {
                self.conn = None;
                self.try_exchange(&frame)
            }
            other => other,
        }
    }
}

impl QaClient {
                                    fn try_exchange(&mut self, frame: &[u8]) -> Result<QaResponse, McpError> {
        self.ensure_connected()?;
        let mut conn = self.conn.take().ok_or(McpError::Disconnected)?;
        let result = conn.exchange(frame);
        if result.is_ok() {
            self.conn = Some(conn);
        }
        result
    }
}
