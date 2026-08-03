use bevy::prelude::*;

use super::ActIntent;

#[derive(Resource, Debug, Default)]
pub struct PendingActIntent(Vec<ActIntent>);

impl PendingActIntent {
                                    pub fn push(&mut self, intent: ActIntent) {
        self.0.push(intent);
    }

                        pub(crate) fn drain(&mut self) -> Vec<ActIntent> {
        core::mem::take(&mut self.0)
    }

        #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
