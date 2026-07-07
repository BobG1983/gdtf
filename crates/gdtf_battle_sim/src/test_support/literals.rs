//! Typed **fixture-literal** builders — the terse `const fn` helpers that spell a
//! non-trivially-constructible domain value from a plain test literal, shared by the
//! sim's unit tests and the downstream crates' integration tests (the [`super`]
//! test-support convention). The home for `NonZero`-backed literal builders: a type
//! whose invariant makes the bare literal unspellable gets its one helper HERE, not a
//! per-test copy.

use std::num::NonZeroU8;

use crate::weapon::DotTurns;

/// A [`DotTurns`] from a positive TEST literal — the terse fixture helper for DOT
/// durations (a zero-turn duration is unrepresentable — [`DotTurns`] wraps
/// [`NonZeroU8`], GTW-643). A fixture literal is never `0`; the zero arm
/// `unreachable!`s — in the `const` position the fixtures use, a `0` is a COMPILE
/// error, never a silent clamp.
#[must_use]
pub const fn dot_turns(turns: u8) -> DotTurns {
    match NonZeroU8::new(turns) {
        Some(turns) => DotTurns::new(turns),
        // A test fixture's DOT duration literal is always positive (zero is the
        // unrepresentable state this helper exists to spell safely).
        None => unreachable!(),
    }
}
