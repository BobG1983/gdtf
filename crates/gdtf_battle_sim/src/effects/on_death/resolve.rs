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
    terrain::entity::TerrainIndexKey,
    weapon::{MeleeWeapon, MountedWeapon, Wields},
};

/// Terrain on-death effects keyed by the terrain piece that carries them.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct TerrainOnDeathRegistry(HashMap<TerrainIndexKey, Vec<OnDeathEffect>>);

impl TerrainOnDeathRegistry {
    /// From key/effect-list pairs.
    #[must_use]
    pub fn new(effects: impl IntoIterator<Item = (TerrainIndexKey, Vec<OnDeathEffect>)>) -> Self {
        Self(effects.into_iter().collect())
    }

    /// Insert or replace a key's effects.
    pub fn insert(
        &mut self,
        at: TerrainIndexKey,
        effects: Vec<OnDeathEffect>,
    ) -> Option<Vec<OnDeathEffect>> {
        self.0.insert(at, effects)
    }

    /// Look up a key's effects, in the authored order.
    #[must_use]
    pub fn effects(&self, at: &TerrainIndexKey) -> Option<&[OnDeathEffect]> {
        self.0.get(at).map(Vec::as_slice)
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
    // The on-death effects carried by the victim's firing weapon, in the authored order.
    fn effect_of(&self, victim: Entity) -> Option<Vec<OnDeathEffect>> {
        let weapon = self.wields.get(victim).ok()?.firing_weapon(
            |entity| self.mounted.get(entity).is_ok(),
            |entity| self.melee.get(entity).is_ok(),
        )?;
        self.on_deaths
            .get(weapon)
            .ok()
            .map(|on_death| on_death.effects().to_vec())
    }
}

/// The field ledger a death effect spawns into, and the catalog it reads.
#[derive(SystemParam)]
pub struct FieldSpawning<'w> {
    placed: ResMut<'w, FieldRegistry>,
    defs:   Option<Res<'w, FieldDefRegistry>>,
}

/// One death already fanned: a terrain piece by its key, a body by its cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DeathVisit {
    /// A terrain death, told apart by which piece died.
    Terrain(TerrainIndexKey),
    /// An entity death, told apart by where it fell.
    Cell(CellLevel),
}

/// Fan each buffered death through its weapon or terrain on-death effect.
pub fn resolve_on_death(
    mut deaths: MessageReader<OnDeathOccurred>,
    weapons: DeathWeapons,
    mut victims: Query<VictimRow>,
    grid: Res<OccupancyGrid>,
    mut fields: FieldSpawning,
    terrain_on_death: Res<TerrainOnDeathRegistry>,
) {
    let mut queue: Vec<OnDeathOccurred> = deaths.read().copied().collect();
    let mut visited: HashSet<DeathVisit> = HashSet::new();

    while let Some(death) = queue.pop() {
        let visit = death
            .terrain
            .map_or(DeathVisit::Cell(death.at), DeathVisit::Terrain);
        if !visited.insert(visit) {
            continue;
        }

        let effects: Option<Vec<OnDeathEffect>> = match death.terrain {
            Some(key) => terrain_on_death.effects(&key).map(<[_]>::to_vec),
            None => weapons.effect_of(death.entity),
        };
        let Some(effects) = effects else {
            continue;
        };

        for effect in &effects {
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
}
