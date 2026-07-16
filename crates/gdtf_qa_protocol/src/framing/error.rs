//! [`WireError`] — the typed framing / codec error (GTW-734).

use core::fmt::{self, Display};

use super::limits::{FrameLen, MaxFrameLen};

/// A framing / codec failure — the typed error the pure codec returns instead of
/// panicking.
///
/// Local to the codec (it never crosses the wire — a protocol error the peer should
/// SEE is a [`QaError`](crate::envelope::QaError)). `Copy` because every field is a
/// `Copy` newtype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WireError {
    /// A frame declared (or a message serialized to) more than
    /// [`MAX_FRAME_LEN`](crate::framing::MAX_FRAME_LEN) bytes — rejected before any
    /// unbounded allocation.
    Oversize {
        /// The declared / needed payload length.
        declared: FrameLen,
        /// The cap it exceeded.
        max:      MaxFrameLen,
    },
    /// A frame's payload was not valid UTF-8 / compact RON for the requested type —
    /// junk, or a message shape mismatch.
    Malformed,
    /// A message could not be serialized to compact RON (the encode side).
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
