//! Curated prelude — the ~dozen genuinely ubiquitous sim types.
//!
//! Everything else imports concern-pathed (`gdtf_battle_sim::ganger::…`,
//! `gdtf_battle_sim::effects::fields::…`); this module exists so the handful
//! of types that appear across nearly every sim consumer don't drag their
//! concern path into every import block. Membership is CURATED (GTW-628, the
//! Q3 user ruling): the [`crate::foundation::metric`] coordinate family, the
//! core ganger identity/state components, the battle-active witness, and the
//! occupancy grid — each earned its slot by measured ubiquity (≥50 consumer
//! import sites at curation time). Grow it only for a type that is genuinely
//! ubiquitous across consumer crates, never for import convenience.
//!
//! This is the ONE cross-concern re-export surface below the crate
//! root: the whole point of a prelude is to gather the ubiquitous names in
//! one place, which the GTW-628 Q3 user ruling approved explicitly.

pub use crate::{
    combatants::ganger::{Direction, Faction, LifeState, Position, Stance, StanceKind, Tu},
    foundation::metric::{Cell, CellLevel, Level, SimPos},
    lifecycle::battle::BattleInProgress,
    terrain::occupancy::OccupancyGrid,
};
