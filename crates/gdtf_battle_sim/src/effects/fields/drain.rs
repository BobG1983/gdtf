//! The **per-turn flat drain** field consequence (GTW-545; GTW-553 one-file-per-consequence)
//! — the [`FieldDamage`] payload newtype and the isolated [`ApplyDrain`] behaviour that eats
//! a flat amount of the occupant's [`Hp`] each round, signals the tick, and KILLS on an
//! emptied pool (the GTW-544 DOT-kills precedent).

use bevy::prelude::{Deref, Entity};
use serde::Deserialize;

use super::{ApplyFieldEffect, OccupantDrain};
use crate::{
    effects::{fields::FieldTicked, on_death::OnDeathOccurred},
    ganger::{Hp, LifeState},
    metric::CellLevel,
};

/// The **per-turn HP damage** an area-damage field deals each turn a ganger stands in it —
/// the flat amount the field eats from the occupant's [`Hp`] pool every
/// turn, bypassing armor entirely (gated only by whole-source immunity, never a matchup).
///
/// A field damage NUMBER (a small per-turn count, `u16` to match the [`Hp`]
/// inner). A no-bare-types newtype: private inner + derived [`Deref`];
/// `#[serde(transparent)]` so a field's authored [`FieldDef`](crate::effects::fields::FieldDef)
/// `.ron` names it as a bare integer. Distinct from [`crate::weapon::DotDamage`] (the DOT
/// per-turn tick) — a field's per-turn drain is its OWN quantity, keyed to the field type,
/// never a weapon's.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct FieldDamage(u16);

impl FieldDamage {
    /// Build a per-turn field damage from its count.
    #[must_use]
    pub const fn new(damage: u16) -> Self {
        Self(damage)
    }
}

/// **Drain** — the per-turn flat HP drain field consequence (GTW-545; isolated per
/// GTW-553).
///
/// Each round the occupant stands on the field (and no consequence exempts it), this
/// behaviour subtracts the field's [`FieldDamage`] from its [`Hp`] DIRECTLY — no armor
/// matchup, no injury roll, no RNG (the deterministic field tick) — emits one
/// [`FieldTicked`] signal, and applies the terminal gate: a drain that EMPTIES the pool
/// flips the occupant to [`LifeState::Dead`] and emits its
/// [`OnDeathOccurred`] at the field cell (the GTW-544 DOT-kills precedent: a
/// persistent-zone drain that brings HP to `0` is lethal, NOT a down).
pub struct ApplyDrain {
    /// The flat per-turn HP this consequence eats (bypasses the matchup wheel).
    damage: FieldDamage,
}

impl ApplyDrain {
    /// Build the drain behaviour from its authored per-turn damage.
    #[must_use]
    pub const fn new(damage: FieldDamage) -> Self {
        Self { damage }
    }
}

impl ApplyFieldEffect for ApplyDrain {
    /// Drain the occupant a flat [`FieldDamage`] (`saturating_sub`, floors at `0` — no
    /// underflow), emit one [`FieldTicked`], and KILL on an emptied pool (flip to
    /// [`LifeState::Dead`] + emit the [`OnDeathOccurred`] terminal-death signal at the
    /// field cell, so the GTW-547 resolver fans the dead ganger's on-death effect).
    ///
    /// Writes through the surface's [`Mut`](bevy::prelude::Mut) wrappers ONLY on an
    /// actual mutation, so `Changed<`[`LifeState`]`>` fires on the lethal tick alone —
    /// the pre-palette change-detection timing exactly.
    fn drain_occupant(
        &self,
        at: CellLevel,
        occupant: Entity,
        drain: &mut OccupantDrain<'_, '_, '_, '_, '_>,
    ) {
        **drain.hp = Hp::new(drain.hp.saturating_sub(*self.damage));
        drain
            .ticks
            .write(FieldTicked::new(occupant, at, self.damage));
        if **drain.hp == Hp::new(0) {
            **drain.life = LifeState::Dead;
            drain.deaths.write(OnDeathOccurred::new(occupant, at));
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::{
        ecs::system::SystemState,
        prelude::{Entity, MessageWriter, Messages, Query, World},
    };

    use super::{ApplyDrain, FieldDamage};
    use crate::{
        effects::{
            fields::{ApplyFieldEffect, FieldTicked, OccupantDrain},
            on_death::OnDeathOccurred,
        },
        ganger::{Hp, LifeState},
        metric::{Cell, CellLevel, Level},
    };

    /// A ground-floor `(cell, level)` key at `(x, y)`.
    fn ground(x: i32, y: i32) -> CellLevel {
        CellLevel::new(Cell::new(x, y), Level::new(0))
    }

    /// The drain harness's param state — one victim-vitals query + the two signal
    /// writers (a `type` so the tuple stays under clippy's type-complexity gate).
    type DrainParams = SystemState<(
        Query<'static, 'static, (&'static mut Hp, &'static mut LifeState)>,
        MessageWriter<'static, FieldTicked>,
        MessageWriter<'static, OnDeathOccurred>,
    )>;

    /// Run `ApplyDrain(damage).drain_occupant` once against a fresh occupant at
    /// `start_hp`, returning `(hp, life, ticks, deaths)` after the drain. Bare-`World` +
    /// `SystemState` is the pure-sim unit-test idiom (`bevy-traps.md` #7
    /// carve-out (b)); the message buffers are world resources the writers validate
    /// against.
    fn drained(damage: u16, start_hp: u16) -> (u16, LifeState, usize, usize) {
        let mut world = World::new();
        world.init_resource::<Messages<FieldTicked>>();
        world.init_resource::<Messages<OnDeathOccurred>>();
        let occupant = world.spawn((Hp::new(start_hp), LifeState::Alive)).id();
        let mut state: DrainParams = SystemState::new(&mut world);
        run_drain(&mut state, &mut world, occupant, damage);
        let hp = world.get::<Hp>(occupant).map_or(0, |h| **h);
        let life = world
            .get::<LifeState>(occupant)
            .copied()
            .unwrap_or(LifeState::Alive);
        let ticks = world.resource::<Messages<FieldTicked>>().len();
        let deaths = world.resource::<Messages<OnDeathOccurred>>().len();
        (hp, life, ticks, deaths)
    }

    /// Borrow the params out of `state` and apply one drain to `occupant`.
    fn run_drain(state: &mut DrainParams, world: &mut World, occupant: Entity, damage: u16) {
        let Ok((mut occupants, mut ticks, mut deaths)) = state.get_mut(world) else {
            unreachable!("the params validate: both message buffers are initialized");
        };
        let Ok((mut hp, mut life)) = occupants.get_mut(occupant) else {
            unreachable!("the occupant was just spawned with Hp + LifeState");
        };
        let mut drain = OccupantDrain {
            hp:     &mut hp,
            life:   &mut life,
            ticks:  &mut ticks,
            deaths: &mut deaths,
        };
        ApplyDrain::new(FieldDamage::new(damage)).drain_occupant(
            ground(4, 4),
            occupant,
            &mut drain,
        );
    }

    /// A non-lethal drain eats exactly the flat amount, leaves the occupant Alive, and
    /// signals ONE tick and NO death.
    #[test]
    fn a_non_lethal_drain_eats_the_flat_amount_and_signals_one_tick() {
        let (hp, life, ticks, deaths) = drained(3, 10);
        assert_eq!(hp, 7, "the drain is a flat direct subtraction");
        assert_eq!(life, LifeState::Alive, "a non-lethal drain leaves Alive");
        assert_eq!(ticks, 1, "one FieldTicked per draining round");
        assert_eq!(deaths, 0, "a non-lethal drain emits no death signal");
    }

    /// A lethal drain floors HP at `0` (saturating), flips the occupant to Dead, and
    /// emits BOTH the tick and the terminal-death signal.
    #[test]
    fn a_lethal_drain_floors_at_zero_kills_and_signals_the_death() {
        let (hp, life, ticks, deaths) = drained(10, 8);
        assert_eq!(hp, 0, "the drain saturates — no underflow");
        assert_eq!(
            life,
            LifeState::Dead,
            "an emptied pool KILLS (DOT precedent)"
        );
        assert_eq!(ticks, 1, "the lethal tick still signals FieldTicked");
        assert_eq!(deaths, 1, "the lethal tick emits one OnDeathOccurred");
    }
}
