//! The encode side — a message / raw payload → a length-prefixed frame (GTW-734).

use serde::Serialize;

use super::{
    error::WireError,
    limits::{FrameLen, MAX_FRAME_LEN, PREFIX_BYTES},
};

/// Frame a raw payload — prepend its big-endian `u32` length prefix.
///
/// The primitive both [`encode`] and the transport reuse: it does NO serialization, it
/// only length-checks against [`MAX_FRAME_LEN`] and
/// prepends the 4-byte prefix. A payload longer than the cap is rejected
/// ([`WireError::Oversize`]) before the allocation; an empty payload frames fine (a
/// zero-length prefix). Pure — no I/O.
///
/// # Errors
///
/// [`WireError::Oversize`] when `payload` is longer than
/// [`MAX_FRAME_LEN`].
pub fn encode_frame(payload: &[u8]) -> Result<Vec<u8>, WireError> {
    // Saturating so an over-`u32` payload still reports a legible length in the error.
    let declared = FrameLen::new(u32::try_from(payload.len()).unwrap_or(u32::MAX));
    if payload.len() > MAX_FRAME_LEN.as_usize() {
        return Err(WireError::Oversize {
            declared,
            max: MAX_FRAME_LEN,
        });
    }
    let mut framed = Vec::with_capacity(PREFIX_BYTES + payload.len());
    framed.extend_from_slice(&declared.to_prefix());
    framed.extend_from_slice(payload);
    Ok(framed)
}

/// Encode a message into a length-prefixed frame — the contract's `encode`: a big-
/// endian `u32` length prefix followed by the compact-RON payload.
///
/// Serializes `message` to compact RON ([`WireError::Encode`] on a serializer failure),
/// then frames it with [`encode_frame`] (which rejects an oversize payload). Pure — no
/// I/O.
///
/// # Errors
///
/// [`WireError::Encode`] if `message` cannot be serialized to compact RON, or
/// [`WireError::Oversize`] if the serialized payload exceeds
/// [`MAX_FRAME_LEN`].
pub fn encode<T: Serialize>(message: &T) -> Result<Vec<u8>, WireError> {
    let Ok(payload) = ron::ser::to_string(message) else {
        return Err(WireError::Encode);
    };
    encode_frame(payload.as_bytes())
}
