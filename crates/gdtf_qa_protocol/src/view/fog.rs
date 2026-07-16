//! [`FogView`] — the player squad's fog-of-war summary (GTW-734).

use serde::{Deserialize, Serialize};

use crate::ids::CellLevelNet;

/// The player squad's **fog** snapshot — which `(cell, level)` keys are currently
/// visible and which have ever been explored.
///
/// A curated summary of the sim `SquadVisibility`: the two cell lists a QA client needs
/// to reason about what the squad can see (targeting / move planning gate on
/// visibility). Independent serde lists of [`CellLevelNet`] keys — never a leak of the
/// sim fog grid. A cell in [`visible`](Self::visible) is also in
/// [`explored`](Self::explored) (visible implies explored), but both are carried so the
/// wire is self-describing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FogView {
    /// The `(cell, level)` keys currently visible to the player squad.
    pub visible:  Vec<CellLevelNet>,
    /// The `(cell, level)` keys the player squad has ever seen.
    pub explored: Vec<CellLevelNet>,
}

impl FogView {
    /// Build a fog view from its visible + explored cell lists.
    #[must_use]
    pub const fn new(visible: Vec<CellLevelNet>, explored: Vec<CellLevelNet>) -> Self {
        Self { visible, explored }
    }
}
