//! Length-prefixed RON frames for the QA wire protocol.

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
