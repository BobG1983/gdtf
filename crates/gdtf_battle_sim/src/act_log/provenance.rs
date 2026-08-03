use bevy::prelude::Entity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActProvenance {
        Commanded,
            AiTurn,
            Reaction {
                                interrupted: Entity,
    },
            Clock,
}

impl ActProvenance {
        #[must_use]
    pub const fn is_reaction(self) -> bool {
        matches!(self, Self::Reaction { .. })
    }
}
