//! The **TU-economy primitives** the rest of E4 spends through — model-authoritative
mod economy;
#[cfg(test)]
mod test;

pub use economy::{TuAffordable, can_spend_tu, reset_tu, spend_tu};
