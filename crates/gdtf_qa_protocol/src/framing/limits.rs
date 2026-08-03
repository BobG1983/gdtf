use bevy_derive::Deref;

pub(crate) const PREFIX_BYTES: usize = core::mem::size_of::<u32>();

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameLen(u32);

impl FrameLen {
        #[must_use]
    pub const fn new(len: u32) -> Self {
        Self(len)
    }

            #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

        #[must_use]
    pub(crate) const fn to_prefix(self) -> [u8; PREFIX_BYTES] {
        self.0.to_be_bytes()
    }

        #[must_use]
    pub(crate) const fn from_prefix(bytes: [u8; PREFIX_BYTES]) -> Self {
        Self(u32::from_be_bytes(bytes))
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaxFrameLen(u32);

impl MaxFrameLen {
        #[must_use]
    pub const fn new(max: u32) -> Self {
        Self(max)
    }

        #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

pub const MAX_FRAME_LEN: MaxFrameLen = MaxFrameLen::new(16 * 1024 * 1024);
