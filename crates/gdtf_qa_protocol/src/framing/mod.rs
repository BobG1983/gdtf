//! The length-prefixed frame codec — PURE functions, no I/O (GTW-734).
//!
//! The transport-agnostic framing both halves use to carve a byte stream into
//! messages: a 4-byte big-endian `u32` length prefix followed by that many bytes of
//! compact-RON payload. Nothing here does I/O — the T3 game side and the T8 MCP bridge
//! own the sockets; this crate only turns messages into frames and a byte stream back
//! into frames.
//!
//! - [`limits`] — the [`FrameLen`] / [`MaxFrameLen`] newtypes + the [`MAX_FRAME_LEN`]
//!   cap.
//! - [`error`] — the typed [`WireError`].
//! - [mod@encode] — the encode side: [`encode()`](fn@encode) (message → frame) plus the
//!   raw [`encode_frame`] primitive.
//! - [`decoder`] — the incremental, split-read-tolerant [`FrameDecoder`] yielding
//!   [`Frame`]s.

pub mod decoder;
pub mod encode;
pub mod error;
pub mod limits;

pub use decoder::{Frame, FrameDecoder};
pub use encode::{encode, encode_frame};
pub use error::WireError;
pub use limits::{FrameLen, MAX_FRAME_LEN, MaxFrameLen};

#[cfg(test)]
mod test;
