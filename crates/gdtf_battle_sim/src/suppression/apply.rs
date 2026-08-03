//! Mark enemies in the fire impact radius as suppressed.

use bevy::prelude::{
    Commands, Deref, Entity, Message, MessageReader, MessageWriter, Query, Res, With,
};

use crate::{
    acts::FireRequested,
    fire::{MeleeQuery, MountedQuery, WieldsQuery},
    ganger::{Faction, Position, Suppressed, SuppressorCell},
    metric::{Cell, CellLevel, Level},
    tuning::{CombatTuning, SuppressionRadius},
    weapon::{Silenced, shooter_weapon_silenced},
};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
struct WithinSuppressionRadius(bool);

impl WithinSuppressionRadius {
    const fn new(within: bool) -> Self {
        Self(within)
    }
}

/// First-time suppression of a ganger this tick.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SuppressionApplied {
    /// Who was suppressed.
    pub ganger: Entity,
    /// Where they stood.
    pub at: CellLevel,
}

impl SuppressionApplied {
    /// Build a suppression-applied message.
    #[must_use]
    pub const fn new(ganger: Entity, at: CellLevel) -> Self {
        Self { ganger, at }
    }
}

fn within_radius(
    cell: Cell,
    level: Level,
    target_cell: Cell,
    target_level: Level,
    radius: SuppressionRadius,
) -> WithinSuppressionRadius {
    if *level != *target_level {
        return WithinSuppressionRadius::new(false);
    }
    let dx = (cell.x - target_cell.x).unsigned_abs();
    let dy = (cell.y - target_cell.y).unsigned_abs();
    WithinSuppressionRadius::new(dx.max(dy) <= u32::from(*radius))
}

/// On [`FireRequested`], suppress enemies near the impact (unless the weapon is silenced).
#[expect(
    clippy::too_many_arguments,
    reason = "the producer reads the FireRequested buffer, the shooter position/faction \
              query, the candidate-ganger query, the GTW-542/674 silenced-weapon resolution (the \
              wield + mounted-probe + melee-probe + Silenced-marker queries), the \
              Option<CombatTuning>, and the Commands + SuppressionApplied writer; each is a \
              distinct Bevy SystemParam (mirroring dispatch_fire's own carve-out) — bundling \
              would only hide the reads"
)]
pub fn apply_suppression(
    mut fires: MessageReader<FireRequested>,
    positions: Query<(&Position, &Faction)>,
    gangers: Query<(Entity, &Position, &Faction, Option<&Suppressed>)>,
    wields: WieldsQuery,
    mounted: MountedQuery,
    melee: MeleeQuery,
    silenced: Query<(), With<Silenced>>,
    tuning: Option<Res<CombatTuning>>,
    mut commands: Commands,
    mut applied: MessageWriter<SuppressionApplied>,
) {
    let Some(tuning) = tuning else {
        return;
    };
    let radius = tuning.reaction.suppression_radius;

    let mut suppressed_this_tick: bevy::platform::collections::HashSet<Entity> =
        bevy::platform::collections::HashSet::default();

    for fire in fires.read() {
        if *shooter_weapon_silenced(fire.shooter, &wields, &mounted, &melee, &silenced) {
            continue;
        }
        let Ok((shooter_position, shooter_faction)) = positions.get(fire.shooter) else {
            continue;
        };
        let suppressor = SuppressorCell::new(**shooter_position);

        for (entity, position, faction, already) in &gangers {
            if *faction == *shooter_faction || entity == fire.shooter {
                continue;
            }
            if !*within_radius(
                position.cell(),
                position.level(),
                fire.target_cell,
                fire.target_level,
                radius,
            ) {
                continue;
            }

            let is_fresh = already.is_none() && !suppressed_this_tick.contains(&entity);
            commands.entity(entity).insert(Suppressed::new(suppressor));
            if is_fresh {
                applied.write(SuppressionApplied::new(entity, **position));
                suppressed_this_tick.insert(entity);
            }
        }
    }
}
