//! Cover HP ledger: structural hit points and height bands for cover cells.

mod band;
mod ledger;
mod types;

#[cfg(test)]
mod test;

pub use band::{BandFraction, BandRank, band_for};
pub use ledger::CoverLedger;
pub use types::{CoverDamage, CoverEntry, CoverEvent, CoverHp, Destroyed, HeightBand};
