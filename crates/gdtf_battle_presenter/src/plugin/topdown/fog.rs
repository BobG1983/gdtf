use bevy::prelude::*;
use gdtf_battle_sim::{prelude::BattleInProgress, visibility::SquadVisibility};

use crate::{
    CombatLogSystems, PresenterSystems, ShownSquadVisibility, TerrainFogMaterial, present_fog,
    promote_shown_fog,
};

pub(super) fn register_fog_systems(app: &mut App) {
    app.init_resource::<ShownSquadVisibility>();
    app.add_systems(
        Update,
        (
            promote_shown_fog
                .before(present_fog)
                .before(CombatLogSystems::Forward)
                .run_if(resource_exists::<SquadVisibility>),
            present_fog.run_if(
                resource_exists::<BattleInProgress>
                    .and_then(resource_exists::<SquadVisibility>)
                    .and_then(resource_exists::<Assets<TerrainFogMaterial>>),
            ),
        )
            .in_set(PresenterSystems::Compose),
    );
}
