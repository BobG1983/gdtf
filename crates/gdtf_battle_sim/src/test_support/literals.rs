use std::num::NonZeroU8;

use crate::{effects::fields::FieldTurns, weapon::DotTurns};

#[must_use]
pub const fn dot_turns(turns: u8) -> DotTurns {
    match NonZeroU8::new(turns) {
        Some(turns) => DotTurns::new(turns),
        None => unreachable!(),
    }
}

#[must_use]
pub const fn field_turns(turns: u8) -> FieldTurns {
    match NonZeroU8::new(turns) {
        Some(turns) => FieldTurns::new(turns),
        None => unreachable!(),
    }
}
