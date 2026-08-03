//! System: movers and declarers can be interrupted by enemy reactors in LOS.

use bevy::{
    platform::collections::HashSet,
    prelude::{Changed, Entity, MessageReader, Query, Res, ResMut, With},
};

use super::{
    declared::InterruptSignals,
    interrupt::try_reaction,
    ledger::PendingSpendLedger,
    snapshot::{ReactionGangers, ReactionRow, cell_order},
};
use crate::{
    acts::{FireDeclaration, WeaponProbes},
    cover::CoverLedger,
    fire::WieldsQuery,
    ganger::{LifeState, Position, Suppressed},
    magazine::Magazine,
    occupancy::OccupancyGrid,
    rng::ReactionRng,
    surface::SurfaceGrid,
    tuning::{CombatTuning, ReactionsUsed},
    weapon::{FireMode, Handedness, Silenced, shooter_weapon_silenced},
};

/// On position change or loud fire declaration, try enemy opportunity shots.
#[expect(
    clippy::too_many_arguments,
    reason = "the trigger reads the two act-in-LOS surfaces (Changed<Position> movers + the \
              FireDeclaration buffer), the full ganger snapshot, the wielded-weapon + \
              weapon-entity queries (to resolve the reactor's single-shot spec exactly as \
              dispatch_fire does), the shared weapon-marker probe bundle (melee + mounted, \
              GTW-660) + the GTW-526 suppressed-marker probe, the mutable ReactionsUsed \
              counter, the four read grids + tuning the can_see/can_fire/can_engage gates \
              need, the seeded ReactionRng, the GTW-542 silenced-weapon probe, and the two \
              act MessageWriters; each is a distinct Bevy SystemParam, mirroring \
              dispatch_fire's own argument-count carve-out — bundling would only hide the \
              reads"
)]
pub fn reaction_trigger(
    moved: Query<Entity, Changed<Position>>,
    mut declarations: MessageReader<FireDeclaration>,
    gangers: ReactionGangers,
    wields: WieldsQuery,
    weapons: Query<(&Magazine, &FireMode, &Handedness)>,
    probes: WeaponProbes,
    suppressed: Query<(), With<Suppressed>>,
    silenced: Query<(), With<Silenced>>,
    mut used: Query<&mut ReactionsUsed>,
    tuning: Res<CombatTuning>,
    occupancy: Res<OccupancyGrid>,
    surface: Res<SurfaceGrid>,
    cover: Res<CoverLedger>,
    rng: Option<ResMut<ReactionRng>>,
    mut signals: InterruptSignals,
) {
    let Some(mut rng) = rng else {
        return;
    };

    let rows: Vec<ReactionRow> = gangers
        .iter()
        .map(
            |(entity, position, stance, facing, aiming, life, tu, tu_max, faction, reactions)| {
                ReactionRow {
                    entity,
                    position: *position,
                    stance: *stance,
                    facing: *facing,
                    aiming: *aiming,
                    life: *life,
                    tu: *tu,
                    tu_max: *tu_max,
                    faction: *faction,
                    reactions: *reactions,
                }
            },
        )
        .collect();

    let mut actors: HashSet<Entity> = moved.iter().collect();
    for declaration in declarations.read() {
        if *shooter_weapon_silenced(
            declaration.shooter,
            &wields,
            &probes.mounted,
            &probes.melee,
            &silenced,
        ) {
            continue;
        }
        actors.insert(declaration.shooter);
    }
    if actors.is_empty() {
        return;
    }

    let is_dead_fn = |entity: Entity| {
        rows.iter()
            .find(|row| row.entity == entity)
            .is_some_and(|row| row.life == LifeState::Dead)
    };
    let is_dead = &is_dead_fn;

    let mut acting_rows: Vec<ReactionRow> = rows
        .iter()
        .copied()
        .filter(|row| actors.contains(&row.entity))
        .collect();
    acting_rows.sort_by_key(|row| cell_order(&row.position));

    let mut ledger = PendingSpendLedger::default();

    for actor in &acting_rows {
        if !*actor.life.is_active() {
            continue;
        }
        let mut reactors: Vec<ReactionRow> = rows
            .iter()
            .copied()
            .filter(|row| row.faction != actor.faction && row.entity != actor.entity)
            .collect();
        reactors.sort_by_key(|row| cell_order(&row.position));
        for reactor in &reactors {
            if let Some(commit) = try_reaction(
                actor,
                reactor,
                &ledger,
                &wields,
                &weapons,
                &probes,
                &suppressed,
                &mut used,
                &tuning,
                &occupancy,
                &surface,
                &cover,
                &mut rng,
                is_dead,
                &mut signals,
            ) {
                ledger.commit(commit);
            }
        }
    }
}
