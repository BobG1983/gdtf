use bevy::prelude::*;
use gdtf_battle_sim::prelude::Level;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ViewMode {
                #[default]
    DownToActive,
                    FullView,
}

impl ViewMode {
                                    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::DownToActive => Self::FullView,
            Self::FullView => Self::DownToActive,
        }
    }
}

#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActiveLevel(Level);

impl ActiveLevel {
        #[must_use]
    pub const fn new(level: Level) -> Self {
        Self(level)
    }

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

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PresenterSystems {
                    Draw,
                        Replay,
                    Scene,
                Compose,
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
