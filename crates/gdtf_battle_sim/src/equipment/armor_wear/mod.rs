//! **message** (`#[derive(Message)]` — Bevy 0.18 renamed buffered
#[cfg(test)]
mod test;
mod wear;

pub use wear::{ArmorBroken, ArmorDamaged, ArmorWearOutcome, wear_armor};
