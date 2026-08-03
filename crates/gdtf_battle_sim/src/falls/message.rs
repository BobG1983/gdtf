use bevy::prelude::{Entity, Message};

use crate::metric::Level;

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FallOccurred {
            pub ganger:     Entity,
            pub from_level: Level,
            pub to_level:   Level,
            pub storeys:    StoreysFallen,
}

impl FallOccurred {
            #[must_use]
    pub const fn new(
        ganger: Entity,
        from_level: Level,
        to_level: Level,
        storeys: StoreysFallen,
    ) -> Self {
        Self {
            ganger,
            from_level,
            to_level,
            storeys,
        }
    }
}

#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StoreysFallen(u8);

impl StoreysFallen {
        #[must_use]
    pub const fn new(storeys: u8) -> Self {
        Self(storeys)
    }
}
