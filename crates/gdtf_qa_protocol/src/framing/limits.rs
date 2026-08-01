//! The frame-length newtypes + the oversize cap (GTW-734).

use bevy_derive::Deref;

/// The number of bytes in a frame's length prefix — a big-endian `u32`.
///
/// Encoding plumbing (the fixed width of the prefix), not a domain value; module-
/// private so it never crosses the crate surface.
pub(crate) const PREFIX_BYTES: usize = core::mem::size_of::<u32>();

/// A frame's payload **length**, in bytes — the value the `u32` big-endian prefix
/// carries.
///
/// A private-inner newtype (no-bare-types): the byte count of one framed message. Not
/// a wire type itself (framing is transport plumbing, never serialized as a message),
/// so it derives no serde — only the accessors the codec needs.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameLen(u32);

impl FrameLen {
    /// Build a frame length from its byte count.
    #[must_use]
    pub const fn new(len: u32) -> Self {
        Self(len)
    }

    /// This length as a `usize` (widening from the wire `u32`) — for slicing the byte
    /// buffer.
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    /// The big-endian 4-byte prefix encoding of this length.
    #[must_use]
    pub(crate) const fn to_prefix(self) -> [u8; PREFIX_BYTES] {
        self.0.to_be_bytes()
    }

    /// Read a frame length from its big-endian 4-byte prefix.
    #[must_use]
    pub(crate) const fn from_prefix(bytes: [u8; PREFIX_BYTES]) -> Self {
        Self(u32::from_be_bytes(bytes))
    }
}

/// The maximum payload length a frame may carry — the oversize guard.
///
/// A frame declaring more than this is rejected ([`WireError::Oversize`](crate::framing::WireError::Oversize)),
/// so a malicious or corrupt peer cannot make either half allocate an unbounded
/// buffer. A distinct newtype from [`FrameLen`] (no-bare-types rule 3: a cap is not a
/// length).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaxFrameLen(u32);

impl MaxFrameLen {
    /// Build a maximum frame length from its byte cap.
    #[must_use]
    pub const fn new(max: u32) -> Self {
        Self(max)
    }

    /// This cap as a `usize` — for comparing against a `Vec` length.
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

/// The frame-length cap — 16 MiB.
///
/// Generous enough for a whole battle-roster reply on a
/// large grid, small enough that a single corrupt prefix can never trigger a runaway
/// allocation.
pub const MAX_FRAME_LEN: MaxFrameLen = MaxFrameLen::new(16 * 1024 * 1024);
