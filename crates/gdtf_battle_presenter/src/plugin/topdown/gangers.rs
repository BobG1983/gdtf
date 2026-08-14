use bevy::{ecs::message::Messages, prelude::*};
use gdtf_battle_sim::{prelude::BattleInProgress, shot_fired::ShotFired};

use crate::{
    CharacterRoles, PresenterSystems, ShotImpactResolved, TopDownAtlases, advance_sprite_tweens,
    despawn_killed_ganger_on_impact, despawn_removed_ganger_sprites, move_ganger_sprites,
    resolve_ganger_appearance, resolve_ganger_visibility, spawn_ganger_sprites,
    update_ganger_life_state,
};

pub(super) fn register_ganger_draw(app: &mut App) {
    app.add_message::<ShotFired>();
    let gate = resource_exists::<BattleInProgress>
        .and_then(resource_exists::<CharacterRoles>)
        .and_then(resource_exists::<TopDownAtlases>);
    app.add_systems(
        Update,
        (
            spawn_ganger_sprites,
            move_ganger_sprites.after(spawn_ganger_sprites),
            resolve_ganger_appearance,
            update_ganger_life_state,
        )
            .in_set(PresenterSystems::Scene)
            .run_if(gate),
    )
    .add_systems(
        Update,
        resolve_ganger_visibility
            .in_set(PresenterSystems::Compose)
            .run_if(resource_exists::<BattleInProgress>),
    )
    // Ungated: teardown drops `BattleInProgress`, so a gated system never reads the removal batch.
    .add_systems(
        Update,
        despawn_removed_ganger_sprites.in_set(PresenterSystems::Scene),
    )
    .add_systems(
        Update,
        advance_sprite_tweens
            .in_set(PresenterSystems::Scene)
            .after(move_ganger_sprites)
            .run_if(resource_exists::<BattleInProgress>),
    )
    .add_systems(
        Update,
        despawn_killed_ganger_on_impact
            .in_set(PresenterSystems::Scene)
            .run_if(
                resource_exists::<BattleInProgress>
                    .and_then(resource_exists::<Messages<ShotImpactResolved>>),
            ),
    );
}
