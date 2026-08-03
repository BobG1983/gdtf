//! Non-zero turn helpers for tests.

use std::num::NonZeroU8;

use crate::{effects::fields::FieldTurns, weapon::DotTurns};

/// `DotTurns` from a non-zero count (panics on zero).
#[must_use]
pub const fn dot_turns(turns: u8) -> DotTurns {
    match NonZeroU8::new(turns) {
        Some(turns) => DotTurns::new(turns),
        None => unreachable!(),
    }
}

/// `FieldTurns` from a non-zero count (panics on zero).
#[must_use]
pub const fn field_turns(turns: u8) -> FieldTurns {
    match NonZeroU8::new(turns) {
        Some(turns) => FieldTurns::new(turns),
        None => unreachable!(),
    }
}
