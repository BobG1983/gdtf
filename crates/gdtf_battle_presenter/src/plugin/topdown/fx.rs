use bevy::{ecs::message::Messages, prelude::*};
use gdtf_battle_sim::{
    acts::{MeleeResolved, ThrowResolved},
    armor_wear::ArmorBroken,
    effects::bleed::Bleeding,
    falls::FallOccurred,
    occupancy_sync::TerrainPieceDestroyed,
    prelude::BattleInProgress,
    shot_fired::ShotFired,
};

use crate::{
    ArmorBrokenFct, BleedingFct, ConsequenceFctAppExt, DotFct, EffectRoles, FieldFct,
    FxReaderAppExt, FxTuning, InjuryFct, OnDeathFct, Played, PresenterSystems, ShotImpactResolved,
    SuppressionFct, TopDownAtlases, advance_impact_animations, advance_projectiles,
    animate_floating_text, expire_flashes, read_armor_broken, read_bleeding, read_cover_destroyed,
    read_fall_occurred, read_melee_resolved, read_throw_resolved, register_consequence_fct_core,
    seed_impact_animations, spawn_shot_projectiles,
};

pub(super) fn register_fx_flash_systems(app: &mut App) {
    app.add_message::<ShotImpactResolved>();
    app.add_fx_reader::<Bleeding, _>(read_bleeding)
        .add_fx_reader::<ArmorBroken, _>(read_armor_broken)
        .add_fx_reader::<Played<TerrainPieceDestroyed>, _>(read_cover_destroyed)
        .add_fx_reader::<MeleeResolved, _>(read_melee_resolved)
        .add_fx_reader::<FallOccurred, _>(
            read_fall_occurred
                .after(animate_floating_text)
                .run_if(resource_exists::<FxTuning>),
        )
        .add_fx_reader::<ThrowResolved, _>(read_throw_resolved);
    let render_gate = resource_exists::<BattleInProgress>
        .and_then(resource_exists::<EffectRoles>)
        .and_then(resource_exists::<TopDownAtlases>);
    app.add_systems(
        Update,
        spawn_shot_projectiles
            .in_set(PresenterSystems::Overlay)
            .after(animate_floating_text)
            .run_if(
                render_gate
                    .clone()
                    .and_then(resource_exists::<FxTuning>)
                    .and_then(resource_exists::<Messages<ShotFired>>),
            ),
    )
    .add_systems(
        Update,
        // No sync point between the two: a flash seeded this frame must not also
        // be ticked this frame.
        (seed_impact_animations, advance_impact_animations)
            .chain_ignore_deferred()
            .in_set(PresenterSystems::Overlay)
            .after(animate_floating_text)
            .run_if(render_gate.and_then(resource_exists::<FxTuning>)),
    )
    .add_systems(
        Update,
        advance_projectiles.in_set(PresenterSystems::Overlay),
    )
    .add_systems(
        Update,
        animate_floating_text.in_set(PresenterSystems::Overlay),
    )
    .add_systems(Update, expire_flashes.in_set(PresenterSystems::Overlay));
}

pub(super) fn register_consequence_fct_families(app: &mut App) {
    register_consequence_fct_core(app);
    app.add_consequence_fct::<BleedingFct>()
        .add_consequence_fct::<ArmorBrokenFct>()
        .add_consequence_fct::<InjuryFct>()
        .add_consequence_fct::<SuppressionFct>()
        .add_consequence_fct::<DotFct>()
        .add_consequence_fct::<FieldFct>()
        .add_consequence_fct::<OnDeathFct>();
}
