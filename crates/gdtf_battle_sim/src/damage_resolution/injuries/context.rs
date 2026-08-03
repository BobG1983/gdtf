//! How the injury was caused (ranged, melee, or fall).

use serde::{Deserialize, Serialize};

/// Source of the damage that produced an injury roll.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, Default)]
pub enum DamageContext {
    /// Shot or other ranged hit.
    #[default]
    Ranged,
    /// Melee strike.
    Melee,
    /// Fall damage.
    Fall,
}

impl DamageContext {
    /// All contexts.
    pub const ALL: [Self; 3] = [Self::Ranged, Self::Melee, Self::Fall];
}
