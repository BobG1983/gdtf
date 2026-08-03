use bevy::prelude::Entity;
use gdtf_battle_sim::{
    acts::{FireRequested, MoveRequested, SetFacingRequested},
    prelude::StanceKind,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ActIntent {
        SelectionClear,
        LevelUp,
        LevelDown,
                                        ToggleFullView,
                        StanceCycle,
                    SetStance(StanceKind),
            AimToggle,
            FacingCycle,
                Fire(FireRequested),
                        Move(MoveRequested),
                            Turn(SetFacingRequested),
                        Reload,
                                    EndTurn,
                                    SelectNext,
                        SelectPrev,
                                                                    Select(Entity),
}

impl ActIntent {
                                                        #[must_use]
    pub const fn needs_caught_up(&self) -> bool {
        !matches!(self, Self::LevelUp | Self::LevelDown | Self::ToggleFullView)
    }
}
