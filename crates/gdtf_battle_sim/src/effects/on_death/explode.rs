//! On-death explosion effect.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use super::{ApplyOnDeathEffect, DeathFanOut};
use crate::{
    effects::on_death::OnDeathOccurred,
    ganger::{Hp, LifeState},
    metric::CellLevel,
    shot_pipeline::aoe::aoe_affected,
    weapon::HitType,
};

/// Flat damage dealt by an on-death explosion.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize, Serialize)]
#[serde(transparent)]
pub struct ExplodeDamage(u16);

impl ExplodeDamage {
    /// Wrap a damage amount.
    #[must_use]
    pub const fn new(damage: u16) -> Self {
        Self(damage)
    }
}

/// Fans blast damage to occupants in the hit area.
pub struct ApplyExplode {
    hit_type: HitType,
    damage:   ExplodeDamage,
}

impl ApplyExplode {
    /// Build the applicator.
    #[must_use]
    pub const fn new(hit_type: HitType, damage: ExplodeDamage) -> Self {
        Self { hit_type, damage }
    }
}

impl ApplyOnDeathEffect for ApplyExplode {
    fn fan_at(&self, at: CellLevel, fan_out: &mut DeathFanOut<'_, '_, '_>) {
        for cell in aoe_affected(at, self.hit_type, at) {
            let Some(occupant) = fan_out.grid.occupant(&cell) else {
                continue;
            };
            let Ok((mut hp, mut life)) = fan_out.victims.get_mut(occupant) else {
                continue;
            };
            if *life == LifeState::Dead {
                continue;
            }
            *hp = Hp::new(hp.saturating_sub(*self.damage));
            if *hp == Hp::new(0) {
                *life = LifeState::Dead;
                fan_out.cascade.push(OnDeathOccurred::new(occupant, cell));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::{
        ecs::system::SystemState,
        prelude::{Query, World},
    };

    use super::{ApplyExplode, ApplyOnDeathEffect, ExplodeDamage};
    use crate::{
        effects::{
            fields::FieldRegistry,
            on_death::{DeathFanOut, OnDeathOccurred, VictimRow},
        },
        ganger::{Hp, LifeState},
        metric::{Cell, CellLevel, Level},
        occupancy::OccupancyGrid,
        weapon::{BlastRadius, HitType},
    };

    fn ground(x: i32, y: i32) -> CellLevel {
        CellLevel::new(Cell::new(x, y), Level::new(0))
    }

    fn fan(
        world: &mut World,
        grid: &OccupancyGrid,
        effect: &ApplyExplode,
        at: CellLevel,
    ) -> Vec<OnDeathOccurred> {
        let mut fields = FieldRegistry::new();
        let mut cascade = Vec::new();
        let mut state: SystemState<Query<VictimRow>> = SystemState::new(world);
        let Ok(mut victims) = state.get_mut(world) else {
            unreachable!("a plain Query SystemParam always validates");
        };
        let mut fan_out = DeathFanOut {
            grid,
            victims: &mut victims,
            fields: &mut fields,
            field_defs: None,
            cascade: &mut cascade,
        };
        effect.fan_at(at, &mut fan_out);
        cascade
    }

    #[test]
    fn non_lethal_blast_drains_without_killing() {
        let mut world = World::new();
        let victim = world.spawn((Hp::new(10), LifeState::Alive)).id();
        let mut grid = OccupancyGrid::new();
        grid.set_occupant(ground(6, 5), Some(victim));

        let effect = ApplyExplode::new(
            HitType::Blast {
                radius: BlastRadius::new(1),
            },
            ExplodeDamage::new(5),
        );
        let cascade = fan(&mut world, &grid, &effect, ground(5, 5));

        assert_eq!(
            world.get::<Hp>(victim).map(|h| **h),
            Some(5),
            "the adjacent victim took the blast's flat 5 HP"
        );
        assert_eq!(
            world.get::<LifeState>(victim).copied(),
            Some(LifeState::Alive),
            "a non-lethal blast leaves the victim alive"
        );
        assert!(cascade.is_empty(), "a non-lethal fan pushes no cascade");
    }

    #[test]
    fn lethal_blast_kills_and_pushes_the_cascade_work_item() {
        let mut world = World::new();
        let victim = world.spawn((Hp::new(3), LifeState::Alive)).id();
        let mut grid = OccupancyGrid::new();
        grid.set_occupant(ground(6, 5), Some(victim));

        let effect = ApplyExplode::new(
            HitType::Blast {
                radius: BlastRadius::new(1),
            },
            ExplodeDamage::new(100),
        );
        let cascade = fan(&mut world, &grid, &effect, ground(5, 5));

        assert_eq!(
            world.get::<LifeState>(victim).copied(),
            Some(LifeState::Dead),
            "a lethal blast kills the victim"
        );
        assert_eq!(
            cascade,
            vec![OnDeathOccurred::new(victim, ground(6, 5))],
            "the kill pushes the victim's death onto the cascade queue at its cell"
        );
    }

    #[test]
    fn a_corpse_in_the_radius_is_skipped() {
        let mut world = World::new();
        let corpse = world.spawn((Hp::new(0), LifeState::Dead)).id();
        let mut grid = OccupancyGrid::new();
        grid.set_occupant(ground(6, 5), Some(corpse));

        let effect = ApplyExplode::new(
            HitType::Blast {
                radius: BlastRadius::new(1),
            },
            ExplodeDamage::new(100),
        );
        let cascade = fan(&mut world, &grid, &effect, ground(5, 5));

        assert!(
            cascade.is_empty(),
            "a corpse neither takes damage nor re-fans the cascade"
        );
    }
}
