//! Length-prefixed RON frames for the QA wire protocol.

/// Stream decoder for length-prefixed frames.
pub mod decoder;
/// Encode helpers for length-prefixed frames.
pub mod encode;
/// Wire-level errors.
pub mod error;
/// Frame size limits.
pub mod limits;

pub use decoder::{Frame, FrameDecoder};
pub use encode::{encode, encode_frame};
pub use error::WireError;
pub use limits::{FrameLen, MAX_FRAME_LEN, MaxFrameLen};

#[cfg(test)]
mod test;
