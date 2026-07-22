//! The inspect panel's cursor-time SHADOW resources and their promote systems (GTW-762):
//! grid-shaped snapshots of the sim's live [`OccupancyGrid`](gdtf_battle_sim::prelude::OccupancyGrid)
//! and [`CoverLedger`](gdtf_battle_sim::cover::CoverLedger) that freeze during closed-gate
//! playback and refresh the instant the cursor catches up, so the inspect panel reads
//! cursor-time state instead of the sim's live (possibly-ahead-of-the-view) state.

mod promote;
mod resources;

#[cfg(test)]
mod test;

pub(in crate::states::running::game::battlescape::inspect_panel) use promote::{
    promote_shown_cover, promote_shown_occupancy,
};
pub(in crate::states::running::game::battlescape::inspect_panel) use resources::{
    ShownCoverLedger, ShownOccupancyGrid,
};
