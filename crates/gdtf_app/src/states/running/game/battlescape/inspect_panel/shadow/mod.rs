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
