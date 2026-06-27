//! The **enemy-turn act cadence** (GTW-461) — the tick-count dwell that paces the enemy
//! brain to AT MOST ONE act per cadence-step, so the enemy turn resolves act-by-act on
//! screen instead of as a one-frame volley.
//!
//! ## Why this exists
//!
//! [`enemy_ai_turn`](crate::ai::enemy_ai_turn) is stateless across frames and, before this
//! slice, emitted a [`FireRequested`](crate::acts::FireRequested) for EVERY conscious enemy
//! EVERY [`Update`](bevy::prelude::Update) tick. Because the brain is ordered
//! `.before(`[`dispatch_fire`](crate::acts::dispatch_fire)`)`, each fire resolved
//! SYNCHRONOUSLY the same frame — so the whole enemy volley landed within a handful of
//! frames, and every presenter view reacting to `Changed<T>` saw the entire turn mutate at
//! once (the inspect panel / combat log / floating-combat-text snapped to the final
//! post-turn state).
//!
//! The fix is SIM-SIDE cadence (ADR-0001 — the presenter stays a one-way mirror that reads
//! the sim, never gates it): mirror the EXISTING one-cell-per-tick pacing of
//! [`advance_walk`](crate::move_acts::advance_walk) for the FIRE/turn path. The brain now
//! emits one enemy act, then waits [`ActCadence`] ticks before the next — so each downstream
//! view paces FOR FREE off the per-act sim deltas it already watches, with no presenter
//! refactor.
//!
//! ## Why a TICK COUNT, not wall-clock time
//!
//! Pacing by tick count (not [`Time`](bevy::prelude::Time)) keeps the cadence deterministic:
//! a headless test advances exactly N ticks per act and asserts the one-per-step emission,
//! while the running app at ~60fps shows a natural beat between enemy acts. This is the same
//! choice `advance_walk` made (one discrete step per tick), so the two pacing models match.
//!
//! ## Lifetime
//!
//! [`EnemyActCooldown`] is a battle-lifetime [`Resource`](bevy::prelude::Resource): the
//! setup system inserts it on the same successful-setup `Ok` path that inserts
//! [`BattleInProgress`](crate::battle::BattleInProgress), and the teardown system removes it
//! alongside — so it is present for exactly the battle-active window (the established
//! battle-lifetime pattern; `bevy-traps.md` #1). The brain reads it as `Option<ResMut<_>>`
//! so its access stays panic-free outside a live battle.

use bevy::{
    ecs::system::SystemParam,
    prelude::{Deref, Res, ResMut, Resource},
};

/// The **enemy-act cadence** — the number of [`Update`](bevy::prelude::Update) ticks the
/// brain waits between successive enemy acts (GTW-461).
///
/// A named newtype over the tick count (no-bare-types: it carries a domain value — the
/// inter-act dwell, measured in ticks — so it is a real named newtype, NOT a bare `u32`).
/// The derived [`Deref`] reads the inner count back; the inner field is PRIVATE, set only
/// through [`ActCadence::new`].
#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActCadence(u32);

impl ActCadence {
    /// The shipped default cadence — `30` ticks (~0.5s at 60fps).
    ///
    /// Chosen to be slightly LONGER than the presenter's `0.35s` inter-shot stagger
    /// (`InterShotSeconds::DEFAULT`), so each enemy act's prior animation has a clear beat
    /// to read before the next act fires — the enemy turn reads as a deliberate act-by-act
    /// sequence rather than an instant volley. A tick count (not wall-clock) so headless
    /// tests stay deterministic; arbitrary-but-defensible, not pinned to any other tunable.
    pub const DEFAULT: u32 = 30;

    /// Build a cadence from a tick count.
    ///
    /// The public constructor (house style) so a future tunable / a test can seed a cadence
    /// without reaching the private field.
    #[must_use]
    pub const fn new(ticks: u32) -> Self {
        Self(ticks)
    }
}

impl Default for ActCadence {
    /// The default cadence is [`ActCadence::DEFAULT`] ticks.
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

/// The **enemy-act cooldown** — the number of ticks remaining before the brain may emit its
/// next enemy act (GTW-461).
///
/// A named newtype [`Resource`](bevy::prelude::Resource) over the remaining tick count
/// (no-bare-types). The brain decrements it each gated tick; when it reaches `0` the brain
/// emits ONE act and recharges the cooldown to the [`ActCadence`]. The derived [`Deref`]
/// reads the inner count; the inner field is PRIVATE, mutated only through
/// [`EnemyActCooldown::tick`] / [`EnemyActCooldown::recharge`].
///
/// **Lifetime tracks [`BattleInProgress`](crate::battle::BattleInProgress):** inserted at
/// `0` (so the first enemy act is immediate, no startup dwell) on the same successful-setup
/// `Ok` path as [`BattleInProgress`](crate::battle::BattleInProgress), removed alongside on
/// teardown.
#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnemyActCooldown(u32);

impl EnemyActCooldown {
    /// Build a cooldown ready to act NOW (`0` ticks remaining).
    ///
    /// Seeded at setup so the enemy's FIRST act of a battle is not delayed — the dwell only
    /// applies BETWEEN successive acts (the [`recharge`](EnemyActCooldown::recharge) after
    /// each emission), matching `advance_walk`'s "first step lands the same frame" behaviour.
    #[must_use]
    pub const fn ready() -> Self {
        Self(0)
    }

    /// Whether the cooldown has elapsed — the brain may emit an act this tick.
    #[must_use]
    pub const fn is_ready(self) -> bool {
        self.0 == 0
    }

    /// Count DOWN one tick toward readiness (saturating at `0` — never wraps below zero).
    ///
    /// Called once per gated tick while the cooldown is non-zero; the next tick after it
    /// reaches `0` lets the brain act.
    pub const fn tick(&mut self) {
        self.0 = self.0.saturating_sub(1);
    }

    /// Recharge the cooldown to `cadence` ticks — called right after the brain emits an act,
    /// so the NEXT act waits a full cadence-step.
    pub const fn recharge(&mut self, cadence: ActCadence) {
        self.0 = cadence.0;
    }
}

/// The brain's GTW-461 act-pacing reads, bundled into ONE [`SystemParam`] so
/// [`enemy_ai_turn`](crate::ai::enemy_ai_turn) stays under Bevy's 16-param system arity (the
/// `dispatch_fire` `ShooterQuery` / inspect-panel `InspectReads` bundling precedent).
///
/// Both resources are read as `Option` so the brain's access stays panic-free outside a live
/// battle (`bevy-traps.md` #1): an ABSENT cooldown means no pacing resource was seeded (a
/// lean harness) — the brain then acts every tick (the prior un-paced behaviour) rather than
/// stalling.
#[derive(SystemParam)]
pub struct ActPacing<'w> {
    /// The live cooldown counter, mutated (counted down / recharged) by [`ActPacing::gate`].
    cooldown: Option<ResMut<'w, EnemyActCooldown>>,
    /// The cadence ticks (an absent resource defaults to [`ActCadence::DEFAULT`]).
    cadence:  Option<Res<'w, ActCadence>>,
}

impl ActPacing<'_> {
    /// The cadence gate (GTW-461 step 2) — call ONCE at the top of the brain's per-tick pass.
    ///
    /// If the cooldown has NOT elapsed, count it down one tick and return `false` (the brain
    /// must emit NO act this tick — at most one act per cadence-step). If it HAS elapsed (or
    /// is absent), return `true` (the brain may emit an act). An absent cooldown always
    /// returns `true` (the un-paced fallback).
    pub fn gate(&mut self) -> bool {
        match self.cooldown.as_deref_mut() {
            Some(cooldown) if !cooldown.is_ready() => {
                cooldown.tick();
                false
            }
            _ => true,
        }
    }

    /// Recharge the cooldown to the cadence (GTW-461) — call right AFTER the brain emits an
    /// act, so the next act waits a full cadence-step. A no-op when the cooldown is absent.
    pub fn recharge(&mut self) {
        let cadence = self.cadence.as_deref().copied().unwrap_or_default();
        if let Some(cooldown) = self.cooldown.as_deref_mut() {
            cooldown.recharge(cadence);
        }
    }
}
