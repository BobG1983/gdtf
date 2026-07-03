//! Unit tests for the shared pooled-draw walk (GTW-568): reuse-in-order,
//! grow-when-exhausted, grow-may-decline, hide-surplus, the 0/1-length singleton path,
//! and the `set_if_neq` no-re-dirty guarantee.
//!
//! `draw_pool` is a pure walk over a pooled query's items, not a registered system, so
//! these drive it over a bare [`World`] + [`QueryState`](bevy::ecs::query::QueryState)
//! (the `bevy-traps.md` #7 carve-out (b): a unit test exercising the helper in isolation,
//! where an app harness is genuinely inapplicable — the real query [`Mut`] items are the
//! whole point). The REAL migrated draw systems are proven by the existing headless
//! integration tests plus the GTW-568 `tests/pool_redirty.rs` regression.

use bevy::{ecs::change_detection::Tick, prelude::*};

use super::draw_pool;

/// Marker for a pooled test entity (mirrors the per-overlay pooled-sprite markers).
#[derive(Component)]
struct Pooled;

/// Spawn one pooled test entity at x = `x` with `visibility`.
fn spawn_pooled(world: &mut World, x: f32, visibility: Visibility) -> Entity {
    world
        .spawn((Pooled, Transform::from_xyz(x, 0.0, 0.0), visibility))
        .id()
}

/// Run the standard test walk over the [`Pooled`] entities: `show` writes the draw into
/// `translation.x`, `grow` RECORDS the overflow draw into `grown` and spawns nothing
/// (i.e. it declines — the caller-opaque spawn is exercised by what lands in `grown`),
/// and the helper owns both visibility flips through the tuple's visibility projection.
fn walk(world: &mut World, draws: impl IntoIterator<Item = f32>, grown: &mut Vec<f32>) {
    let mut query = world.query_filtered::<(&mut Transform, &mut Visibility), With<Pooled>>();
    draw_pool(
        query.iter_mut(world),
        draws,
        |x, (transform, _)| transform.translation.x = x,
        |x| grown.push(x),
        |(_, visibility)| visibility,
    );
}

/// The pooled entity's `translation.x` — the per-overlay "show wrote here" witness
/// (`NAN` if the entity lost its `Transform`, which no test expects).
fn x_of(world: &World, entity: Entity) -> f32 {
    world
        .entity(entity)
        .get::<Transform>()
        .map_or(f32::NAN, |transform| transform.translation.x)
}

/// The pooled entity's `Visibility` (`Inherited` if absent, which no test expects).
fn vis_of(world: &World, entity: Entity) -> Visibility {
    world
        .entity(entity)
        .get::<Visibility>()
        .copied()
        .unwrap_or(Visibility::Inherited)
}

/// The pooled entity's `Visibility` CHANGED tick — the re-dirty witness for the
/// `set_if_neq` tests.
fn vis_changed_tick(world: &World, entity: Entity) -> Option<Tick> {
    world
        .entity(entity)
        .get_change_ticks::<Visibility>()
        .map(|ticks| ticks.changed)
}

/// Count of live [`Pooled`] entities — the never-despawn witness.
fn pooled_count(world: &mut World) -> usize {
    world
        .query_filtered::<(), With<Pooled>>()
        .iter(world)
        .count()
}

/// `x`-equality within float noise (the test writes exact literals).
fn x_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.001
}

/// Reuse-in-order: with enough pooled entities, each draw item mutates the NEXT pooled
/// entity in iteration order and shows it; `grow` is never called.
#[test]
fn reuses_pooled_entities_in_iteration_order() {
    let mut world = World::new();
    let first = spawn_pooled(&mut world, 0.0, Visibility::Hidden);
    let second = spawn_pooled(&mut world, 0.0, Visibility::Hidden);

    let mut grown: Vec<f32> = Vec::new();
    walk(&mut world, [10.0, 20.0], &mut grown);

    assert!(
        grown.is_empty(),
        "two draws over two pooled entities need no growth"
    );
    assert!(
        x_eq(x_of(&world, first), 10.0) && x_eq(x_of(&world, second), 20.0),
        "each pooled entity takes its same-rank draw (reuse in iteration order)",
    );
    assert_eq!(
        (vis_of(&world, first), vis_of(&world, second)),
        (Visibility::Visible, Visibility::Visible),
        "every reused pooled entity is shown by the helper",
    );
}

/// Grow-when-exhausted: draws beyond the pool size are handed to `grow` in order, AFTER
/// the pool is used up.
#[test]
fn grows_lazily_when_the_pool_is_exhausted() {
    let mut world = World::new();
    let only = spawn_pooled(&mut world, 0.0, Visibility::Hidden);

    let mut grown: Vec<f32> = Vec::new();
    walk(&mut world, [10.0, 20.0, 30.0], &mut grown);

    assert!(
        x_eq(x_of(&world, only), 10.0),
        "the one pooled entity takes the first draw before any growth",
    );
    assert_eq!(
        grown,
        vec![20.0, 30.0],
        "draws past the pool size go to `grow`, in order"
    );
}

/// Grow-may-decline: `grow` is an opaque per-overlay spawn — a caller that declines
/// (e.g. the vertical-link draw while its atlas is loading) spawns nothing, and the walk
/// still offers it EVERY overflow draw without panicking or requiring a pool entity.
#[test]
fn grow_may_decline_without_disturbing_the_walk() {
    let mut world = World::new();

    let mut offered: Vec<f32> = Vec::new();
    walk(&mut world, [1.0, 2.0], &mut offered);

    assert_eq!(
        offered,
        vec![1.0, 2.0],
        "every overflow draw is offered to `grow` once, in order"
    );
    assert_eq!(
        pooled_count(&mut world),
        0,
        "a declining `grow` leaves the pool exactly as it was (nothing spawned)",
    );
}

/// Hide-surplus: pooled entities beyond the draw list are hidden — never despawned — and
/// a previously-hidden reused entity is shown.
#[test]
fn hides_every_surplus_pooled_entity() {
    let mut world = World::new();
    let shown = spawn_pooled(&mut world, 0.0, Visibility::Hidden);
    let surplus_a = spawn_pooled(&mut world, 0.0, Visibility::Visible);
    let surplus_b = spawn_pooled(&mut world, 0.0, Visibility::Hidden);

    walk(&mut world, [10.0], &mut Vec::new());

    assert_eq!(
        vis_of(&world, shown),
        Visibility::Visible,
        "the reused pooled entity is shown"
    );
    assert_eq!(
        vis_of(&world, surplus_a),
        Visibility::Hidden,
        "a visible surplus pooled entity is hidden",
    );
    assert_eq!(
        vis_of(&world, surplus_b),
        Visibility::Hidden,
        "an already-hidden surplus pooled entity stays hidden",
    );
    assert_eq!(
        pooled_count(&mut world),
        3,
        "surplus pooled entities are hidden, NEVER despawned",
    );
}

/// The 0/1-length singleton path (the fire-target tile / cost-label / path target-label
/// shape): `Some` shows and moves the one pooled entity; `None` hides it via the surplus
/// sweep; an empty pool with `Some` grows exactly once.
#[test]
fn singleton_option_draw_list_shows_and_hides_the_one_entity() {
    let mut world = World::new();
    let only = spawn_pooled(&mut world, 0.0, Visibility::Visible);

    // `None` → the one pooled entity is surplus → hidden (hidden-when-absent).
    walk(&mut world, None, &mut Vec::new());
    assert_eq!(
        vis_of(&world, only),
        Visibility::Hidden,
        "a `None` singleton draw hides the one pooled entity",
    );

    // `Some` → the one pooled entity is reused: moved + shown.
    walk(&mut world, Some(5.0), &mut Vec::new());
    assert_eq!(
        vis_of(&world, only),
        Visibility::Visible,
        "a `Some` singleton draw shows the one pooled entity",
    );
    assert!(
        x_eq(x_of(&world, only), 5.0),
        "a `Some` singleton draw moves the one pooled entity"
    );

    // An EMPTY pool with `Some` grows exactly once (the lazy first spawn).
    let mut empty_world = World::new();
    let mut grown: Vec<f32> = Vec::new();
    walk(&mut empty_world, Some(7.0), &mut grown);
    assert_eq!(
        grown,
        vec![7.0],
        "an empty pool with a `Some` singleton draw grows exactly once"
    );
}

/// GTW-568 C2 (unit-level) — a steady walk with unchanged draws leaves BOTH the
/// already-visible shown entity's and the already-hidden surplus entity's `Visibility`
/// change ticks untouched (`set_if_neq` on both flips), while the FIRST walk's real
/// hide DID bump the surplus tick (so the harness provably detects writes).
#[test]
fn steady_walk_does_not_redirty_visibility_ticks() {
    let mut world = World::new();
    let shown = spawn_pooled(&mut world, 0.0, Visibility::Visible);
    let surplus = spawn_pooled(&mut world, 0.0, Visibility::Visible);
    let spawn_tick = vis_changed_tick(&world, surplus);

    // First walk: `shown` is reused (already Visible → no flip), `surplus` is hidden
    // (Visible → Hidden, a REAL write).
    world.increment_change_tick();
    walk(&mut world, [10.0], &mut Vec::new());
    let after_first_shown = vis_changed_tick(&world, shown);
    let after_first_surplus = vis_changed_tick(&world, surplus);
    // Harness sanity: the first walk's genuine Visible→Hidden write IS visible to the
    // tick witness (guards against a vacuously-passing steady assertion below).
    assert_ne!(
        after_first_surplus, spawn_tick,
        "the first walk's real hide must bump the surplus Visibility change tick",
    );

    // Steady walk at a NEWER world tick with the SAME draws: set_if_neq must no-op on
    // both the show-flip (already Visible) and the surplus-hide (already Hidden).
    world.increment_change_tick();
    walk(&mut world, [10.0], &mut Vec::new());

    assert_eq!(
        vis_changed_tick(&world, shown),
        after_first_shown,
        "a steady frame leaves the already-visible shown entity's Visibility ticks untouched",
    );
    assert_eq!(
        vis_changed_tick(&world, surplus),
        after_first_surplus,
        "a steady frame leaves the already-hidden surplus entity's Visibility ticks untouched",
    );
}
