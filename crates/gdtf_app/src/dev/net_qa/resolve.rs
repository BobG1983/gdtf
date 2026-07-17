//! The world-touching resolution the inject pump reads (GTW-737): the two
//! `SystemParam` bundles [`apply_injects`](super::inject::apply_injects) drives, the
//! FAIL-CLOSED token resolvers, the fire-mode lookup, and the contextual offer gate.
//!
//! Split from [`inject`](super::inject) (module-layout: the drain/dispatch is one
//! concern, the input-queue bundle + resolution another). Every entity token resolves via
//! [`Entity::try_from_bits`] (NEVER `from_bits`, which PANICS on a malformed bit
//! pattern) plus a liveness check, so a malformed OR dead token yields a typed
//! [`RejectReason::UnknownEntity`] — never a panic, never a silent drop (GTW-737
//! clause 2).

use core::ops::Deref;

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_input::{
    PendingActIntent, SelectedShooter,
    contextual::{
        ContextualAct, EnterEmplacementAct, ExecuteAct, ExitEmplacementAct, MeleeAct, OpenDoorAct,
        PendingContextualIntents, ShoveAct, StabilizeAct, ThrowGrenadeAct,
    },
};
use gdtf_battle_sim::{
    acts::MeleeTarget,
    ganger::{Aiming, LifeState},
    terrain::{emplacement::EmplacementState, openable::OpenState},
    weapon::{FireMode, FireModeSpec, MeleeWeapon, WieldedBy, Wields},
};
use gdtf_qa_protocol::{
    envelope::RejectReason,
    ids::{FireModeIndex, GangerToken},
    intent::MeleeTargetNet,
};

use super::convert::cell_level;
use crate::states::running::game::battlescape::contextual_panel::ContextualOffer;

/// The WRITE-side bundle the inject pump pushes through — the SAME public intent queues the
/// local keyboard / button / panel surfaces use ([`PendingActIntent`] for classic acts,
/// the per-act [`PendingContextualIntents`] for contextual ones), plus the per-act
/// [`ContextualOffer`]s the offer gate reads and the current selection the actor-bearing
/// intents need. No bypass, no shadow queue (GTW-737 clause 4). Each offer is
/// `Option<Res<…>>` so the pump is inert-safe if the contextual panel has not registered
/// them (`bevy-traps.md` #1); the eighteen distinct resource types never alias.
#[derive(SystemParam)]
pub(super) struct InjectQueues<'w> {
    /// The classic-intent queue (`ActIntent::push` write-point).
    pub(super) pending:         ResMut<'w, PendingActIntent>,
    /// The current selection — the implicit actor of the actor-bearing intents.
    pub(super) selected:        Res<'w, SelectedShooter>,
    /// The Melee per-act intent queue.
    pub(super) melee_queue:     ResMut<'w, PendingContextualIntents<MeleeAct>>,
    /// The Shove per-act intent queue.
    pub(super) shove_queue:     ResMut<'w, PendingContextualIntents<ShoveAct>>,
    /// The Stabilize per-act intent queue.
    pub(super) stabilize_queue: ResMut<'w, PendingContextualIntents<StabilizeAct>>,
    /// The Execute per-act intent queue.
    pub(super) execute_queue:   ResMut<'w, PendingContextualIntents<ExecuteAct>>,
    /// The Throw-Grenade per-act intent queue.
    pub(super) throw_queue:     ResMut<'w, PendingContextualIntents<ThrowGrenadeAct>>,
    /// The Open-Door per-act intent queue.
    pub(super) door_queue:      ResMut<'w, PendingContextualIntents<OpenDoorAct>>,
    /// The Enter-Emplacement per-act intent queue.
    pub(super) enter_queue:     ResMut<'w, PendingContextualIntents<EnterEmplacementAct>>,
    /// The Exit-Emplacement per-act intent queue.
    pub(super) exit_queue:      ResMut<'w, PendingContextualIntents<ExitEmplacementAct>>,
    /// The Melee offer (the offer gate's read).
    pub(super) melee_offer:     Option<Res<'w, ContextualOffer<MeleeAct>>>,
    /// The Shove offer.
    pub(super) shove_offer:     Option<Res<'w, ContextualOffer<ShoveAct>>>,
    /// The Stabilize offer.
    pub(super) stabilize_offer: Option<Res<'w, ContextualOffer<StabilizeAct>>>,
    /// The Execute offer.
    pub(super) execute_offer:   Option<Res<'w, ContextualOffer<ExecuteAct>>>,
    /// The Throw-Grenade offer.
    pub(super) throw_offer:     Option<Res<'w, ContextualOffer<ThrowGrenadeAct>>>,
    /// The Open-Door offer.
    pub(super) door_offer:      Option<Res<'w, ContextualOffer<OpenDoorAct>>>,
    /// The Enter-Emplacement offer.
    pub(super) enter_offer:     Option<Res<'w, ContextualOffer<EnterEmplacementAct>>>,
    /// The Exit-Emplacement offer.
    pub(super) exit_offer:      Option<Res<'w, ContextualOffer<ExitEmplacementAct>>>,
}

/// The READ-only world queries the resolvers consult — the actor's aim + wielded weapon
/// (for the fire-mode lookup and the aim-toggle decision) and the liveness filters the
/// token resolvers check membership against. All borrows are shared, so no field aliases
/// another (no `B0001`).
#[derive(SystemParam)]
pub(super) struct InjectActors<'w, 's> {
    /// The actor's current aim flag (the aim-toggle decision).
    pub(super) aiming:       Query<'w, 's, &'static Aiming>,
    /// The actor's wielded-weapon relationship (fire-mode resolution).
    pub(super) wields:       Query<'w, 's, &'static Wields>,
    /// The wielded weapon's authored fire-mode list.
    pub(super) weapons:      Query<'w, 's, &'static FireMode, With<WieldedBy>>,
    /// The melee-weapon marker probe (so the RANGED weapon resolves).
    pub(super) melee_marker: Query<'w, 's, (), With<MeleeWeapon>>,
    /// Ganger liveness — a token resolves only to an entity that still holds a
    /// [`LifeState`].
    pub(super) gangers:      Query<'w, 's, (), With<LifeState>>,
    /// Door liveness — a token resolves only to an entity that still holds an
    /// [`OpenState`].
    pub(super) doors:        Query<'w, 's, (), With<OpenState>>,
    /// Emplacement liveness — a token resolves only to an entity that still holds an
    /// [`EmplacementState`].
    pub(super) emplacements: Query<'w, 's, (), With<EmplacementState>>,
}

/// Resolve an entity token FAIL-CLOSED (GTW-737 clause 2): [`try_from_bits`] rejects a
/// malformed bit pattern (never `from_bits`, which panics), and `is_live` re-validates
/// the entity still exists holding its required component. Either miss is a typed
/// [`RejectReason::UnknownEntity`], never a panic.
///
/// [`try_from_bits`]: Entity::try_from_bits
fn resolve_live<T: Deref<Target = u64>>(
    token: T,
    is_live: impl Fn(Entity) -> bool,
) -> Result<Entity, RejectReason> {
    let Some(entity) = Entity::try_from_bits(*token) else {
        return Err(RejectReason::UnknownEntity);
    };
    if is_live(entity) {
        Ok(entity)
    } else {
        Err(RejectReason::UnknownEntity)
    }
}

/// Resolve a ganger token to a live ganger entity (holds a [`LifeState`]), fail-closed.
pub(super) fn resolve_ganger(
    token: GangerToken,
    actors: &InjectActors,
) -> Result<Entity, RejectReason> {
    resolve_live(token, |entity| actors.gangers.get(entity).is_ok())
}

/// Resolve the wire melee target — a ganger token (fail-closed) or a structure cell — to
/// the sim [`MeleeTarget`].
pub(super) fn resolve_melee(
    target: MeleeTargetNet,
    actors: &InjectActors,
) -> Result<MeleeTarget, RejectReason> {
    match target {
        MeleeTargetNet::Ganger(token) => resolve_ganger(token, actors).map(MeleeTarget::Ganger),
        MeleeTargetNet::Structure(cell) => Ok(MeleeTarget::Structure(cell_level(cell))),
    }
}

/// Resolve a door token to a live door entity (holds an [`OpenState`]), fail-closed.
pub(super) fn resolve_door(
    token: gdtf_qa_protocol::ids::DoorToken,
    actors: &InjectActors,
) -> Result<Entity, RejectReason> {
    resolve_live(token, |entity| actors.doors.get(entity).is_ok())
}

/// Resolve an emplacement token to a live emplacement entity (holds an
/// [`EmplacementState`]), fail-closed.
pub(super) fn resolve_emplacement(
    token: gdtf_qa_protocol::ids::EmplacementToken,
    actors: &InjectActors,
) -> Result<Entity, RejectReason> {
    resolve_live(token, |entity| actors.emplacements.get(entity).is_ok())
}

/// Resolve a wire fire-mode INDEX against the actor's wielded RANGED weapon's authored
/// mode list — the mode a [`Fire`](gdtf_qa_protocol::intent::NetIntent::Fire) injects.
///
/// An index past the list (or an unarmed actor, whose weapon offers zero modes) is a
/// typed [`RejectReason::BadFireMode`] — the client named a mode the weapon does not
/// offer. Mirrors the local fire surface's `ganger → Wields → the ranged weapon → FireMode`
/// resolution (`ranged_weapon` excludes the melee weapon the ganger also wields).
pub(super) fn resolve_fire_mode(
    actor: Entity,
    mode: FireModeIndex,
    actors: &InjectActors,
) -> Result<FireModeSpec, RejectReason> {
    let fire_mode = actors
        .wields
        .get(actor)
        .ok()
        .and_then(|wields| wields.ranged_weapon(|entity| actors.melee_marker.get(entity).is_ok()))
        .and_then(|weapon| actors.weapons.get(weapon).ok());
    let Some(fire_mode) = fire_mode else {
        return Err(RejectReason::BadFireMode);
    };
    let Some(spec) = fire_mode.get(*mode as usize) else {
        return Err(RejectReason::BadFireMode);
    };
    Ok(*spec)
}

/// The contextual OFFER GATE (GTW-737 clause 3): push `resolved` onto the act's queue
/// only when the game is currently OFFERING exactly that target, else reject
/// [`NotOffered`](RejectReason::NotOffered) — never a silent push regardless of offer
/// state. Mirrors the local press router, which only ever acts on the offered target.
pub(super) fn gate_and_push<A: ContextualAct>(
    resolved: A::Target,
    offer: Option<&ContextualOffer<A>>,
    queue: &mut PendingContextualIntents<A>,
) -> Result<(), RejectReason> {
    match offer.and_then(ContextualOffer::target) {
        Some(offered) if offered == resolved => {
            queue.push(resolved);
            Ok(())
        }
        _ => Err(RejectReason::NotOffered),
    }
}
