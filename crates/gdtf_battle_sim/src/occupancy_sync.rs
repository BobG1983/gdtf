//! Change-driven occupancy maintenance — the E1.7 sync slice (GTW-157).
//!
//! GTW-156 ([`crate::occupancy`]) shipped the [`OccupancyGrid`] resource with a
//! `build_from_occupancy_input` constructor that pours an occupancy input into a
//! fresh grid. That constructor is the **setup-time** pour, NOT the per-frame
//! maintenance path: per `docs/architecture.md`'s "change-driven grid
//! maintenance" thesis (and the GTW-6 / GTW-12 architectural ruling), the live
//! battle keeps the grid current by reacting to the **changes** — a ganger moved,
//! a ganger went down, a piece of cover was smashed — and editing the grid IN
//! PLACE, never by re-running a full-grid rebuild per shot.
//!
//! This module is that maintenance layer. It is three focused Bevy systems, all
//! sharing `ResMut<`[`OccupancyGrid`]`>`, each driven by a SINGLE trigger:
//!
//! 1. [`sync_moved_gangers`] reacts to `Changed<`[`Position`]`>` (Bevy change
//!    detection): it clears the entity's OLD occupancy slot and marks its NEW one.
//!    Because `Changed<Position>` only ever yields the *new* [`Position`], the
//!    system tracks each entity's previously-synced slot in a [`PrevSlot`]
//!    component it writes itself — that is how it knows which old slot to clear
//!    (the ticket-sanctioned "track the previous slot" approach).
//! 2. [`sync_dead_gangers`] reacts to `Changed<`[`LifeState`]`>` filtered to a
//!    non-[`LifeState::Alive`] state (Downed / Dead): it clears that entity's
//!    occupant marker from the slot it last synced to.
//! 3. [`sync_destroyed_cover`] reads the buffered [`CoverDestroyed`] **message**
//!    (Bevy 0.18 renamed buffered events to messages — `bevy-traps.md` #4) and
//!    folds each one's [`CellLevel`] into the grid's append-only destroyed-cover
//!    set via [`OccupancyGrid::mark_cover_destroyed`].
//!
//! **Change detection / the message reader is the ONLY trigger.** There is no
//! polling, no per-shot full-grid scan, and NOTHING here calls
//! [`OccupancyGrid::build_from_occupancy_input`] — a rebuild-per-shot is the exact
//! anti-pattern this slice exists to replace.
//!
//! [`OccupancyMaintenancePlugin`] is the wiring unit: it registers the
//! [`CoverDestroyed`] message buffer and adds the three systems to [`Update`] in
//! an EXPLICIT [`chain`](bevy::prelude::IntoScheduleConfigs::chain) order
//! (`bevy-traps.md` #3 — three systems sharing one `ResMut` must be ordered
//! deterministically). The production app adds this plugin when the sim is wired
//! into the runtime (E1.8 / E5); that app-wiring is out of scope here, so the
//! systems are exercised by this module's headless tests.

use bevy::prelude::{
    App, Changed, Commands, Component, Entity, IntoScheduleConfigs, Message, MessageReader, Plugin,
    Query, ResMut, Update,
};

use crate::{
    ganger::{LifeState, Position},
    metric::CellLevel,
    occupancy::OccupancyGrid,
};

/// The `(cell, level)` slot an entity was **last synced into** the occupancy grid
/// at — the per-entity bookkeeping [`sync_moved_gangers`] uses to clear the OLD
/// slot on a move.
///
/// `Changed<`[`Position`]`>` only ever yields the entity's *new* [`Position`], so
/// the move system cannot tell where the entity *was* without remembering it.
/// This component is that memory: the sync systems WRITE it (the maintenance
/// layer owns it — it is not authored ganger state), and read it back to know
/// which slot's occupant marker to clear. A named newtype over [`CellLevel`]
/// (no-bare-types: a tracked slot is a domain value, not a bare key); private
/// inner + a [`new`](PrevSlot::new) constructor, no public `Deref` since callers
/// only ever compare/read the whole slot through [`slot`](PrevSlot::slot).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrevSlot(CellLevel);

impl PrevSlot {
    /// Build a previously-synced-slot marker for the `(cell, level)` an entity was
    /// last synced into.
    #[must_use]
    pub const fn new(slot: CellLevel) -> Self {
        Self(slot)
    }

    /// The `(cell, level)` slot this marker records — the slot to clear when the
    /// entity moves on or goes down.
    #[must_use]
    pub const fn slot(self) -> CellLevel {
        self.0
    }
}

/// A piece of cover was **destroyed** at a `(cell, level)` — the buffered message
/// [`sync_destroyed_cover`] folds into the grid's destroyed-cover set.
///
/// Per `docs/architecture.md` the sim's destroyed-cover signal is
/// `CoverDestroyed { cell, level }`; the GTW-154 [`crate::cover::CoverEvent::Destroyed`]
/// depletion result is what becomes this message (a deplete→message bridge is a
/// later slice — this slice consumes the message). Carries a [`CellLevel`]
/// (no-bare-types; never a numeric id). It is a **buffered message**, NOT the
/// observer `Event` API: Bevy 0.18 renamed buffered `Event`/`EventReader` to
/// `Message`/`MessageReader` (`bevy-traps.md` #4 — the ticket's "`EventReader`"
/// is pre-0.18 terminology, identical semantics), so it `#[derive(Message)]` and
/// is read with [`MessageReader`].
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoverDestroyed {
    /// The `(cell, level)` whose cover was destroyed — added to the grid's
    /// append-only destroyed-cover set.
    pub at: CellLevel,
}

impl CoverDestroyed {
    /// Build a cover-destroyed message for the `(cell, level)` that was smashed.
    #[must_use]
    pub const fn new(at: CellLevel) -> Self {
        Self { at }
    }
}

/// Maintain the occupancy grid for **moved** gangers — `Changed<`[`Position`]`>`.
///
/// Reacts to Bevy change detection: for every entity whose [`Position`] changed
/// this frame, it clears the entity's OLD occupancy slot (read from its
/// [`PrevSlot`] bookkeeping, if any) and marks its NEW slot, then records the new
/// slot back into [`PrevSlot`]. The grid is edited **in place** via
/// [`OccupancyGrid::set_occupant`] — it is NEVER rebuilt (C6).
///
/// Behavior per entity:
/// - **Old slot:** if the entity has a [`PrevSlot`] AND that slot's current
///   occupant is this entity, clear it (`set_occupant(old, None)`). The occupant
///   guard means a move never clobbers a slot another entity has since taken.
/// - **New slot:** mark `set_occupant(new, Some(entity))` and write
///   `PrevSlot(new)`.
/// - **First sync (initial placement):** a freshly-inserted [`Position`] reads as
///   `Changed` on the first tick (Bevy first-run semantics) with no prior
///   [`PrevSlot`] — there is no old slot to clear, so the system simply marks the
///   new slot and records the [`PrevSlot`]. Initial placement is handled sanely.
///
/// `Commands` writes the [`PrevSlot`] bookkeeping; the occupant edits go straight
/// to the shared `ResMut<`[`OccupancyGrid`]`>`.
pub fn sync_moved_gangers(
    mut commands: Commands,
    mut grid: ResMut<OccupancyGrid>,
    moved: Query<(Entity, &Position, Option<&PrevSlot>), Changed<Position>>,
) {
    for (entity, position, prev) in &moved {
        let new_slot = **position;
        // Clear the OLD slot, but only if we still own it — a move must not stomp
        // a slot another entity has taken since (C3: clear OLD, mark NEW).
        if let Some(prev) = prev {
            let old_slot = prev.slot();
            if old_slot != new_slot && grid.occupant(&old_slot) == Some(entity) {
                grid.set_occupant(old_slot, None);
            }
        }
        // Mark the NEW slot and remember it for the next move.
        grid.set_occupant(new_slot, Some(entity));
        commands.entity(entity).insert(PrevSlot::new(new_slot));
    }
}

/// Maintain the occupancy grid for **downed / dead** gangers —
/// `Changed<`[`LifeState`]`>` filtered to non-[`LifeState::Alive`].
///
/// Reacts to Bevy change detection on [`LifeState`]: when an entity's life state
/// changes to [`LifeState::Downed`] or [`LifeState::Dead`], its occupant marker is
/// cleared from the slot it last synced to (read from [`PrevSlot`]). An entity
/// whose [`LifeState`] changed but is still [`LifeState::Alive`] is left alone (no
/// slot edit) — only going OUT frees the cell (C4). The grid is edited **in
/// place**; it is never rebuilt.
///
/// The slot cleared is the one in [`PrevSlot`] (the slot the move system last
/// synced this entity into); the occupant guard ensures only this entity's own
/// marker is cleared.
pub fn sync_dead_gangers(
    mut grid: ResMut<OccupancyGrid>,
    downed: Query<(Entity, &LifeState, &PrevSlot), Changed<LifeState>>,
) {
    for (entity, life, prev) in &downed {
        if matches!(life, LifeState::Alive) {
            continue;
        }
        let slot = prev.slot();
        if grid.occupant(&slot) == Some(entity) {
            grid.set_occupant(slot, None);
        }
    }
}

/// Maintain the occupancy grid for **destroyed cover** — reads the buffered
/// [`CoverDestroyed`] message and folds each into the grid's destroyed-cover set.
///
/// For every [`CoverDestroyed`] message buffered this frame, it calls
/// [`OccupancyGrid::mark_cover_destroyed`] with the message's [`CellLevel`], adding
/// the cell to the grid's append-only destroyed-cover set (so a smashed piece
/// stops blocking and can never resurrect). The [`MessageReader`] is the ONLY
/// trigger — no polling, no full-grid scan (C6). The grid is edited **in place**.
///
/// `CoverDestroyed` is a buffered **message** (Bevy 0.18 — `bevy-traps.md` #4),
/// hence [`MessageReader`], not the pre-0.18 `EventReader`.
pub fn sync_destroyed_cover(
    mut grid: ResMut<OccupancyGrid>,
    mut destroyed: MessageReader<CoverDestroyed>,
) {
    for event in destroyed.read() {
        grid.mark_cover_destroyed(event.at);
    }
}

/// Wires the three change-driven occupancy-maintenance systems and the
/// [`CoverDestroyed`] message buffer into a Bevy [`App`].
///
/// This is the **registration unit** for the E1.7 maintenance layer:
/// - it registers the [`CoverDestroyed`] message buffer
///   ([`App::add_message`]), without which [`sync_destroyed_cover`]'s
///   [`MessageReader`] would fail param validation; and
/// - it adds [`sync_moved_gangers`], [`sync_dead_gangers`], and
///   [`sync_destroyed_cover`] to [`Update`] in an **explicit
///   [`chain`](bevy::prelude::IntoScheduleConfigs::chain) order** — the three
///   share `ResMut<`[`OccupancyGrid`]`>`, so they MUST be ordered deterministically
///   (`bevy-traps.md` #3). The chain order (move → die → cover) is deliberate:
///   moves settle each entity's slot first, deaths then free a settled slot, and
///   cover folds into the independent destroyed-cover set last.
///
/// The production app adds this plugin when the sim is wired into the runtime
/// (E1.8 / E5) — that app-wiring is **out of scope** for GTW-157, so this plugin
/// is the registration the headless tests exercise (it is NOT unwired dead code).
#[derive(Debug, Default, Clone, Copy)]
pub struct OccupancyMaintenancePlugin;

impl Plugin for OccupancyMaintenancePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CoverDestroyed>().add_systems(
            Update,
            (sync_moved_gangers, sync_dead_gangers, sync_destroyed_cover).chain(),
        );
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::{App, MinimalPlugins};

    use super::*;
    use crate::{
        ganger::{LifeState, Position},
        metric::{Cell, CellLevel, Level},
        occupancy::OccupancyGrid,
    };

    fn key(x: i32, y: i32, level: u8) -> CellLevel {
        CellLevel::new(Cell::new(x, y), Level::new(level))
    }

    /// Build a headless app: `MinimalPlugins` (no window / renderer — C1), the
    /// full 60×60×8 [`OccupancyGrid`] resource, and the maintenance plugin (which
    /// registers the [`CoverDestroyed`] message + the three chained systems).
    fn headless_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(OccupancyGrid::new());
        app.add_plugins(OccupancyMaintenancePlugin);
        app
    }

    /// Read the grid resource out of the app world for assertions — `Option` so the
    /// test never `unwrap`s (the restriction lints fire in tests too).
    fn grid_occupant(app: &App, at: CellLevel) -> Option<Entity> {
        app.world()
            .get_resource::<OccupancyGrid>()
            .and_then(|g| g.occupant(&at))
    }

    fn cover_destroyed(app: &App, at: CellLevel) -> Option<bool> {
        app.world()
            .get_resource::<OccupancyGrid>()
            .map(|g| g.is_cover_destroyed(&at))
    }

    /// C9(a) — a ganger MOVES: one tick after mutating [`Position`], the OLD slot
    /// is cleared and the NEW slot is marked, WITHOUT any full-grid-rebuild call.
    ///
    /// Spawns a ganger at an initial cell, ticks once (initial placement marks the
    /// start slot via the first-run `Changed` semantics), then mutates `Position`
    /// to a new cell and ticks again. Asserts the start slot is now empty and the
    /// new slot holds the entity — the in-place clear-old + mark-new of C3. The
    /// grid is only ever maintained via the systems; `build_from_occupancy_input`
    /// is never called.
    #[test]
    fn moved_ganger_clears_old_slot_and_marks_new() {
        let mut app = headless_app();
        let start = key(5, 6, 0);
        let dest = key(9, 2, 1);

        let ganger = app
            .world_mut()
            .spawn((Position::new(start), LifeState::Alive))
            .id();

        // First tick: initial placement (Position reads as Changed on first run).
        app.update();
        assert_eq!(
            grid_occupant(&app, start),
            Some(ganger),
            "initial placement must mark the start slot",
        );

        // Move it: mutate Position, then tick once.
        if let Some(mut pos) = app.world_mut().get_mut::<Position>(ganger) {
            *pos = Position::new(dest);
        }
        app.update();

        assert_eq!(
            grid_occupant(&app, start),
            None,
            "the OLD slot must be cleared after a move (C3)",
        );
        assert_eq!(
            grid_occupant(&app, dest),
            Some(ganger),
            "the NEW slot must be marked after a move (C3)",
        );
    }

    /// C9(b) — a ganger DIES: flipping [`LifeState`] to [`LifeState::Dead`] and
    /// ticking once clears its occupant slot.
    ///
    /// Spawns + places a ganger, then flips its `LifeState` to `Dead` and ticks.
    /// The slot it occupied must be freed (C4). In place — no rebuild.
    #[test]
    fn dead_ganger_clears_its_slot() {
        let mut app = headless_app();
        let at = key(12, 13, 2);

        let ganger = app
            .world_mut()
            .spawn((Position::new(at), LifeState::Alive))
            .id();
        app.update();
        assert_eq!(
            grid_occupant(&app, at),
            Some(ganger),
            "the ganger must occupy its slot before death",
        );

        if let Some(mut life) = app.world_mut().get_mut::<LifeState>(ganger) {
            *life = LifeState::Dead;
        }
        app.update();

        assert_eq!(
            grid_occupant(&app, at),
            None,
            "a dead ganger's occupant slot must be cleared (C4)",
        );
    }

    /// A DOWNED ganger frees its slot too — C4 covers Downed and Dead alike (only
    /// non-Alive frees the cell).
    #[test]
    fn downed_ganger_clears_its_slot() {
        let mut app = headless_app();
        let at = key(20, 20, 0);

        let ganger = app
            .world_mut()
            .spawn((Position::new(at), LifeState::Alive))
            .id();
        app.update();

        if let Some(mut life) = app.world_mut().get_mut::<LifeState>(ganger) {
            *life = LifeState::Downed;
        }
        app.update();

        assert_eq!(
            grid_occupant(&app, at),
            None,
            "a downed ganger's occupant slot must be cleared (C4)",
        );
    }

    /// A `LifeState` change that stays [`LifeState::Alive`] does NOT free the slot —
    /// only going OUT (Downed / Dead) clears it (C4).
    #[test]
    fn still_alive_change_keeps_slot() {
        let mut app = headless_app();
        let at = key(7, 7, 1);

        let ganger = app
            .world_mut()
            .spawn((Position::new(at), LifeState::Alive))
            .id();
        app.update();

        // Touch LifeState (mark it changed) but leave it Alive.
        if let Some(mut life) = app.world_mut().get_mut::<LifeState>(ganger) {
            *life = LifeState::Alive;
        }
        app.update();

        assert_eq!(
            grid_occupant(&app, at),
            Some(ganger),
            "an Alive LifeState change must NOT free the slot (C4)",
        );
    }

    /// C9(c) — emitting a [`CoverDestroyed`] message and ticking once adds the cell
    /// to the grid's destroyed-cover set.
    ///
    /// Writes the message into the world buffer, ticks, and asserts the cell now
    /// reads destroyed (and an unrelated cell does not). In place — no rebuild.
    #[test]
    fn cover_destroyed_message_marks_the_cell() {
        let mut app = headless_app();
        let smashed = key(30, 31, 3);
        let intact = key(0, 0, 0);

        app.world_mut().write_message(CoverDestroyed::new(smashed));
        app.update();

        assert_eq!(
            cover_destroyed(&app, smashed),
            Some(true),
            "a CoverDestroyed message must mark its cell destroyed (C5)",
        );
        assert_eq!(
            cover_destroyed(&app, intact),
            Some(false),
            "an unrelated cell must not be marked destroyed",
        );
    }

    /// Two consecutive moves keep the grid consistent — the second move clears the
    /// FIRST destination (now the tracked previous slot), not the original start.
    /// Proves the [`PrevSlot`] bookkeeping advances with each move.
    #[test]
    fn two_moves_track_the_previous_slot() {
        let mut app = headless_app();
        let a = key(1, 1, 0);
        let b = key(2, 2, 0);
        let c = key(3, 3, 0);

        let ganger = app
            .world_mut()
            .spawn((Position::new(a), LifeState::Alive))
            .id();
        app.update();

        if let Some(mut pos) = app.world_mut().get_mut::<Position>(ganger) {
            *pos = Position::new(b);
        }
        app.update();

        if let Some(mut pos) = app.world_mut().get_mut::<Position>(ganger) {
            *pos = Position::new(c);
        }
        app.update();

        assert_eq!(grid_occupant(&app, a), None, "the original start is empty");
        assert_eq!(
            grid_occupant(&app, b),
            None,
            "the first destination is empty"
        );
        assert_eq!(
            grid_occupant(&app, c),
            Some(ganger),
            "only the latest destination holds the ganger",
        );
    }

    /// The [`PrevSlot`] newtype round-trips the slot it records — the bookkeeping
    /// the move system relies on.
    #[test]
    fn prev_slot_round_trips() {
        let slot = key(4, 5, 6);
        assert_eq!(PrevSlot::new(slot).slot(), slot);
    }
}
