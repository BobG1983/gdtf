use bevy::prelude::{Component, Deref};

use crate::{armor::BodyPart, severity::Severity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InflictedWound {
                pub tier:     Severity,
            pub location: BodyPart,
}

impl InflictedWound {
                    #[must_use]
    pub const fn new(tier: Severity, location: BodyPart) -> Self {
        Self { tier, location }
    }
}

/// is the sim's own [`record`](InflictedWounds::record) append). A `#[derive(Component)]`
#[derive(Deref, Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct InflictedWounds(Vec<InflictedWound>);

impl InflictedWounds {
                        #[must_use]
    pub const fn new(wounds: Vec<InflictedWound>) -> Self {
        Self(wounds)
    }

                                pub fn record(&mut self, wound: InflictedWound) {
        self.0.push(wound);
    }
}
