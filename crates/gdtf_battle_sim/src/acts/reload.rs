//! Dispatch reload requests against the actor's ranged magazine.

use bevy::{
    ecs::query::With,
    prelude::{Entity, Message, MessageReader, MessageWriter, Query},
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

        let cost = Tu::new(*magazine.reload_tu());
        if !*can_spend_tu(&tu, cost) {
            results.write(ReloadResult::new(request.actor, ReloadOutcome::NoTu));
            continue;
        }

        spend_tu(&mut tu, cost);
        magazine.refill();
        results.write(ReloadResult::new(request.actor, ReloadOutcome::Reloaded));
    }
}
