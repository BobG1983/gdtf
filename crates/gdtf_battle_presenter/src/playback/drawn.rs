//! The `Drawn*` components (GTW-727 C17) — the presenter's own mirror of the sim state it
//! is CURRENTLY SHOWING, as opposed to the state the sim has already reached.
//!
//! ## Why components, and not one shadow resource
//!
//! Three systems decide what a ganger looks like, and none of them reads a message: the
//! sprite mover filters on `Changed<Position>`, the appearance resolver on a union of
//! `Changed<Facing / Stance / Aiming / Suppressed / LifeState>`, and the death despawn on
//! `Changed<LifeState>`. A `HashMap<Entity, …>` resource cannot drive any of them:
//! `Res::is_changed()` is whole-resource granularity, so the only alternatives would be
//! repainting every ganger every frame — exactly what those `Changed<T>` filters exist to
//! prevent — or hand-rolling per-entity dirty tracking.
//!
//! Carrying the shown state as COMPONENTS on the same entity restores native per-entity
//! change detection. Each mirror's diff is one word: the filter and the read type change,
//! the body does not.
//!
//! ## The one-way dependency is untouched
//!
//! These types live in the presenter and are named nowhere in `gdtf_battle_sim` — they are
//! a VIEW of sim facts, written only by [`advance_playback`](super::advance_playback) from
//! values the sim already recorded. The sim neither reads them nor waits on them, and its
//! `Cargo.toml` gains no dependency.

use bevy::prelude::*;
use gdtf_battle_sim::{
    act_log::{MagazineFacts, PoseFacts, PositionFacts, VitalsFacts},
    ganger::{Aiming, Facing, Hp, LifeState, Position, Stance, Tu, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::InflictedInjuries,
    magazine::Magazine,
};

/// The `(cell, level)` a ganger is currently DRAWN at — replaces `Changed<Position>` as
/// the sprite mover's trigger.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawnPosition(PositionFacts);

impl DrawnPosition {
    /// Build a drawn position from a recorded (or seeded) settled position.
    #[must_use]
    pub const fn new(position: PositionFacts) -> Self {
        Self(position)
    }

    /// Build a drawn position by seeding it from a ganger's live sim position — used ONCE
    /// per ganger, when its mirror components are first inserted.
    #[must_use]
    pub const fn seeded(position: Position) -> Self {
        Self(PositionFacts::new(position))
    }

    /// The drawn position as a sim [`Position`].
    #[must_use]
    pub const fn position(&self) -> Position {
        self.0.inner()
    }
}

/// The POSTURE a ganger is currently drawn in — facing, stance, aim and suppression
/// together. Replaces the whole `Changed<Facing / Stance / Aiming / Suppressed>` union in
/// the appearance resolver.
///
/// Suppression is a FIELD here, not a component presence, which is what lets the resolver
/// delete its `RemovedComponents<Suppressed>` drain outright: clearing suppression is now
/// an ordinary value change that `Changed<DrawnPose>` observes like any other.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawnPose(PoseFacts);

impl DrawnPose {
    /// Build a drawn posture from a recorded (or seeded) settled posture.
    #[must_use]
    pub const fn new(pose: PoseFacts) -> Self {
        Self(pose)
    }

    /// The drawn facing.
    #[must_use]
    pub const fn facing(&self) -> Facing {
        self.0.facing
    }

    /// The drawn stance.
    #[must_use]
    pub const fn stance(&self) -> Stance {
        self.0.stance
    }

    /// Whether the ganger is drawn aiming.
    #[must_use]
    pub const fn aiming(&self) -> Aiming {
        self.0.aiming
    }

    /// Whether the ganger is drawn suppressed.
    #[must_use]
    pub const fn suppressed(&self) -> bool {
        self.0.suppressed.is_suppressed()
    }
}

/// The LIFE STATE a ganger is currently drawn in — replaces `Changed<LifeState>` in both
/// the appearance resolver and the death despawn.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawnLife(LifeState);

impl DrawnLife {
    /// Build a drawn life state.
    #[must_use]
    pub const fn new(life: LifeState) -> Self {
        Self(life)
    }
}

/// The VITALS a ganger's stat block is currently SHOWING — the cursor-time numbers the
/// status / inspect panels read in preference to the live ones.
///
/// This is the component that fixes the reported defect directly: without it the HUD reads
/// live `Hp` / `Wounds` / `InflictedInjuries`, so a shot's damage, its injury entry and its
/// floating text all appear the frame the sim resolves them — while the bolt that caused
/// them is still mid-flight.
///
/// NOT `Copy`: the wound and injury ledgers are owned lists.
#[derive(Component, Deref, Debug, Clone, PartialEq)]
pub struct DrawnVitals(VitalsFacts);

impl DrawnVitals {
    /// Build drawn vitals from a recorded (or seeded) settled snapshot.
    #[must_use]
    pub const fn new(vitals: VitalsFacts) -> Self {
        Self(vitals)
    }

    /// The drawn time units.
    #[must_use]
    pub const fn tu(&self) -> Tu {
        self.0.tu
    }

    /// The drawn hit points.
    #[must_use]
    pub const fn hp(&self) -> Hp {
        self.0.hp
    }

    /// The drawn remaining wounds pool.
    #[must_use]
    pub const fn wounds(&self) -> Wounds {
        self.0.wounds
    }

    /// The drawn inflicted-wound list.
    #[must_use]
    pub const fn inflicted(&self) -> &InflictedWounds {
        &self.0.inflicted
    }

    /// The drawn injury ledger.
    #[must_use]
    pub const fn injuries(&self) -> &InflictedInjuries {
        &self.0.injuries
    }
}

/// The MAGAZINE a weapon panel is currently SHOWING — carried on the WEAPON entity, the
/// same entity the panel resolves through `ganger → Wields → weapon`.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawnMagazine(MagazineFacts);

impl DrawnMagazine {
    /// Build a drawn magazine from a recorded (or seeded) settled magazine.
    #[must_use]
    pub const fn new(magazine: MagazineFacts) -> Self {
        Self(magazine)
    }

    /// The drawn magazine.
    #[must_use]
    pub const fn magazine(&self) -> Magazine {
        self.0.inner()
    }
}
