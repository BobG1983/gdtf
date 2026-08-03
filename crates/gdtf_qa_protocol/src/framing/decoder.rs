use bevy_derive::Deref;
use serde::de::DeserializeOwned;

use super::{
    error::WireError,
    limits::{FrameLen, MAX_FRAME_LEN, PREFIX_BYTES},
};

/// One decoded payload (bytes after the length prefix).
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Frame(Vec<u8>);

impl Frame {
    /// Borrow the payload bytes.
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.0
    }

    /// Deserialize the payload as compact RON into `T`.
    ///
    /// # Errors
    ///
    /// Returns [`WireError::Malformed`] if the bytes are not UTF-8 or not valid RON for `T`.
    pub fn decode<T: DeserializeOwned>(&self) -> Result<T, WireError> {
        let Ok(text) = core::str::from_utf8(&self.0) else {
            return Err(WireError::Malformed);
        };
        ron::de::from_str::<T>(text).map_err(|_| WireError::Malformed)
    }
}

/// Streaming length-prefix frame decoder.
#[derive(Debug, Default)]
pub struct FrameDecoder {
    buffer: Vec<u8>,
    poison: Option<WireError>,
}

impl FrameDecoder {
    /// Empty decoder.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buffer: Vec::new(),
            poison: None,
        }
    }

    /// Append bytes from the socket.
    pub fn push(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    /// Pull the next complete frame, if any.
    ///
    /// # Errors
    ///
    /// Returns [`WireError::Oversize`] when a declared length exceeds the max frame size;
    /// the decoder stays poisoned and keeps returning that error on later calls.
    pub fn next_frame(&mut self) -> Result<Option<Frame>, WireError> {
        if let Some(err) = self.poison {
            return Err(err);
        }
        if self.buffer.len() < PREFIX_BYTES {
            return Ok(None);
        }
        let Ok(prefix) = <[u8; PREFIX_BYTES]>::try_from(&self.buffer[..PREFIX_BYTES]) else {
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
        if self.buffer.len() < total {
            return Ok(None);
        }
        let payload = self.buffer[PREFIX_BYTES..total].to_vec();
        self.buffer.drain(..total);
        Ok(Some(Frame(payload)))
    }
}
