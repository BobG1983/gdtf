//! Apply integrity wear to armor pieces.

#[cfg(test)]
mod test;
mod wear;

pub use wear::{ArmorBroken, ArmorDamaged, ArmorWearOutcome, wear_armor};
