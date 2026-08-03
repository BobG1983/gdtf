use bevy::prelude::Deref;

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandsAvailable(u8);

impl HandsAvailable {
            pub(crate) const MAX: u8 = 2;

            #[must_use]
    pub const fn new(hands: u8) -> Self {
        Self(if hands > Self::MAX { Self::MAX } else { hands })
    }
}

impl Default for HandsAvailable {
        fn default() -> Self {
        Self(Self::MAX)
    }
}
