//! Wire-level framing failures.

use core::fmt::{self, Display};

use super::limits::{FrameLen, MaxFrameLen};

/// Error while encoding or decoding a length-prefixed frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WireError {
    /// Declared payload length exceeds [`super::MAX_FRAME_LEN`].
    Oversize {
        /// Length claimed in the prefix.
        declared: FrameLen,
        /// Configured maximum.
        max:      MaxFrameLen,
    },
    /// Payload bytes were not valid compact RON (or not UTF-8).
    Malformed,
    /// Message could not be serialized to compact RON.
    Encode,
}

impl Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Oversize { declared, max } => write!(
                f,
                "frame payload of {} bytes exceeds the {} byte cap",
                **declared, **max
            ),
            Self::Malformed => f.write_str("frame payload was not valid compact RON"),
            Self::Encode => f.write_str("message could not be serialized to compact RON"),
        }
    }
}

impl std::error::Error for WireError {}
