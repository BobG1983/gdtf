//! TCP client for a host's MCP channel.

use core::{ops::Deref, time::Duration};
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, TcpStream},
};

use cobalt_mcp_protocol::{
    framing::{FrameDecoder, encode},
    message::{ProtocolVersion, QaRequest, QaResponse},
};

use crate::{error::McpError, hosts::QaHostSpec, lifecycle::EnvVarName};

/// Default connect/read/write timeout for a QA link.
///
/// Outlives the host's own reply wait so a slow command answers rather than breaking the socket.
pub const LINK_TIMEOUT: LinkTimeout = LinkTimeout::new(Duration::from_secs(200));

/// TCP port of a host's MCP listener.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QaPort(u16);

impl QaPort {
    /// Wrap a port number.
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

/// Timeout applied to connect, read, and write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LinkTimeout(Duration);

impl LinkTimeout {
    /// Wrap a duration.
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

/// Send QA requests and receive responses.
pub trait QaLink {
    /// Exchange one request/response.
    ///
    /// # Errors
    ///
    /// Returns [`McpError`] on connect, I/O, framing, or handshake failure.
    fn request(&mut self, request: QaRequest) -> Result<QaResponse, McpError>;

    /// Point this link at a different port (drops any open connection).
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

/// Default TCP implementation of [`QaLink`].
pub struct QaClient {
    port:    QaPort,
    channel: EnvVarName,
    timeout: LinkTimeout,
    conn:    Option<Connection>,
}

impl QaClient {
    /// Client for `port`, naming the env var that enables that channel, with the default timeout.
    #[must_use]
    pub const fn new(port: QaPort, channel: EnvVarName) -> Self {
        Self::with_timeout(port, channel, LINK_TIMEOUT)
    }

    /// Client for `port` with an explicit timeout.
    #[must_use]
    pub const fn with_timeout(port: QaPort, channel: EnvVarName, timeout: LinkTimeout) -> Self {
        Self {
            port,
            channel,
            timeout,
            conn: None,
        }
    }

    /// Client for a registered host's env-resolved port and channel.
    #[must_use]
    pub fn for_host(host: &QaHostSpec) -> Self {
        Self::new(host.port_from_env(), host.channel().enable().clone())
    }

    /// Port this client is aimed at.
    #[must_use]
    pub const fn port(&self) -> QaPort {
        self.port
    }

    /// Env var name that enables the channel this client talks to.
    #[must_use]
    pub const fn channel(&self) -> &EnvVarName {
        &self.channel
    }

    fn ensure_connected(&mut self) -> Result<(), McpError> {
        if self.conn.is_some() {
            return Ok(());
        }
        let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, *self.port)).map_err(|error| {
            McpError::Connect {
                channel: self.channel.clone(),
                error,
            }
        })?;
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
