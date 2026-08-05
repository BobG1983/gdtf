//! Resolve buffered deaths and fan their authored effects.

use bevy::{
    ecs::system::SystemParam,
    platform::collections::{HashMap, HashSet},
    prelude::{Entity, MessageReader, Query, Res, ResMut, Resource, With},
};

use super::{OnDeath, OnDeathEffect, OnDeathOccurred};
use crate::{
    effects::{
        fields::{FieldDefRegistry, FieldRegistry},
        on_death::{ApplyOnDeathEffect, DeathFanOut, VictimRow},
    },
    metric::CellLevel,
    occupancy::OccupancyGrid,
    weapon::{MeleeWeapon, MountedWeapon, Wields},
};

/// Cover-cell on-death effects keyed by cell.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct CoverOnDeathRegistry(HashMap<CellLevel, OnDeathEffect>);

impl CoverOnDeathRegistry {
    /// From cell/effect pairs.
    #[must_use]
    pub fn new(effects: impl IntoIterator<Item = (CellLevel, OnDeathEffect)>) -> Self {
        Self(effects.into_iter().collect())
    }

    /// Insert or replace an effect.
    pub fn insert(&mut self, at: CellLevel, effect: OnDeathEffect) -> Option<OnDeathEffect> {
        self.0.insert(at, effect)
    }

    /// Look up an effect.
    #[must_use]
    pub fn effect(&self, at: &CellLevel) -> Option<&OnDeathEffect> {
        self.0.get(at)
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The weapon a victim carried, and the effect it fires when they die.
#[derive(SystemParam)]
pub struct DeathWeapons<'w, 's> {
    wields:    Query<'w, 's, &'static Wields>,
    on_deaths: Query<'w, 's, &'static OnDeath>,
    melee:     Query<'w, 's, (), With<MeleeWeapon>>,
    mounted:   Query<'w, 's, (), With<MountedWeapon>>,
}

impl DeathWeapons<'_, '_> {
    // The on-death effect carried by the victim's firing weapon, if any.
    fn effect_of(&self, victim: Entity) -> Option<OnDeathEffect> {
        let weapon = self.wields.get(victim).ok()?.firing_weapon(
            |entity| self.mounted.get(entity).is_ok(),
            |entity| self.melee.get(entity).is_ok(),
        )?;
        self.on_deaths
            .get(weapon)
            .ok()
            .map(|on_death| on_death.effect().clone())
    }
}

/// The field ledger a death effect spawns into, and the catalog it reads.
#[derive(SystemParam)]
pub struct FieldSpawning<'w> {
    placed: ResMut<'w, FieldRegistry>,
    defs:   Option<Res<'w, FieldDefRegistry>>,
}

/// Fan each buffered death through its weapon or cover on-death effect.
pub fn resolve_on_death(
    mut deaths: MessageReader<OnDeathOccurred>,
    weapons: DeathWeapons,
    mut victims: Query<VictimRow>,
    grid: Res<OccupancyGrid>,
    mut fields: FieldSpawning,
    cover_on_death: Res<CoverOnDeathRegistry>,
) {
    let mut queue: Vec<OnDeathOccurred> = deaths.read().copied().collect();
    let mut visited: HashSet<CellLevel> = HashSet::new();

    while let Some(death) = queue.pop() {
        if !visited.insert(death.at) {
            continue;
        }

        let effect: Option<OnDeathEffect> = if death.entity == Entity::PLACEHOLDER {
            cover_on_death.effect(&death.at).cloned()
        } else {
            weapons.effect_of(death.entity)
        };
        let Some(effect) = effect else {
            continue;
        };

        let mut fan_out = DeathFanOut {
            grid:       &grid,
            victims:    &mut victims,
            fields:     &mut fields.placed,
            field_defs: fields.defs.as_deref(),
            cascade:    &mut queue,
        };
        effect.fan_at(death.at, &mut fan_out);
    }
}
