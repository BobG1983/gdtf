//! Cover-HP ledger: the model's single authoritative store of cover structural
//! HP, keyed `(cell, level)` — **one** ledger for walls *and* scatter/props.
//!
//! This is the E1.4 cover-ledger slice. Per `docs/combat/resolution.md` §3, cover
//! is a physical object with a **height** and its **own armor stats + HP**, using
//! the same armor model as a ganger (so this module **reuses**
//! [`crate::armor::ArmorProtection`] / [`crate::armor::ArmorHardness`] — cover is
//! not a second armor model). "The HP lives on the model's cover ledger (one
//! ledger for walls *and* props, keyed (cell, level)): `apply_cover_hit` spends
//! it, and depletion emits a cover-destroyed event carrying (cell, level)"
//! (resolution.md §3) — a model-authoritative fact the view mirrors (ADR-0001,
//! `docs/decisions/0001-rust-bevy-rewrite.md`).
//!
//! Three deliberate design points carried from the docs and this ticket:
//!
//! 1. **One unified map.** [`CoverLedger`] keys [`CellLevel`](crate::metric::CellLevel)
//!    → [`CoverEntry`] for BOTH walls and props — there is no second cover-HP
//!    structure anywhere in the sim. A wall and a prop on different `(cell, level)`
//!    keys both live in the same map and are both retrievable.
//! 2. **Lazy seeding.** `current_hp` is seeded to `max_hp` on **first access**, not
//!    pre-populated for every cell of the grid. Querying an absent entry returns a
//!    freshly seeded entry with `current_hp == max_hp` (lazily seeded from each
//!    piece's max HP).
//! 3. **Marker-only depletion.** [`CoverLedger::deplete_cover`] spends HP and, when
//!    `current_hp` reaches zero, sets [`Destroyed`] and returns a
//!    [`CoverEvent::Destroyed`] carrying the [`CellLevel`](crate::metric::CellLevel).
//!    The destroyed-cover occupancy update + prop removal are **deferred to GTW-35**
//!    — this slice emits the marker ONLY and does not act on it.
//!
//! Height-band thresholds are **never** hardcoded here: [`band_for`] classifies a
//! within-level fraction into a [`HeightBand`] by reading the E1.1
//! [`BandEdge`](crate::tuning::BandEdge) level-fraction edges off
//! [`CombatTuning`](crate::tuning::CombatTuning) (`docs/combat/battle-space.md`
//! §"Banding": the band edges are tunable level-fractions ≈ ⅓ and ⅔ of a level,
//! dimensionless and decoupled from any pixel).

mod band;
mod ledger;
mod types;

#[cfg(test)]
mod test;

pub use band::{BandFraction, band_for};
pub use ledger::CoverLedger;
pub use types::{CoverDamage, CoverEntry, CoverEvent, CoverHp, Destroyed, HeightBand};
