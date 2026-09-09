//! Stack-slot allocation for pops sharing a cell.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::prelude::CellLevel;

use super::text::{FctStackIndex, FloatingCombatText};

/// Cell a live floating combat text is anchored to.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Deref)]
pub struct FctAnchorCell(CellLevel);

impl FctAnchorCell {
    /// Anchor a pop to this cell.
    #[must_use]
    pub const fn new(at: CellLevel) -> Self {
        Self(at)
    }
}

/// Counts live pops per cell to assign the next stack index.
#[derive(SystemParam)]
pub struct FctSlotAllocator<'w, 's> {
    live_pops: Query<'w, 's, &'static FctAnchorCell, With<FloatingCombatText>>,
}

impl FctSlotAllocator<'_, '_> {
    /// Next free stack slot at `at`.
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
        prelude::{Color, Commands, IntoScheduleConfigs, ResMut, Resource, resource_exists},
        scene::ScenePlugin,
        time::TimeUpdateStrategy,
    };
    use cobalt_test_utils::unwatched_asset_plugin;
    use gdtf_battle_sim::prelude::{Cell, CellLevel, Level};

    use super::{super::text::spawn_floating_text, FctSlotAllocator};
    use crate::{
        CombatText, FctDrift, FctEmphasis, FctLabel, FctRiseRate, FctSlot, FctStackIndex,
        FctTtlSeconds, FloatingCombatText, animate_floating_text,
    };

    const TEST_TTL: FctTtlSeconds = FctTtlSeconds::new(0.15);

    const LONG_TTL: FctTtlSeconds = FctTtlSeconds::new(1_000.0);

    #[derive(Resource, Default)]
    struct AllocProbe {
        target:   Option<CellLevel>,
        ttl:      FctTtlSeconds,
        recorded: Vec<FctStackIndex>,
    }

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
        spawn_floating_text(
            &mut commands,
            FctLabel::new(CombatText::new("-7"), Color::WHITE, FctEmphasis::Normal),
            FctSlot::new(at, slot),
            FctDrift::new(FctRiseRate::default(), probe.ttl),
        );
    }

    fn alloc_app(delta: Duration, ttl: FctTtlSeconds) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, unwatched_asset_plugin(), ScenePlugin))
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

    fn request(app: &mut App, at: CellLevel) {
        if let Some(mut probe) = app.world_mut().get_resource_mut::<AllocProbe>() {
            probe.target = Some(at);
        }
    }

    fn recorded(app: &App) -> Vec<FctStackIndex> {
        app.world()
            .get_resource::<AllocProbe>()
            .map(|p| p.recorded.clone())
            .unwrap_or_default()
    }

    fn at(x: i32, y: i32, z: u8) -> CellLevel {
        CellLevel::new(Cell::new(x, y), Level::new(z))
    }

    fn pop_count(app: &mut App) -> usize {
        let mut q = app.world_mut().query::<&FloatingCombatText>();
        q.iter(app.world()).count()
    }

    #[test]
    fn pops_on_one_cell_across_frames_ascend() {
        let mut app = alloc_app(Duration::ZERO, LONG_TTL);
        let cell = at(3, 4, 0);

        request(&mut app, cell);
        app.update();
        assert_eq!(pop_count(&mut app), 1, "the first pop must materialize");

        request(&mut app, cell);
        app.update();
        assert_eq!(pop_count(&mut app), 2, "the second pop must materialize");

        assert_eq!(
            recorded(&app),
            vec![FctStackIndex::BASE, FctStackIndex::new(1)],
            "pops spawned on one cell across frames must get distinct ascending slots",
        );
    }

    #[test]
    fn a_despawned_pop_frees_its_slot() {
        let mut app = alloc_app(Duration::from_millis(100), TEST_TTL);
        let cell = at(5, 5, 0);

        request(&mut app, cell);
        app.update();
        request(&mut app, cell);
        app.update();
        assert_eq!(
            recorded(&app),
            vec![FctStackIndex::BASE, FctStackIndex::new(1)],
            "the first two pops must stack 0, 1",
        );

        for _ in 0..8 {
            app.update();
        }
        assert_eq!(
            pop_count(&mut app),
            0,
            "both pops must have despawned before the reuse allocation",
        );

        request(&mut app, cell);
        app.update();
        let slots = recorded(&app);
        assert_eq!(
            slots.last().copied(),
            Some(FctStackIndex::BASE),
            "after both pops despawn the freed slot is reused (index drops back to base), not leaked: {slots:?}",
        );
    }

    #[test]
    fn a_pop_mid_despawn_this_frame_is_not_counted() {
        let mut app = alloc_app(Duration::from_millis(100), TEST_TTL);
        let cell = at(7, 8, 0);

        request(&mut app, cell);
        app.update();
        assert_eq!(pop_count(&mut app), 1, "pop #1 must materialize");

        request(&mut app, cell);
        app.update();

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
