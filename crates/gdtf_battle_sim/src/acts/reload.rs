//! Dispatch reload requests against the actor's ranged magazine.

use bevy::{
    ecs::query::With,
    prelude::{Deref, Entity, Message, MessageReader, MessageWriter, Query},
};

use crate::{
    acts::request::ReloadRequested,
    fire::MeleeQuery,
    ganger::{LifeState, Tu},
    magazine::Magazine,
    tu::{can_spend_tu, spend_tu},
    weapon::{WieldedBy, Wields},
};

/// Result of a reload attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadOutcome {
    /// Magazine refilled.
    Reloaded,
    /// Already full.
    AlreadyFull,
    /// Not enough TU.
    NoTu,
}

/// Outcome message for a reload attempt.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReloadResult {
    /// Actor who tried to reload.
    pub actor:   Entity,
    /// Outcome.
    pub outcome: ReloadOutcome,
}

impl ReloadResult {
    /// Build a result message.
    #[must_use]
    pub const fn new(actor: Entity, outcome: ReloadOutcome) -> Self {
        Self { actor, outcome }
    }
}

/// Whether the actor may reload this magazine right now.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanReload(bool);

impl CanReload {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(allowed: bool) -> Self {
        Self(allowed)
    }
}

/// TU charged for refilling this magazine.
#[must_use]
pub fn reload_tu_cost(magazine: &Magazine) -> Tu {
    Tu::new(*magazine.reload_tu())
}

/// Alive, the magazine has room, and the pool covers the cost.
#[must_use]
pub fn can_reload(life: LifeState, tu: &Tu, magazine: &Magazine) -> CanReload {
    CanReload::new(
        life == LifeState::Alive
            && !*magazine.is_full()
            && *can_spend_tu(tu, reload_tu_cost(magazine)),
    )
}

/// System: process [`ReloadRequested`] messages.
pub fn dispatch_reload(
    mut requests: MessageReader<ReloadRequested>,
    mut actors: Query<(&'static mut Tu, &'static LifeState, &'static Wields)>,
    mut weapons: Query<&'static mut Magazine, With<WieldedBy>>,
    melee: MeleeQuery,
    mut results: MessageWriter<ReloadResult>,
) {
    for request in requests.read() {
        let Ok((mut tu, &life, wields)) = actors.get_mut(request.actor) else {
            continue;
        };

        if life != LifeState::Alive {
            continue;
        }

        let Some(weapon_entity) = wields.ranged_weapon(|entity| melee.get(entity).is_ok()) else {
            continue;
        };
        let Ok(mut magazine) = weapons.get_mut(weapon_entity) else {
            continue;
        };

        if *magazine.is_full() {
            results.write(ReloadResult::new(request.actor, ReloadOutcome::AlreadyFull));
            continue;
        }

        if !*can_reload(life, &tu, &magazine) {
            results.write(ReloadResult::new(request.actor, ReloadOutcome::NoTu));
            continue;
        }

        if spend_tu(&mut tu, reload_tu_cost(&magazine)).is_err() {
            results.write(ReloadResult::new(request.actor, ReloadOutcome::NoTu));
            continue;
        }
        magazine.refill();
        results.write(ReloadResult::new(request.actor, ReloadOutcome::Reloaded));
    }
}
