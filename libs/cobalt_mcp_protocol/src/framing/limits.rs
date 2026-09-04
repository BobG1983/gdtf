//! Frame size types and the maximum payload length.

use bevy_derive::Deref;

pub(crate) const PREFIX_BYTES: usize = core::mem::size_of::<u32>();

/// Declared payload length (big-endian u32 on the wire).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameLen(u32);

impl FrameLen {
    /// Wrap a length value.
    #[must_use]
    pub const fn new(len: u32) -> Self {
        Self(len)
    }

    /// Length as `usize`.
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    /// Encode as the wire prefix.
    #[must_use]
    pub(crate) const fn to_prefix(self) -> [u8; PREFIX_BYTES] {
        self.0.to_be_bytes()
    }

    /// Decode from the wire prefix.
    #[must_use]
    pub(crate) const fn from_prefix(bytes: [u8; PREFIX_BYTES]) -> Self {
        Self(u32::from_be_bytes(bytes))
    }
}

/// Maximum allowed payload length.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaxFrameLen(u32);

impl MaxFrameLen {
    /// Wrap a max value.
    #[must_use]
    pub const fn new(max: u32) -> Self {
        Self(max)
    }

    /// Max as `usize`.
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

/// Hard cap on a single frame payload (16 MiB).
pub const MAX_FRAME_LEN: MaxFrameLen = MaxFrameLen::new(16 * 1024 * 1024);
