use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, Default)]
pub enum DamageContext {
        #[default]
    Ranged,
                Melee,
        Fall,
}

impl DamageContext {
                pub const ALL: [Self; 3] = [Self::Ranged, Self::Melee, Self::Fall];
}
