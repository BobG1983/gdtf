use bevy::{ecs::change_detection::Tick, prelude::*};

use super::draw_pool;

#[derive(Component)]
struct Pooled;

fn spawn_pooled(world: &mut World, x: f32, visibility: Visibility) -> Entity {
    world
        .spawn((Pooled, Transform::from_xyz(x, 0.0, 0.0), visibility))
        .id()
}

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

fn x_of(world: &World, entity: Entity) -> f32 {
    world
        .entity(entity)
        .get::<Transform>()
        .map_or(f32::NAN, |transform| transform.translation.x)
}

fn vis_of(world: &World, entity: Entity) -> Visibility {
    world
        .entity(entity)
        .get::<Visibility>()
        .copied()
        .unwrap_or(Visibility::Inherited)
}

fn vis_changed_tick(world: &World, entity: Entity) -> Option<Tick> {
    world
        .entity(entity)
        .get_change_ticks::<Visibility>()
        .map(|ticks| ticks.changed)
}

fn pooled_count(world: &mut World) -> usize {
    world
        .query_filtered::<(), With<Pooled>>()
        .iter(world)
        .count()
}

fn x_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.001
}

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

#[test]
fn singleton_option_draw_list_shows_and_hides_the_one_entity() {
    let mut world = World::new();
    let only = spawn_pooled(&mut world, 0.0, Visibility::Visible);

    walk(&mut world, None, &mut Vec::new());
    assert_eq!(
        vis_of(&world, only),
        Visibility::Hidden,
        "a `None` singleton draw hides the one pooled entity",
    );

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

    let mut empty_world = World::new();
    let mut grown: Vec<f32> = Vec::new();
    walk(&mut empty_world, Some(7.0), &mut grown);
    assert_eq!(
        grown,
        vec![7.0],
        "an empty pool with a `Some` singleton draw grows exactly once"
    );
}

#[test]
fn steady_walk_does_not_redirty_visibility_ticks() {
    let mut world = World::new();
    let shown = spawn_pooled(&mut world, 0.0, Visibility::Visible);
    let surplus = spawn_pooled(&mut world, 0.0, Visibility::Visible);
    let spawn_tick = vis_changed_tick(&world, surplus);

    world.increment_change_tick();
    walk(&mut world, [10.0], &mut Vec::new());
    let after_first_shown = vis_changed_tick(&world, shown);
    let after_first_surplus = vis_changed_tick(&world, surplus);
    assert_ne!(
        after_first_surplus, spawn_tick,
        "the first walk's real hide must bump the surplus Visibility change tick",
    );

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
