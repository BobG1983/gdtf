//! GTW-646 spend-integrity helpers shared by the `spend_integrity` /
//! `suppressed_reactor` pins: position-keyed ganger lookup, direct TU / suppression
//! state mutators (the test-body idiom), and the shooter's single-shot fire cost
//! resolved through the SAME `mode_tu_cost` source the dispatcher charges.

use bevy::{app::App, ecs::relationship::RelationshipTarget as _, prelude::Entity};
use gdtf_battle_sim::{
    ganger::{Aiming, Suppressed, SuppressorCell, TuMax},
    magazine::mode_tu_cost,
    metric::CellLevel,
    prelude::{Position, Tu},
    tuning::CombatTuning,
    weapon::{FireMode, Wields},
};

/// The entity of the ganger currently standing at `at` — a position-keyed lookup, so a
/// multi-ganger fixture can address each spawn unambiguously (spawn order is not
/// entity-order-guaranteed through the deferred scene path).
pub(crate) fn ganger_at(app: &mut App, at: CellLevel) -> Option<Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Position)>();
    query
        .iter(world)
        .find(|(_, position)| ***position == at)
        .map(|(entity, _)| entity)
}

/// Overwrite `entity`'s current TU pool (the direct test-body mutator — the
/// `slab_destroyed_bridge` idiom), so a fixture can pin a reactor's affordability
/// exactly.
pub(crate) fn set_tu(app: &mut App, entity: Entity, value: u8) {
    let Some(mut tu) = app.world_mut().get_mut::<Tu>(entity) else {
        unreachable!("the fixture ganger carries a Tu pool");
    };
    *tu = Tu::new(value);
}

/// Mark `entity` as `Suppressed` anchored at `from` (the direct test-body idiom the
/// `suppression_core` suite uses), isolating the reaction gate from the producer.
pub(crate) fn suppress(app: &mut App, entity: Entity, from: CellLevel) {
    app.world_mut()
        .entity_mut(entity)
        .insert(Suppressed::new(SuppressorCell::new(from)));
}

/// The `single`-mode `FireModeSpec` of `shooter`, resolved off its wielded RANGED
/// weapon (the `suppression_core` harness resolver — reads what the setup armed the
/// shooter with, not a canonical fixture).
pub(crate) fn wielded_single_mode(
    app: &mut App,
    shooter: Entity,
) -> gdtf_battle_sim::weapon::FireModeSpec {
    let world = app.world_mut();
    let wielded: Vec<Entity> = {
        let Some(wields) = world.get::<Wields>(shooter) else {
            unreachable!("the shooter wields weapons at setup");
        };
        wields.iter().collect()
    };
    let mode = wielded
        .into_iter()
        .find_map(|entity| world.get::<FireMode>(entity).map(FireMode::single));
    let Some(mode) = mode else {
        unreachable!("the shooter wields a ranged weapon carrying a FireMode");
    };
    mode
}

/// The TU a single-mode interrupt shot costs `shooter` — computed through the ONE
/// shared charge source (`mode_tu_cost`) the trigger's gate AND the dispatcher's
/// debit both read, so the fixture's affordability boundaries are exact by
/// construction.
pub(crate) fn single_fire_cost(app: &mut App, shooter: Entity) -> u8 {
    let mode = wielded_single_mode(app, shooter);
    let world = app.world();
    let (Some(tu_max), Some(aiming), Some(tuning)) = (
        world.get::<TuMax>(shooter).copied(),
        world.get::<Aiming>(shooter).copied(),
        world.get_resource::<CombatTuning>(),
    ) else {
        unreachable!("the shooter carries TuMax + Aiming and the app carries CombatTuning");
    };
    *mode_tu_cost(&mode, &tu_max, &aiming, tuning)
}
