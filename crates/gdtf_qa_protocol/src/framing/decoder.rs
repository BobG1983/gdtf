//! The decode side — the incremental [`FrameDecoder`] + the extracted [`Frame`]
//! (GTW-734).

use bevy_derive::Deref;
use serde::de::DeserializeOwned;

use super::{
    error::WireError,
    limits::{FrameLen, MAX_FRAME_LEN, PREFIX_BYTES},
};

/// One extracted frame's raw **payload** — the compact-RON bytes between two prefixes.
///
/// A private-inner newtype over the payload bytes (no-bare-types: a frame payload is a
/// domain value, not a bare `Vec<u8>`). The framing layer never parses it — a client
/// turns it into a typed message with [`decode`](Self::decode). Derived `Deref` gives
/// read access to the bytes.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Frame(Vec<u8>);

impl Frame {
    /// The raw payload bytes of this frame.
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.0
    }

    /// Parse this frame's payload as compact RON into `T`.
    ///
    /// # Errors
    ///
    /// [`WireError::Malformed`] if the payload is not valid UTF-8 or does not parse as
    /// `T` (junk, or a shape mismatch). Pure — no I/O.
    pub fn decode<T: DeserializeOwned>(&self) -> Result<T, WireError> {
        let Ok(text) = core::str::from_utf8(&self.0) else {
            return Err(WireError::Malformed);
        };
        ron::de::from_str::<T>(text).map_err(|_| WireError::Malformed)
    }
}

/// An incremental frame decoder — feed it bytes as they arrive, take whole frames out.
///
/// The transport pushes raw reads (which may split a frame anywhere, or carry several
/// frames at once) with [`push`](Self::push); [`next_frame`](Self::next_frame) yields
/// the next COMPLETE frame or `None` while one is still arriving. It tolerates a read
/// split at ANY byte boundary — mid-prefix or mid-payload — because it buffers until a
/// whole frame is present. An oversize declared length is rejected
/// ([`WireError::Oversize`]) BEFORE the payload is buffered, and the decoder stays in
/// that error state (a corrupt prefix desynchronizes the stream irrecoverably). Pure —
/// no I/O.
#[derive(Debug, Default)]
pub struct FrameDecoder {
    /// The as-yet-unframed bytes.
    buffer: Vec<u8>,
    /// Set once an oversize prefix desynchronizes the stream — every later
    /// `next_frame` returns it.
    poison: Option<WireError>,
}

impl FrameDecoder {
    /// Build an empty decoder.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buffer: Vec::new(),
            poison: None,
        }
    }

    /// Append a raw read to the decoder's buffer.
    ///
    /// The read may be any size — a single byte, part of a frame, or several frames.
    pub fn push(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    /// Take the next complete frame, or `None` if one is still arriving.
    ///
    /// Drive a drain loop by calling it until it returns `Ok(None)`.
    ///
    /// # Errors
    ///
    /// [`WireError::Oversize`] (and latches into the error state) when the buffered
    /// prefix declares more than [`MAX_FRAME_LEN`]
    /// bytes.
    pub fn next_frame(&mut self) -> Result<Option<Frame>, WireError> {
        if let Some(err) = self.poison {
            return Err(err);
        }
        // Not even a full length prefix yet.
        if self.buffer.len() < PREFIX_BYTES {
            return Ok(None);
        }
        let Ok(prefix) = <[u8; PREFIX_BYTES]>::try_from(&self.buffer[..PREFIX_BYTES]) else {
            // Unreachable: the slice is exactly `PREFIX_BYTES` long by the guard above.
            return Ok(None);
        };
        let declared = FrameLen::from_prefix(prefix);
        if declared.as_usize() > MAX_FRAME_LEN.as_usize() {
            let err = WireError::Oversize {
                declared,
                max: MAX_FRAME_LEN,
            };
            self.poison = Some(err);
            return Err(err);
        }
        let total = PREFIX_BYTES + declared.as_usize();
        // The whole payload has not arrived yet — wait for more pushes.
        if self.buffer.len() < total {
            return Ok(None);
        }
        let payload = self.buffer[PREFIX_BYTES..total].to_vec();
        self.buffer.drain(..total);
        Ok(Some(Frame(payload)))
    }
}
