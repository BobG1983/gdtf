//! Active storey, view mode, and presenter system sets.

use bevy::prelude::*;
use gdtf_battle_sim::prelude::Level;

/// Whether the camera shows only storeys at or below the active level, or everything.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ViewMode {
    /// Hide storeys above the active level.
    #[default]
    DownToActive,
    /// Show every storey with context treatment.
    FullView,
}

impl ViewMode {
    /// Flip between the two view modes.
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::DownToActive => Self::FullView,
            Self::FullView => Self::DownToActive,
        }
    }
}

/// The storey the player is currently focused on.
#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActiveLevel(Level);

impl ActiveLevel {
    /// Build from a sim level.
    #[must_use]
    pub const fn new(level: Level) -> Self {
        Self(level)
    }

    /// Whether `storey` should be drawn under `mode`.
    #[must_use]
    pub fn draws_storey(&self, storey: Level, mode: super::treatment::StoreyViewMode) -> bool {
        super::treatment::storey_treatment(storey, *self, mode)
            != super::treatment::StoreyTreatment::Hidden
    }
}

impl Default for ActiveLevel {
    fn default() -> Self {
        Self::new(Level::new(0))
    }
}

/// Ordered system sets inside the presenter draw pipeline.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PresenterSystems {
    /// Parent set for all presenter draw work.
    Draw,
    /// Replay / act-log cursor advance.
    Replay,
    /// Scene graph updates from sim state.
    Scene,
    /// Compose drawn actors and terrain.
    Compose,
    /// HUD overlays on top.
    Overlay,
}

#[cfg(test)]
mod tests {
    use super::ViewMode;

    #[test]
    fn toggled_flips_between_the_two_modes() {
        assert_eq!(ViewMode::DownToActive.toggled(), ViewMode::FullView);
        assert_eq!(ViewMode::FullView.toggled(), ViewMode::DownToActive);
        assert_eq!(
            ViewMode::DownToActive.toggled().toggled(),
            ViewMode::DownToActive,
            "double toggle is the identity",
        );
    }
}
