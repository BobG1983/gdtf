//! The **openable toggle** — the SIM mechanism that flips an
//! [`OpenState`] and drives the GTW-501 / GTW-502 blocking components
//! (GTW-503 C3 / C4). This is the API GTW-315 (the contextual Open-Door act + button) will
//! CALL; GTW-503 ships the sim mechanism ONLY — no UI, no act, no keybind.
//!
//! ## How it reuses GTW-501 / GTW-502 (C4 — no new projection/recompute)
//!
//! Toggling does NOT add any projection or recompute system. It only ADDS or REMOVES the
//! [`BlocksPathfinding`] +
//! [`BlocksVision`] components on the door entity:
//!
//! - on `→ Open` it REMOVES both, which trips
//!   `RemovedComponents<BlocksPathfinding>` / `RemovedComponents<BlocksVision>` — the existing
//!   [`project_path_blocking`] /
//!   [`project_vision_blocking`] systems re-open
//!   the cell on both surfaces, and
//!   [`should_recompute_visibility`](crate::visibility::should_recompute_visibility) re-fires
//!   the squad-fog recompute (its `RemovedComponents<BlocksVision>` arm);
//! - on `→ Closed` it INSERTS both (the vision component at the band recorded in
//!   [`OpenableBlocking`]), which trips `Added<…>` — the same
//!   projections re-block the cell.
//!
//! So a path / `LoS` query reflects the new door state once the existing GTW-501 / GTW-502
//! systems run (one tick after the toggle's deferred `Commands` apply — see ORDERING below).
//!
//! ## Ordering vs the GTW-501 / GTW-502 projection (C3)
//!
//! [`apply_openable_toggle`] runs `.in_set(`[`SimSystems::Simulate`]`)`
//! and `.before(`[`project_path_blocking`]`)` /
//! `.before(`[`project_vision_blocking`]`)`. But a
//! `Commands` insert/remove is DEFERRED — it applies at the next sync point (`bevy-traps.md`
//! #7 note), so the component add/remove this system queues is NOT visible to the SAME tick's
//! projection; the projection's `Added` / `RemovedComponents` fires on the FOLLOWING tick.
//! That **one-frame settle** is intentional and acceptable (the GTW-503 ticket permits a
//! documented one-frame settle): the change-detection lands deterministically the next
//! `app.update()`, exactly as the GTW-501 / GTW-502 spawn-insert already settles one tick
//! after `setup_battle`. The `.before` ordering still matters for the `OpenState` flip itself
//! (a same-tick reader sees the new state) and keeps the toggle deterministically ordered
//! against the chain that shares no resource with it.
//!
//! `bevy-traps.md` #7: a normal system over `MessageReader` / `Query` / `Commands` — no
//! `&mut World`.

use bevy::prelude::{
    App, Commands, Entity, IntoScheduleConfigs, Message, MessageReader, Plugin, Query, Update,
};

use super::{OpenState, OpenableBlocking};
use crate::{
    occupancy::{project_path_blocking, project_vision_blocking},
    occupancy_sync::SimSystems,
    terrain::entity::{BlocksPathfinding, BlocksVision},
};

/// A request to set an openable piece's [`OpenState`] — the toggle message GTW-315 will write
/// (GTW-503 C3).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`, mirroring the
/// landed `*Requested` act messages). Carries the door [`Entity`] plus the TARGET
/// [`OpenState`]. `state` is an absolute set (open OR close), not a blind flip, so the caller
/// (a UI button bound to "open" / "close", or a context act) drives an explicit, idempotent
/// outcome — re-sending `Open` for an already-open door is a harmless no-op. For a fieldless
/// flip, read the current state and send `OpenState::toggled()` (see
/// [`SetOpenable::toggle`]).
///
/// The door is a Bevy [`Entity`] handle — framework plumbing, the only bare type the
/// no-bare-types rule permits in a message payload (it is not a domain value).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetOpenable {
    /// The openable terrain entity to (re)set.
    entity: Entity,
    /// The target state to set it to — [`Open`](OpenState::Open) or
    /// [`Closed`](OpenState::Closed).
    state:  OpenState,
}

impl SetOpenable {
    /// Build a set-state request for `entity` → `state` (the explicit open-or-close form).
    #[must_use]
    pub const fn new(entity: Entity, state: OpenState) -> Self {
        Self { entity, state }
    }

    /// Build an OPEN request for `entity` (a convenience over [`new`](SetOpenable::new)).
    #[must_use]
    pub const fn open(entity: Entity) -> Self {
        Self::new(entity, OpenState::Open)
    }

    /// Build a CLOSE request for `entity` (a convenience over [`new`](SetOpenable::new)).
    #[must_use]
    pub const fn close(entity: Entity) -> Self {
        Self::new(entity, OpenState::Closed)
    }

    /// Build a FLIP request for `entity` given its CURRENT state — the fieldless-toggle form
    /// GTW-315's "Open/Close door" button maps to (open ⇄ closed via
    /// [`OpenState::toggled`]).
    #[must_use]
    pub const fn toggle(entity: Entity, current: OpenState) -> Self {
        Self::new(entity, current.toggled())
    }

    /// The targeted door entity.
    #[must_use]
    pub const fn entity(self) -> Entity {
        self.entity
    }

    /// The target state.
    #[must_use]
    pub const fn state(self) -> OpenState {
        self.state
    }
}

/// Apply every buffered [`SetOpenable`]: flip the door's [`OpenState`] and add/remove the
/// GTW-501 / GTW-502 blocking components to match (GTW-503 C3).
///
/// For each message, the entity's [`OpenState`] + its [`OpenableBlocking`] band record are
/// fetched from the `doors` query (a `SetOpenable` for a non-openable / despawned entity — no
/// `OpenState` — is skipped, panic-free). Then, when the requested state DIFFERS from the
/// current one (idempotent — re-setting the same state is a no-op that writes nothing, so it
/// never trips a spurious `Changed`):
///
/// - the [`OpenState`] is written to the target (a same-tick reader sees the new state); and
/// - `→ Open` REMOVES [`BlocksPathfinding`] + [`BlocksVision`] (the door clears path + vision)
///   via `Commands`; `→ Closed` INSERTS them — [`BlocksVision`] at the band recorded in
///   [`OpenableBlocking`] (C2). These deferred component edits drive the GTW-501 / GTW-502
///   change-detection on the NEXT tick (the documented one-frame settle — see the module
///   docs), re-projecting both surfaces + re-firing the squad-fog recompute (C4).
///
/// `bevy-traps.md` #7: `MessageReader` / `Query` / `Commands` — no `&mut World`.
/// `bevy-traps.md` #4: a buffered `MessageReader`, drained every run.
pub fn apply_openable_toggle(
    mut requests: MessageReader<SetOpenable>,
    mut doors: Query<(&mut OpenState, &OpenableBlocking)>,
    mut commands: Commands,
) {
    for request in requests.read() {
        let Ok((mut open_state, blocking)) = doors.get_mut(request.entity()) else {
            // Not an openable entity (no OpenState / despawned) — skip, panic-free.
            continue;
        };
        if *open_state == request.state() {
            // Idempotent: already in the requested state — no flip, no component edit, and no
            // spurious `Changed<OpenState>`.
            continue;
        }
        *open_state = request.state();
        let mut entity = commands.entity(request.entity());
        if *request.state().is_open() {
            // Open: clear BOTH — the GTW-501 / GTW-502 RemovedComponents change-detection
            // re-opens path + vision and re-fires the squad-fog recompute next tick.
            entity.remove::<BlocksPathfinding>();
            entity.remove::<BlocksVision>();
        } else {
            // Closed: re-block BOTH — path-blocking marker + the height-aware vision occluder
            // at the recorded band (C2). `Added` fires next tick → the surfaces re-block.
            entity.insert(BlocksPathfinding);
            entity.insert(BlocksVision::new(**blocking));
        }
    }
}

/// Wires the openable toggle into a Bevy [`App`] (GTW-503 C3 / C6) — registers the
/// [`SetOpenable`] message buffer and adds [`apply_openable_toggle`] to [`Update`].
///
/// In `build()` the plugin:
///
/// - [`add_message`](App::add_message)s [`SetOpenable`] (`bevy-traps.md` #5 — an unregistered
///   buffer fails [`apply_openable_toggle`]'s `MessageReader` param validation); and
/// - adds [`apply_openable_toggle`] `.in_set(`[`SimSystems::Simulate`]`)`,
///   ordered `.before` BOTH the GTW-501
///   [`project_path_blocking`] and the GTW-502
///   [`project_vision_blocking`] — so the toggle's
///   component add/remove is QUEUED before the projection runs (the projection sees it on the
///   next tick once `Commands` apply, the documented one-frame settle — module docs / C3).
///
/// The `SimSystems::Simulate` set is OWNED upstream (by
/// [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin), which
/// `configure_sets` it), so this plugin only `.in_set`s into it — it never re-configures the
/// set (`bevy-traps.md` #5). It REUSES the GTW-501 / GTW-502 projection + the GTW-341 /
/// GTW-502 recompute (C4 — no new projection or recompute system here).
#[derive(Debug, Default, Clone, Copy)]
pub struct OpenableTogglePlugin;

impl Plugin for OpenableTogglePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SetOpenable>().add_systems(
            Update,
            apply_openable_toggle
                .in_set(SimSystems::Simulate)
                .before(project_path_blocking)
                .before(project_vision_blocking),
        );
    }
}
