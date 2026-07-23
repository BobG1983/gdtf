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

use bevy::{
    ecs::{message::Messages, system::SystemParam},
    input::keyboard::{KeyCode, KeyboardInput},
    input_focus::InputFocus,
    prelude::*,
    window::{CursorMoved, PrimaryWindow},
};
use gdtf_battle_input::{
    PendingActIntent, SelectedShooter,
    contextual::{
        ContextualAct, EnterEmplacementAct, ExecuteAct, ExitEmplacementAct, MeleeAct, OpenDoorAct,
        PendingContextualIntents, ShoveAct, StabilizeAct, ThrowGrenadeAct,
    },
    keybinds::Keybinds,
};
use gdtf_battle_sim::{
    acts::MeleeTarget,
    ganger::{Aiming, LifeState},
    terrain::{emplacement::EmplacementState, openable::OpenState},
    weapon::{FireMode, FireModeSpec, MeleeWeapon, WieldedBy, Wields},
};
use gdtf_qa_protocol::{
    envelope::RejectReason,
    ids::{FireModeIndex, FocusTargetNet, GangerToken},
    intent::{KeyPressNet, KeybindActionNet, MeleeTargetNet},
};

use super::convert::{cell_level, key_code_of};
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

/// The write-side bundle the GTW-783 raw-input pump drives — the SAME windowing-input path
/// the backend feeds, never a direct sim mutation: the buffered `KeyboardInput` /
/// `CursorMoved` message streams, the primary window (whose cursor position a hover sets),
/// the [`InputFocus`] resource a focus-set points, plus the live [`Keybinds`] a bound-action
/// keypress resolves through and an all-entity liveness probe for the focus token.
///
/// Every field a system might touch off-battle or under `MinimalPlugins` is
/// `Option`-wrapped (the message streams, focus, keybinds) or an empty-safe `Query` (the
/// window / entity probes), so the pump stays inert-safe when the windowing / focus stack is
/// absent (`bevy-traps.md` #1) — the real app (with `DefaultPlugins`) always has them.
#[derive(SystemParam)]
pub(super) struct RawInputSink<'w, 's> {
    /// The buffered keyboard-input message stream — where a keypress writes its
    /// press+release pair, exactly as the windowing backend does (folded into
    /// `ButtonInput<KeyCode>` by Bevy's `keyboard_input_system`).
    pub(super) key_events:    Option<ResMut<'w, Messages<KeyboardInput>>>,
    /// The buffered cursor-moved message stream — where a hover writes its move.
    pub(super) cursor_events: Option<ResMut<'w, Messages<CursorMoved>>>,
    /// The primary window — whose cursor position a hover sets (what `bevy_ui`'s hover
    /// detection reads).
    pub(super) windows:       Query<'w, 's, (Entity, &'static mut Window), With<PrimaryWindow>>,
    /// The UI input-focus resource — where a focus-set points, the SAME write
    /// `sync_hover_to_focus` performs.
    pub(super) focus:         Option<ResMut<'w, InputFocus>>,
    /// The live keybind table — a bound-action keypress resolves its key through it.
    pub(super) keybinds:      Option<Res<'w, Keybinds>>,
    /// An all-entity liveness probe — a focus token resolves only to an entity that exists.
    pub(super) entities:      Query<'w, 's, Entity>,
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

/// Resolve a wire [`KeyPressNet`] to the Bevy [`KeyCode`] it taps (GTW-783).
///
/// A physical [`Key`](KeyPressNet::Key) maps directly ([`key_code_of`], pure); a bound
/// [`Action`](KeyPressNet::Action) reads whatever key the live [`Keybinds`] table currently
/// binds it to. Returns [`None`] only when a bound-action keypress arrives with no
/// [`Keybinds`] resource present (off-battle / `MinimalPlugins`) — the caller then emits no
/// key but still reports the intent queued (an outcome concern, not a wire rejection).
pub(super) fn resolve_key(press: KeyPressNet, sink: &RawInputSink) -> Option<KeyCode> {
    match press {
        KeyPressNet::Key(key) => Some(key_code_of(key)),
        KeyPressNet::Action(action) => sink
            .keybinds
            .as_ref()
            .map(|keybinds| action_key_code(action, keybinds)),
    }
}

/// Map a wire [`KeybindActionNet`] onto the [`KeyCode`] the live [`Keybinds`] binds it to —
/// the state-dependent half of keypress resolution.
const fn action_key_code(action: KeybindActionNet, keybinds: &Keybinds) -> KeyCode {
    match action {
        KeybindActionNet::SelectClear => keybinds.select_clear(),
        KeybindActionNet::LevelUp => keybinds.level_up(),
        KeybindActionNet::LevelDown => keybinds.level_down(),
        KeybindActionNet::ToggleFullView => keybinds.toggle_full_view(),
        KeybindActionNet::StanceCycle => keybinds.stance_cycle(),
        KeybindActionNet::AimToggle => keybinds.aim_toggle(),
        KeybindActionNet::FacingCycle => keybinds.facing_cycle(),
        KeybindActionNet::SelectNext => keybinds.select_next(),
        KeybindActionNet::SelectPrev => keybinds.select_prev(),
    }
}

/// Resolve a wire [`FocusTargetNet`] to a live entity FAIL-CLOSED (GTW-783): a malformed bit
/// pattern (via [`Entity::try_from_bits`], never `from_bits`, which panics) or an entity that
/// no longer exists is a typed [`RejectReason::UnknownEntity`], never a panic. Unlike the
/// gameplay tokens a focus target is any UI entity, so its liveness is bare existence.
pub(super) fn resolve_focus_target(
    token: FocusTargetNet,
    sink: &RawInputSink,
) -> Result<Entity, RejectReason> {
    resolve_live(token, |entity| sink.entities.contains(entity))
}
