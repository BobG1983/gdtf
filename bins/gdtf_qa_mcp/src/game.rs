//! The game-side link — a [`GameClient`] speaking the protocol's framing over a
//! loopback [`TcpStream`] to the running game's `net_qa` listener (GTW-741).
//!
//! The MCP tool layer sends one [`QaRequest`] and blocks for its one [`QaResponse`]
//! through the [`GameLink`] trait; [`GameClient`] is the real implementation. It reuses
//! the protocol crate's length-prefixed compact-RON codec ([`encode()`] +
//! [`FrameDecoder`]) — it never reimplements framing. The connection is opened LAZILY on
//! the first request, so the MCP server starts (and answers `initialize` / `tools/list`)
//! even before the game is up.

use core::{ops::Deref, time::Duration};
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, TcpStream},
};

use gdtf_qa_protocol::{
    envelope::{QaRequest, QaResponse},
    framing::{FrameDecoder, encode},
};

use crate::error::McpError;

/// The environment variable that picks the game's loopback listen port — the same knob
/// the game reads (`net_qa/env.rs`), so both ends agree without configuration.
const GAME_PORT_ENV: &str = "GDTF_NET_QA_PORT";

/// The default loopback port, mirroring the game's `net_qa/config.rs` `DEFAULT_PORT`.
///
/// The two constants are kept in lock-step by hand (the bin cannot import the game's
/// private const without pulling Bevy); if the game's default ever changes, change this.
const DEFAULT_GAME_PORT: GamePort = GamePort::new(7616);

/// The read/write deadline set on the client socket, so a frozen game surfaces as an
/// error instead of hanging the MCP server forever.
const LINK_TIMEOUT: LinkTimeout = LinkTimeout::new(Duration::from_secs(10));

/// The loopback TCP **port** the game's `net_qa` listener is bound on.
///
/// Private-inner newtype over `u16` (no-bare-types). The interface is always
/// [`Ipv4Addr::LOCALHOST`] — only the port varies, via `GDTF_NET_QA_PORT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GamePort(u16);

impl GamePort {
    /// Build a port from its number.
    #[must_use]
    pub const fn new(port: u16) -> Self {
        Self(port)
    }

    /// The port from `GDTF_NET_QA_PORT`, or `DEFAULT_GAME_PORT` when the variable is
    /// unset or does not parse as a `u16` — the same rule the game applies.
    #[must_use]
    pub fn from_env() -> Self {
        std::env::var(GAME_PORT_ENV)
            .ok()
            .and_then(|value| value.trim().parse::<u16>().ok())
            .map_or(DEFAULT_GAME_PORT, Self::new)
    }
}

impl Deref for GamePort {
    type Target = u16;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The two-sided socket **timeout** for the game link.
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

/// A one-request / one-response link to the running game.
///
/// The tool layer depends on this trait, not on [`GameClient`], so the JSON-RPC dispatch
/// can be exercised against a fake game without a socket, and the real client can be
/// swapped for a test double.
pub trait GameLink {
    /// Send one request to the game and block for its single response.
    ///
    /// # Errors
    ///
    /// [`McpError`] when the link cannot be opened, the frame will not encode/decode, or
    /// the socket breaks before a whole response arrives.
    fn request(&mut self, request: QaRequest) -> Result<QaResponse, McpError>;

    /// Re-point the link at `port` — used after `launch_game` selects the port the game
    /// bound, so the following tool calls reach the child that was just started.
    ///
    /// The default is a no-op for links whose target is fixed (a test double).
    fn retarget(&mut self, _port: GamePort) {}
}

/// An open loopback connection plus its incremental frame decoder.
struct Connection {
    /// The loopback socket to the game's `net_qa` listener.
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

/// The real [`GameLink`] — a loopback client for the game's `net_qa` channel.
///
/// Connects lazily on the first [`request`](GameLink::request); a broken connection is
/// dropped so the next request transparently reconnects.
pub struct GameClient {
    /// The port to connect to.
    port: GamePort,
    /// The open connection, or `None` before the first request / after a failure.
    conn: Option<Connection>,
}

impl GameClient {
    /// Build a client for an explicit port (the connection opens on first use).
    #[must_use]
    pub const fn new(port: GamePort) -> Self {
        Self { port, conn: None }
    }

    /// Build a client for the port in `GDTF_NET_QA_PORT` (or the shared default).
    #[must_use]
    pub fn from_env() -> Self {
        Self::new(GamePort::from_env())
    }

    /// Open the loopback connection if it is not already open.
    ///
    /// # Errors
    ///
    /// [`McpError::Connect`] if the game is not listening, or [`McpError::Io`] if the
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

impl GameLink for GameClient {
    fn retarget(&mut self, port: GamePort) {
        if self.port != port {
            self.port = port;
            // Drop any open connection so the next request reconnects to the new port.
            self.conn = None;
        }
    }

    fn request(&mut self, request: QaRequest) -> Result<QaResponse, McpError> {
        let frame = encode(&request).map_err(McpError::Wire)?;
        self.ensure_connected()?;
        // Own the connection for the exchange: on success it goes back for reuse; on any
        // failure it is dropped here (closing the socket) so the next call reconnects.
        let mut conn = self.conn.take().ok_or(McpError::Disconnected)?;
        let result = conn.exchange(&frame);
        if result.is_ok() {
            self.conn = Some(conn);
        }
        result
    }
}
