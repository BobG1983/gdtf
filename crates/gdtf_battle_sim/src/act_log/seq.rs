use bevy::prelude::Deref;

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActSeq(u64);

impl ActSeq {
        pub const START: Self = Self(0);

        #[must_use]
    pub const fn new(seq: u64) -> Self {
        Self(seq)
    }

        #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }

                        #[must_use]
    pub const fn distance_from(self, earlier: Self) -> u64 {
        self.0.saturating_sub(earlier.0)
    }
}

impl Default for ActSeq {
        fn default() -> Self {
        Self::START
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActLogCapacity(usize);

impl ActLogCapacity {
                                        pub const DEFAULT: usize = 2048;

        #[must_use]
    pub const fn new(entries: usize) -> Self {
        Self(entries)
    }
}

impl Default for ActLogCapacity {
        fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ActLogDropped(u32);

impl ActLogDropped {
        #[must_use]
    pub const fn new(dropped: u32) -> Self {
        Self(dropped)
    }

        pub const fn increment(&mut self) {
        self.0 = self.0.saturating_add(1);
    }
}
