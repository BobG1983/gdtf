//! The child-side link — a [`QaClient`] speaking the protocol's framing over a loopback
//! [`TcpStream`] to a running `net_qa` listener (GTW-741; made host-neutral in GTW-808).
//!
//! The MCP tool layer sends one [`QaRequest`] and blocks for its one [`QaResponse`]
//! through the [`QaLink`] trait; [`QaClient`] is the real implementation. It reuses
//! the protocol crate's length-prefixed compact-RON codec ([`encode()`] +
//! [`FrameDecoder`]) — it never reimplements framing. The connection is opened LAZILY on
//! the first request, so the MCP server starts (and answers `initialize` / `tools/list`)
//! even before the child is up.
//!
//! Nothing here knows which host it is talking to: the game and the editor speak the same
//! framing and the same envelope, so one client type serves both and the dual-target host
//! simply holds two of them, one per port ([`QaHost`]).

use core::{ops::Deref, time::Duration};
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, TcpStream},
};

use gdtf_qa_protocol::{
    envelope::{QaRequest, QaResponse},
    framing::{FrameDecoder, encode},
};

use crate::{error::McpError, hosts::QaHost};

/// The read/write deadline set on the client socket, so a frozen child surfaces as an
/// error instead of hanging the MCP server forever.
const LINK_TIMEOUT: LinkTimeout = LinkTimeout::new(Duration::from_secs(10));

/// The loopback TCP **port** a `net_qa` listener is bound on.
///
/// Private-inner newtype over `u16` (no-bare-types). The interface is always
/// [`Ipv4Addr::LOCALHOST`] — only the port varies, per host
/// ([`QaHost::port_from_env`](crate::hosts::QaHost::port_from_env)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QaPort(u16);

impl QaPort {
    /// Build a port from its number.
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

/// The two-sided socket **timeout** for a host link.
///
/// Private-inner newtype over [`Duration`] (no-bare-types).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct LinkTimeout(Duration);

impl LinkTimeout {
    /// Build a link timeout from its duration.
    const fn new(timeout: Duration) -> Self {
        Self(timeout)
    }
}

impl Deref for LinkTimeout {
    type Target = Duration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A one-request / one-response link to a running child.
///
/// The tool layer depends on this trait, not on [`QaClient`], so the JSON-RPC dispatch
/// can be exercised against a fake host without a socket, and the real client can be
/// swapped for a test double.
pub trait QaLink {
    /// Send one request to the child and block for its single response.
    ///
    /// # Errors
    ///
    /// [`McpError`] when the link cannot be opened, the frame will not encode/decode, or
    /// the socket breaks before a whole response arrives.
    fn request(&mut self, request: QaRequest) -> Result<QaResponse, McpError>;

    /// Re-point the link at `port` — used after a launch selects the port the child
    /// bound, so the following tool calls reach the child that was just started.
    ///
    /// The default is a no-op for links whose target is fixed (a test double).
    fn retarget(&mut self, _port: QaPort) {}
}

/// An open loopback connection plus its incremental frame decoder.
struct Connection {
    /// The loopback socket to the child's `net_qa` listener.
    stream:  TcpStream,
    /// Buffers partial reads until a whole response frame is present.
    decoder: FrameDecoder,
}

impl Connection {
    /// Write the request frame, then read and decode exactly one response frame.
    ///
    /// # Errors
    ///
    /// [`McpError::Io`] on a socket error, [`McpError::Disconnected`] on EOF before a
    /// frame completes, or [`McpError::Wire`] on a framing / decode failure.
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
}

/// The real [`QaLink`] — a loopback client for one host's `net_qa` channel.
///
/// Connects lazily on the first [`request`](QaLink::request); a broken connection is
/// dropped so the next request transparently reconnects.
pub struct QaClient {
    /// The port to connect to.
    port: QaPort,
    /// The open connection, or `None` before the first request / after a failure.
    conn: Option<Connection>,
}

impl QaClient {
    /// Build a client for an explicit port (the connection opens on first use).
    #[must_use]
    pub const fn new(port: QaPort) -> Self {
        Self { port, conn: None }
    }

    /// Build a client for the port `host` reads from ITS own port variable (or that
    /// host's default).
    #[must_use]
    pub fn for_host(host: QaHost) -> Self {
        Self::new(host.port_from_env())
    }

    /// Open the loopback connection if it is not already open.
    ///
    /// # Errors
    ///
    /// [`McpError::Connect`] if the child is not listening, or [`McpError::Io`] if the
    /// socket timeouts cannot be set.
    fn ensure_connected(&mut self) -> Result<(), McpError> {
        if self.conn.is_some() {
            return Ok(());
        }
        let stream =
            TcpStream::connect((Ipv4Addr::LOCALHOST, *self.port)).map_err(McpError::Connect)?;
        stream
            .set_read_timeout(Some(*LINK_TIMEOUT))
            .map_err(McpError::Io)?;
        stream
            .set_write_timeout(Some(*LINK_TIMEOUT))
            .map_err(McpError::Io)?;
        self.conn = Some(Connection {
            stream,
            decoder: FrameDecoder::new(),
        });
        Ok(())
    }
}

impl QaLink for QaClient {
    fn retarget(&mut self, port: QaPort) {
        // A (re)launch puts a NEW game process behind the port — even when the port
        // NUMBER is unchanged (the default is reused across a stop/launch cycle). So
        // ALWAYS drop any open connection: reusing a socket to the old, now-dead process
        // would fail the next request with a broken pipe. The next request reconnects to
        // the process this launch ensured (GTW-755).
        self.port = port;
        self.conn = None;
    }

    fn request(&mut self, request: QaRequest) -> Result<QaResponse, McpError> {
        let frame = encode(&request).map_err(McpError::Wire)?;
        // A connection carried over from a previous call may have been closed underneath
        // us: the game reaps a client left idle past its socket timeout, and a relaunch
        // replaces the process behind the port. Such a stale connection fails the exchange
        // at the transport level BEFORE the game processes the request, so on a REUSED
        // connection we reconnect once and retry — the retry cannot double-execute, since
        // the first attempt never reached a live handler. A FRESH connection's failure is
        // a real error (the game is genuinely unreachable) and is NOT retried, keeping the
        // reconnect bounded to a single fresh attempt (GTW-755).
        let reused = self.conn.is_some();
        match self.try_exchange(&frame) {
            Err(McpError::Io(_) | McpError::Disconnected) if reused => {
                // Drop the dead carried-over connection and try exactly once on a fresh one.
                self.conn = None;
                self.try_exchange(&frame)
            }
            other => other,
        }
    }
}

impl QaClient {
    /// Open the connection if needed, own it for ONE exchange, and put it back on success
    /// (dropping it on any failure so the next call reconnects). A single attempt — the
    /// retry policy lives in [`request`](QaLink::request).
    ///
    /// # Errors
    ///
    /// [`McpError::Connect`] if the connection cannot be opened, or the exchange's own
    /// [`McpError`] (`Io` / `Disconnected` / `Wire`).
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
