//! The **Explode** on-death effect (GTW-547; GTW-552 one-file-per-effect) — its
//! [`ExplodeDamage`] payload and the isolated [`ApplyExplode`] behaviour that fans a
//! GTW-541 `AoE` blast at the death cell: a flat, deterministic, armor-bypassing,
//! RNG-free [`Hp`] drain per ganger in the radius, cascading on a kill.

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

/// The **flat per-cell HP damage** an [`Explode`](super::OnDeathEffect::Explode) blast
/// deals to each ganger in its fan-out radius (GTW-547).
///
/// A blast damage NUMBER (a small count, `u16` to match the [`Hp`] inner). A no-bare-types
/// newtype: private inner + derived [`Deref`]; `#[serde(transparent)]` so an authored
/// [`Explode`](super::OnDeathEffect::Explode) `.ron` names it as a bare integer. Distinct
/// from [`WeaponDamage`](crate::weapon::WeaponDamage) (a signed-`i32` spawn sentinel that
/// runs the full §5/§6 armor+wound fold) — an on-death blast is a DETERMINISTIC,
/// armor-bypassing, RNG-free direct drain (the [`DotDamage`](crate::weapon::DotDamage) /
/// [`FieldDamage`](crate::effects::fields::FieldDamage) precedent), so its magnitude is its OWN
/// positive quantity, keyed to the exploding source.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize, Serialize)]
#[serde(transparent)]
pub struct ExplodeDamage(u16);

impl ExplodeDamage {
    /// Build a per-cell blast damage from its count.
    #[must_use]
    pub const fn new(damage: u16) -> Self {
        Self(damage)
    }
}

/// **Explode** — fan a GTW-541 [`aoe_affected`] blast at the death cell (GTW-547;
/// isolated per GTW-552).
///
/// Enumerates the `AoE` template's cell set (centred at the death cell, which is also its
/// notional shooter origin so a cone degenerates to the full disc — a corpse has no fire
/// direction), reads each cell's occupant off the fan-out surface's
/// [`grid`](DeathFanOut::grid), and drains a flat, deterministic [`ExplodeDamage`] from
/// each LIVE ganger's [`Hp`] (`saturating_sub`, armor-bypassing, NO RNG — the
/// [`tick_dot`](crate::effects::dot::tick_dot) / [`tick_fields`](crate::effects::fields::tick_fields)
/// direct-drain model). The variant's authored
/// [`damage_type`](super::OnDeathEffect::Explode) is presentation-only flavour (the drain
/// BYPASSES the armor matchup, the field-tick precedent), so it is not part of this
/// behaviour.
pub struct ApplyExplode {
    /// The `AoE` template shape (GTW-541) the blast fans.
    hit_type: HitType,
    /// The flat per-cell HP the blast drains from each ganger in the radius.
    damage:   ExplodeDamage,
}

impl ApplyExplode {
    /// Build the explode effect from its `AoE` template + per-cell drain.
    #[must_use]
    pub const fn new(hit_type: HitType, damage: ExplodeDamage) -> Self {
        Self { hit_type, damage }
    }
}

impl ApplyOnDeathEffect for ApplyExplode {
    /// Fan the blast: drain every LIVE occupant in the template's radius; a blast that
    /// empties a victim's [`Hp`] flips it to [`LifeState::Dead`] and pushes the fresh
    /// [`OnDeathOccurred`] onto the surface's [`cascade`](DeathFanOut::cascade) work-queue
    /// (the same-frame cascade — the resolver's fixpoint loop processes it, guarded by its
    /// visited-set). A corpse in the radius is skipped (already Dead).
    ///
    /// Faction-BLIND (the GTW-541 friendly-fire property, `docs/combat/resolution.md` §2):
    /// the blast strikes EVERY occupant in the radius, including allies. Deterministic —
    /// [`aoe_affected`] returns a canonically-sorted set and the drain takes no RNG, so a
    /// demo explosion is byte-stable.
    fn fan_at(&self, at: CellLevel, fan_out: &mut DeathFanOut<'_, '_, '_>) {
        // The blast centre is BOTH the impact and the notional shooter (a corpse has no
        // fire direction), so a Cone falls back to the full disc — the documented
        // degenerate fallback.
        for cell in aoe_affected(at, self.hit_type, at) {
            let Some(occupant) = fan_out.grid.occupant(&cell) else {
                continue;
            };
            let Ok((mut hp, mut life)) = fan_out.victims.get_mut(occupant) else {
                continue;
            };
            // Skip a corpse (already Dead) — it neither takes damage nor re-fans (the
            // once-only property the resolver's visited-set also enforces at the
            // death-cell level).
            if *life == LifeState::Dead {
                continue;
            }
            // Flat, armor-bypassing, RNG-free drain (saturating at 0 — Hp is unsigned).
            *hp = Hp::new(hp.saturating_sub(*self.damage));
            // A blast that empties Hp KILLS (the DOT / field-kill precedent) and pushes
            // the cascade death onto the resolver's work-queue (same-frame,
            // visited-set-guarded).
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

    /// A ground-floor `(cell, level)` key at `(x, y)`.
    fn ground(x: i32, y: i32) -> CellLevel {
        CellLevel::new(Cell::new(x, y), Level::new(0))
    }

    /// Run `effect.fan_at(at, …)` against a world holding `victims`, returning the cascade
    /// deaths the fan pushed. Bare-`World` + `SystemState` is the pure-sim
    /// unit-test idiom (`bevy-traps.md` #7 carve-out (b)) — the resolver's end-to-end path
    /// is proven by the `effects/on_death` suite.
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

    /// A non-lethal blast drains its flat damage from a live occupant in the radius and
    /// leaves it alive — no cascade work item is pushed.
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

    /// A LETHAL blast empties the victim's Hp, flips it Dead, and pushes the fresh death
    /// onto the cascade work-queue as a typed `OnDeathOccurred` at the victim's cell.
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

    /// A corpse in the radius is SKIPPED — its Hp is untouched and it does not re-fan
    /// (the once-only cascade property).
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
