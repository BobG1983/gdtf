use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommandTiming {
        #[default]
    Immediate,
        Deferred,
}

impl CommandTiming {
                    pub const ALL: [Self; 2] = [Self::Immediate, Self::Deferred];
}
