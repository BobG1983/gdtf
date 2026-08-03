use core::fmt::{self, Display};

use super::limits::{FrameLen, MaxFrameLen};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WireError {
                Oversize {
                declared: FrameLen,
                max:      MaxFrameLen,
    },
            Malformed,
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
