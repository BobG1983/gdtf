//! App extension to register FX readers gated on battle and assets.

use bevy::{
    ecs::{
        message::{Message, Messages},
        schedule::SystemCondition,
        system::ScheduleSystem,
    },
    prelude::{App, IntoScheduleConfigs, Update, resource_exists},
};
use gdtf_battle_sim::prelude::BattleInProgress;

use super::roles::EffectRoles;
use crate::{PresenterSystems, TopDownAtlases};

/// Register an FX reader system under the Overlay set with common run conditions.
pub trait FxReaderAppExt {
    /// Add a reader for message type `M`.
    fn add_fx_reader<M: Message, Marker>(
        &mut self,
        reader: impl IntoScheduleConfigs<ScheduleSystem, Marker>,
    ) -> &mut Self;
}

impl FxReaderAppExt for App {
    fn add_fx_reader<M: Message, Marker>(
        &mut self,
        reader: impl IntoScheduleConfigs<ScheduleSystem, Marker>,
    ) -> &mut Self {
        self.add_systems(
            Update,
            reader.in_set(PresenterSystems::Overlay).run_if(
                resource_exists::<BattleInProgress>
                    .and_then(resource_exists::<EffectRoles>)
                    .and_then(resource_exists::<TopDownAtlases>)
                    .and_then(resource_exists::<Messages<M>>),
            ),
        );
        self
    }
}
