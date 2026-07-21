//! The curated read-model DTOs — the wire snapshot a QA client reads (GTW-734).
//!
//! Independent serde types designed FROM what the sim exposes (ganger vitals, terrain
//! state, squad fog, app flow) but never a LEAK of a sim type — the crate is bevy-free.
//! One concern per file: the ganger stat scalars ([`stat`]), the life-state mirror
//! ([`life`]), the injury summary ([`injury`]), the weapon + indexed fire-mode list
//! ([`weapon`]), the [`ganger`] card, the [`terrain`] summary (with the door /
//! emplacement token handout), the [`fog`] view, the [`appflow`] view, the
//! [`selection`] view, and the top-level [`battle`] aggregate.

pub mod appflow;
pub mod battle;
pub mod fog;
pub mod ganger;
pub mod injury;
pub mod life;
pub mod selection;
pub mod stat;
pub mod terrain;
pub mod weapon;

pub use appflow::{AppFlowView, AppStateNet, BattleActiveNet, CaughtUpNet, RequestKindNet};
pub use battle::{BattleView, TurnView};
pub use fog::FogView;
pub use ganger::GangerView;
pub use injury::{BodyPartNet, InjuryEntryNet, InjuryNameNet, InjurySummaryNet, SeverityNet};
pub use life::LifeStateNet;
pub use selection::SelectionView;
pub use stat::{
    FactionNet, GangerNameNet, HpMaxNet, HpNet, TuMaxNet, TuNet, WoundsMaxNet, WoundsNet,
};
pub use terrain::{
    DoorOpenNet, DoorView, EmplacementMannedNet, EmplacementView, GridHeightNet, GridLevelsNet,
    GridSizeNet, GridWidthNet, TerrainSummaryView,
};
pub use weapon::{FireModeLabel, FireModeView, WeaponNameNet, WeaponView};

#[cfg(test)]
mod test;
