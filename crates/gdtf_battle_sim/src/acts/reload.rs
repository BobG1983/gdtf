//! authored in the weapon `.ron`, USER DIRECTIVE 2026-06-17 — NOT a global tuning
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadOutcome {
            Reloaded,
                AlreadyFull,
                NoTu,
}

/// A buffered Bevy **message** (`#[derive(Message)]`), NOT the observer `Event`
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReloadResult {
        pub actor:   Entity,
        pub outcome: ReloadOutcome,
}

impl ReloadResult {
            #[must_use]
    pub const fn new(actor: Entity, outcome: ReloadOutcome) -> Self {
        Self { actor, outcome }
    }
}

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
