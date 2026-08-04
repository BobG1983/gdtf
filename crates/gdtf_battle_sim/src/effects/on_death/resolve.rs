//! Resolve buffered deaths and fan their authored effects.

use bevy::{
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

/// Fan each buffered death through its weapon or cover on-death effect.
#[expect(
    clippy::too_many_arguments,
    reason = "the resolver threads the death reader, the three disjoint weapon-resolution \
              queries (wields / melee / mounted markers) + the OnDeath read + the victim-surface \
              query, and the four battle-lifetime world resources (occupancy / field registry / \
              field catalog / cover-on-death registry) — each a distinct Bevy SystemParam (the \
              dispatch_fire / fold_ganger_round argument-count carve-out); bundling would only \
              hide the access set"
)]
pub fn resolve_on_death(
    mut deaths: MessageReader<OnDeathOccurred>,
    wields: Query<&Wields>,
    on_deaths: Query<&OnDeath>,
    melee: Query<(), With<MeleeWeapon>>,
    mounted: Query<(), With<MountedWeapon>>,
    mut victims: Query<VictimRow>,
    grid: Res<OccupancyGrid>,
    mut fields: ResMut<FieldRegistry>,
    field_defs: Option<Res<FieldDefRegistry>>,
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
            wields
                .get(death.entity)
                .ok()
                .and_then(|w| w.firing_weapon(|e| mounted.get(e).is_ok(), |e| melee.get(e).is_ok()))
                .and_then(|weapon_entity| on_deaths.get(weapon_entity).ok())
                .map(|on_death| on_death.effect().clone())
        };
        let Some(effect) = effect else {
            continue;
        };

        let mut fan_out = DeathFanOut {
            grid:       &grid,
            victims:    &mut victims,
            fields:     &mut fields,
            field_defs: field_defs.as_deref(),
            cascade:    &mut queue,
        };
        effect.fan_at(death.at, &mut fan_out);
    }
}
