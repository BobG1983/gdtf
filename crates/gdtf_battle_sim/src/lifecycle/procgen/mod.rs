mod anchor;
mod assembler;
mod deploy;
mod emit;
mod engine;
mod error;
mod fill;
mod findings;
mod geometry;
mod packer;
mod staged;
mod tuning;

#[cfg(test)]
mod test;

pub use anchor::Anchor;
pub use assembler::{PlacedPrefab, Placement, assemble_placement, assemble_placement_with};
pub use deploy::{DeploymentZone, DeploymentZones, Standable, deploy_rosters, facing_for_anchor};
pub use emit::{emit_level, generate_level};
pub use engine::{PlacedFootprint, PlacementRole, ProcgenStage, StagedProcgenRegistries};
pub use error::{PackingError, RosterDemand, ZoneCapacity};
pub use fill::{FilledPlacement, fill_placement, fill_placement_with};
pub use findings::{EmittedLevel, ProcgenFinding};
pub use geometry::{
    CellCount, Footprint, Margin, MinPlayerSide, RectContains, RectNonEmpty, RectsIntersect,
    RegionRect,
};
pub use packer::{MaxRectsPacker, SplitMode};
pub use staged::StagedProcgen;
pub use tuning::{
    DeadRectScatterCount, LargePrefabAreaThreshold, MaxCoverageCap, MinDensityFloor, ProcgenTuning,
    ScatterCount,
};
