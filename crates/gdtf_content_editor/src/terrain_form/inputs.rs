//! The TERRAIN form's **numeric-input newtypes** (GTW-474; split out of `types.rs` in
//! GTW-574): the [`NumericValue`](gdtf_ui::NumericValue)-shaped wrappers the HP / armor
//! fields edit through.

use bevy::prelude::Deref;

/// An HP magnitude the TERRAIN form's HP numeric field edits, clamps, and commits — the
/// [`NumericValue`](gdtf_ui::NumericValue) generic the field is built over (GTW-474).
///
/// A named newtype over [`u32`] (no-bare-types rule 1: a numeric-field generic is a domain value,
/// never a bare `u32`; the sim's [`CoverHp`](gdtf_battle_sim::cover::CoverHp) /
/// [`SlabHp`](gdtf_battle_sim::slab::SlabHp) do not impl `Display` / `FromStr`, so
/// this thin input newtype satisfies the [`NumericValue`](gdtf_ui::NumericValue) bound and is
/// converted into the right HP newtype on commit). Private inner + derived [`Deref`].
#[derive(Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct HpInput(u32);

impl HpInput {
    /// Wrap an HP magnitude.
    #[must_use]
    pub const fn new(hp: u32) -> Self {
        Self(hp)
    }
}

impl core::fmt::Display for HpInput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::str::FromStr for HpInput {
    type Err = core::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u32>().map(Self)
    }
}

/// An armor magnitude the TERRAIN form's armor numeric fields edit, clamp, and commit — the
/// [`NumericValue`](gdtf_ui::NumericValue) generic the protection + hardness fields share
/// (GTW-474).
///
/// A named newtype over [`i32`] (no-bare-types rule 1; the sim's
/// [`ArmorProtection`](gdtf_battle_sim::armor::ArmorProtection) /
/// [`ArmorHardness`](gdtf_battle_sim::armor::ArmorHardness) do not impl `Display` / `FromStr`).
/// Private inner + derived [`Deref`]; converted into the right armor newtype on commit.
#[derive(Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct ArmorInput(i32);

impl ArmorInput {
    /// Wrap an armor magnitude.
    #[must_use]
    pub const fn new(armor: i32) -> Self {
        Self(armor)
    }
}

impl core::fmt::Display for ArmorInput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::str::FromStr for ArmorInput {
    type Err = core::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<i32>().map(Self)
    }
}
