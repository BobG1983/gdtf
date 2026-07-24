//! The LIFETIME-AWARE floating-combat-text stacking-slot allocator (GTW-792) — the shared
//! primitive that hands a fresh pop the next free [`FctStackIndex`] ABOVE every pop still
//! ALIVE on its cell.
//!
//! Three spawn pipelines feed the FCT primitive (the consequence families, the shot damage
//! numbers, and the fall pop). Each USED to compute its stacking slot on its own — a per-frame
//! counter ([`FctStackCounter`](super::stack::FctStackCounter)), a per-shot local index, and
//! a hardcoded `0` respectively — so pops from DIFFERENT pipelines, or from different frames,
//! collided on the same cell. This module ships the ONE shared allocator that resolves the
//! collision at its root: it counts the pops CURRENTLY ALIVE on a cell and returns that count
//! as the new slot, so the next pop always lands one step above the live stack regardless of
//! which pipeline or frame spawned the others. All three pipelines now claim through it — the
//! consequence families (GTW-793) and the shot-damage + fall pops (GTW-794).
//!
//! Two properties the per-frame [`FctStackCounter`](super::stack::FctStackCounter) cannot give:
//!
//! - **Multi-frame.** A pop spawned last frame that has not yet despawned is still counted
//!   when a new pop is allocated this frame — the allocator reads the LIVE world, not a
//!   counter that resets every frame. So a slow trickle of pops on one cell across many
//!   frames still fans out instead of all reclaiming slot `0`.
//! - **Slot reuse.** When a pop despawns (its [`FctTtlSeconds`](super::super::tuning::FctTtlSeconds)
//!   lifetime elapses) the alive count drops, so the next pop reuses the now-lower index — the
//!   stack never leaks slots forever.
//!
//! ORDERING (`bevy-traps.md` #3): a consumer system that allocates through
//! [`FctSlotAllocator`] MUST run `.after(animate_floating_text)` — the system that despawns
//! expired pops. The despawn is a deferred [`Commands`](bevy::prelude::Commands) op, so the
//! explicit ordering edge inserts a command-flush sync point BEFORE the allocator's query
//! runs; a pop expiring THIS frame is already gone from the world when the allocator counts,
//! so it is never double-counted nor handed out a stale slot. Without the edge the allocator
//! could count a pop that is about to vanish and skip its freed slot.
//!
//! GTW-792 shipped the primitive; its three consumers were migrated onto it separately — the
//! consequence families (GTW-793) and the shot-damage + fall pipelines (GTW-794). Every pop
//! [`spawn_floating_text`](super::text::spawn_floating_text)
//! spawns already carries the [`FctAnchorCell`] this allocator queries (an inert marker until a
//! consumer reads it), so the migration is a drop-in — no change to the spawn primitive's
//! pixel-offset math.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::prelude::CellLevel;

use super::text::{FctStackIndex, FloatingCombatText};

/// The `(cell, level)` a live floating-combat-text pop is anchored over — the key
/// [`FctSlotAllocator`] groups pops by.
///
/// A NAMED newtype over the typed [`CellLevel`] (no-bare-types: an FCT anchor is a domain
/// value, not a bare coordinate), [`Deref`]ing to it.
/// [`spawn_floating_text`](super::text::spawn_floating_text) attaches one to every pop it
/// spawns so the allocator can count the pops on a cell without inferring the cell back out of
/// the pop's risen / stack-offset [`Transform`](bevy::prelude::Transform) (which is lossy).
/// Inert until a consumer reads it — nothing about a pop's rise / fade / despawn depends on it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Deref)]
pub struct FctAnchorCell(CellLevel);

impl FctAnchorCell {
    /// Record that a pop is anchored over `at`.
    #[must_use]
    pub const fn new(at: CellLevel) -> Self {
        Self(at)
    }
}

/// The lifetime-aware FCT stacking-slot allocator (GTW-792) — a read-only [`SystemParam`] over
/// every live pop's [`FctAnchorCell`].
///
/// A consumer system takes it as a param and calls [`next_slot`](Self::next_slot) to claim a
/// pop's stacking slot: the returned [`FctStackIndex`] is the count of pops CURRENTLY ALIVE on
/// the target cell, so the new pop stacks one step above them. Because it reads the live world
/// (not a per-frame counter) the count spans frames and frees despawned slots (see the module
/// doc).
///
/// A consuming system MUST be ordered `.after(animate_floating_text)` so a pop expiring this
/// frame is despawned — and its command flushed — before this query counts (module doc /
/// `bevy-traps.md` #3).
#[derive(SystemParam)]
pub struct FctSlotAllocator<'w, 's> {
    /// Every live pop's anchor cell — the set this allocator counts within.
    live_pops: Query<'w, 's, &'static FctAnchorCell, With<FloatingCombatText>>,
}

impl FctSlotAllocator<'_, '_> {
    /// Claim the next stacking slot for a pop about to spawn over `at`.
    ///
    /// Counts the pops still ALIVE on `at` and returns that count as the new
    /// [`FctStackIndex`] — [`FctStackIndex::BASE`] when the cell is empty, one step above the
    /// live stack otherwise. Frame-spanning (a pop from a previous frame that has not despawned
    /// is counted) and reuse-safe (a despawned pop's slot is freed, dropping the count back
    /// down).
    #[must_use]
    pub fn next_slot(&self, at: CellLevel) -> FctStackIndex {
        let alive = self
            .live_pops
            .iter()
            .filter(|&&anchor| *anchor == at)
            .count();
        FctStackIndex::new(alive)
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use bevy::{
        MinimalPlugins,
        app::{App, Update},
        asset::AssetPlugin,
        prelude::{Color, Commands, IntoScheduleConfigs, ResMut, Resource, resource_exists},
        scene::ScenePlugin,
        time::TimeUpdateStrategy,
    };
    use gdtf_battle_sim::prelude::{Cell, CellLevel, Level};

    // `spawn_floating_text` is not lifted to the crate root (fx-internal); reach it through
    // the sibling `text` module of this allocator's parent (`fct`).
    use super::super::text::spawn_floating_text;
    use super::FctSlotAllocator;
    use crate::{
        CombatText, FctEmphasis, FctRiseRate, FctStackIndex, FctTtlSeconds, FloatingCombatText,
        animate_floating_text,
    };

    /// A short pop lifetime for the reuse / mid-despawn tests, decoupled from the shipped
    /// default so the timing stays fast and is not a brittle magnitude pin.
    const TEST_TTL: FctTtlSeconds = FctTtlSeconds::new(0.15);

    /// A never-expiring pop lifetime for the ascending-slots test — pops spawned under a
    /// zero-delta clock never despawn, so the two frames' pops both stay alive to be counted.
    const LONG_TTL: FctTtlSeconds = FctTtlSeconds::new(1_000.0);

    /// Drives the allocator once per frame WHEN asked: on a frame with [`target`](Self::target)
    /// set, the probe system claims a slot for that cell (recording it) and spawns a real pop
    /// there via [`spawn_floating_text`]. The recorded slots are the tests' assertions.
    #[derive(Resource, Default)]
    struct AllocProbe {
        /// The cell to allocate + spawn on this frame, or [`None`] to idle (let live pops tick).
        target:   Option<CellLevel>,
        /// The lifetime the spawned pop is given (short to observe despawn, long to persist).
        ttl:      FctTtlSeconds,
        /// Every slot the allocator has handed out, in order — the test oracle.
        recorded: Vec<FctStackIndex>,
    }

    /// The consumer system (registered `.after(animate_floating_text)`): allocate + spawn on a
    /// requested frame. Ordering after the despawn system is what makes a pop expiring this
    /// frame invisible to the allocator's count (module doc / `bevy-traps.md` #3).
    fn probe_system(
        mut commands: Commands,
        allocator: FctSlotAllocator,
        mut probe: ResMut<AllocProbe>,
    ) {
        let Some(at) = probe.target.take() else {
            return;
        };
        let slot = allocator.next_slot(at);
        probe.recorded.push(slot);
        let (cell, level) = at.split();
        spawn_floating_text(
            &mut commands,
            CombatText::new("-7"),
            Color::WHITE,
            FctEmphasis::Normal,
            cell,
            level,
            slot,
            probe.ttl,
            FctRiseRate::default(),
        );
    }

    /// A headless app with the despawn animator + the allocator probe wired in the required
    /// order. `AssetPlugin` + `ScenePlugin` are needed because [`spawn_floating_text`] spawns
    /// the pop as a `bsn!` scene (the GTW-322 headless requirement).
    fn alloc_app(delta: Duration, ttl: FctTtlSeconds) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin))
            .insert_resource(TimeUpdateStrategy::ManualDuration(delta))
            .insert_resource(AllocProbe {
                ttl,
                ..Default::default()
            })
            .add_systems(
                Update,
                (
                    animate_floating_text,
                    probe_system.after(animate_floating_text),
                )
                    .run_if(resource_exists::<AllocProbe>),
            );
        app
    }

    /// Request an allocate + spawn over `at` on the next frame.
    fn request(app: &mut App, at: CellLevel) {
        if let Some(mut probe) = app.world_mut().get_resource_mut::<AllocProbe>() {
            probe.target = Some(at);
        }
    }

    /// The slots the allocator has handed out so far.
    fn recorded(app: &App) -> Vec<FctStackIndex> {
        app.world()
            .get_resource::<AllocProbe>()
            .map(|p| p.recorded.clone())
            .unwrap_or_default()
    }

    /// A cell key at `(x, y, z)`.
    fn at(x: i32, y: i32, z: u8) -> CellLevel {
        CellLevel::new(Cell::new(x, y), Level::new(z))
    }

    /// The number of live pops in the world.
    fn pop_count(app: &mut App) -> usize {
        let mut q = app.world_mut().query::<&FloatingCombatText>();
        q.iter(app.world()).count()
    }

    /// AC1: two pops spawned on ONE cell in DIFFERENT frames (the second after the first has
    /// ticked a frame but not despawned) receive DISTINCT, ASCENDING slots from the allocator.
    #[test]
    fn pops_on_one_cell_across_frames_ascend() {
        // A zero-delta clock: the LONG_TTL pops never despawn, so both frames' pops stay alive.
        let mut app = alloc_app(Duration::ZERO, LONG_TTL);
        let cell = at(3, 4, 0);

        // Frame 1: no live pops -> slot 0, spawn pop #1 (materializes on this frame's SpawnScene).
        request(&mut app, cell);
        app.update();
        assert_eq!(pop_count(&mut app), 1, "the first pop must materialize");

        // Frame 2: pop #1 is alive (ticked a frame, LONG_TTL) -> slot 1, spawn pop #2.
        request(&mut app, cell);
        app.update();
        assert_eq!(pop_count(&mut app), 2, "the second pop must materialize");

        assert_eq!(
            recorded(&app),
            vec![FctStackIndex::BASE, FctStackIndex::new(1)],
            "pops spawned on one cell across frames must get distinct ascending slots",
        );
    }

    /// AC2: a despawned pop FREES its slot — a later pop on that same cell reuses the now-lower
    /// index (the allocator does not leak slots forever).
    #[test]
    fn a_despawned_pop_frees_its_slot() {
        // 100ms ticks vs the 0.15s TEST_TTL: a couple of idle frames clear both pops.
        let mut app = alloc_app(Duration::from_millis(100), TEST_TTL);
        let cell = at(5, 5, 0);

        // Stack two pops: slot 0 then slot 1.
        request(&mut app, cell);
        app.update();
        request(&mut app, cell);
        app.update();
        assert_eq!(
            recorded(&app),
            vec![FctStackIndex::BASE, FctStackIndex::new(1)],
            "the first two pops must stack 0, 1",
        );

        // Idle until BOTH pops have despawned (0.15s TTL, 100ms ticks — several frames is ample).
        for _ in 0..8 {
            app.update();
        }
        assert_eq!(
            pop_count(&mut app),
            0,
            "both pops must have despawned before the reuse allocation",
        );

        // A later pop on the same cell reuses the freed base slot, not slot 2 — no leak.
        request(&mut app, cell);
        app.update();
        let slots = recorded(&app);
        assert_eq!(
            slots.last().copied(),
            Some(FctStackIndex::BASE),
            "after both pops despawn the freed slot is reused (index drops back to base), not leaked: {slots:?}",
        );
    }

    /// AC3: the allocator never counts a pop that is mid-despawn THIS frame. The probe runs
    /// `.after(animate_floating_text)`, so on the frame a pop's lifetime elapses it is despawned
    /// (and the command flushed at the ordering sync point) BEFORE the allocator counts — the
    /// expiring pop is excluded, and the new pop reuses its slot.
    #[test]
    fn a_pop_mid_despawn_this_frame_is_not_counted() {
        // TEST_TTL 0.15s with 100ms ticks: pop #1 is alive at 0.1s and expires at 0.2s.
        let mut app = alloc_app(Duration::from_millis(100), TEST_TTL);
        let cell = at(7, 8, 0);

        // Frame 1: spawn pop #1 at slot 0.
        request(&mut app, cell);
        app.update();
        assert_eq!(pop_count(&mut app), 1, "pop #1 must materialize");

        // Frame 2 (~0.1s elapsed): pop #1 still alive -> the allocator counts it, slot 1.
        request(&mut app, cell);
        app.update();

        // Frame 3 (~0.2s elapsed): animate_floating_text despawns the expired pop #1 THIS frame;
        // the .after ordering flushes that despawn before the allocator counts, so pop #1 is
        // excluded -> the new pop reuses slot 1 (only pop #2 remains alive), NOT slot 2.
        request(&mut app, cell);
        app.update();

        let slots = recorded(&app);
        assert_eq!(
            slots,
            vec![
                FctStackIndex::BASE,
                FctStackIndex::new(1),
                FctStackIndex::new(1),
            ],
            "the pop despawning this frame must not be counted (would give slot 2): {slots:?}",
        );
    }
}
