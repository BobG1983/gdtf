//! Tests for the `Pips` widget behavior (the `pips.rs` surface).
//!
//! Runs on the shared in-crate harness (see
//! [`test_support`](crate::widgets::core::test_support)). Each test is
//! pin-discriminating: it asserts MUTATE-in-place (stable pip entity ids across
//! an update) and the M-of-N remaining/lost color split.

use bevy::{ecs::system::SystemState, prelude::*, ui::BackgroundColor};

use super::{FilledPips, Pip, set_pips, spawn_pips};
use crate::widgets::core::test_support::{LOST, REMAINING, harness};

/// The two-query [`SystemState`] driving [`set_pips`] in tests (clippy
/// `type_complexity`).
type PipsSet = (
    Query<'static, 'static, &'static Children>,
    Query<'static, 'static, &'static mut BackgroundColor, With<Pip>>,
);

/// All pip entities of the row rooted at `row`, in child order.
fn pips_of(app: &mut App, row: Entity) -> Vec<Entity> {
    let mut state: SystemState<Query<&Children>> = SystemState::new(app.world_mut());
    let Ok(children) = state.get(app.world()) else {
        return Vec::new();
    };
    let Ok(kids) = children.get(row) else {
        return Vec::new();
    };
    kids.iter()
        .filter(|&c| app.world().get::<Pip>(c).is_some())
        .collect()
}

/// Drives [`set_pips`] once against the live world's queries.
fn drive_set_pips(
    app: &mut App,
    row: Entity,
    filled: FilledPips,
    remaining: Color,
    lost: Color,
) -> usize {
    let mut state: SystemState<PipsSet> = SystemState::new(app.world_mut());
    let Ok((children, mut pips)) = state.get_mut(app.world_mut()) else {
        return 0;
    };
    let n = set_pips(row, filled, remaining, lost, &children, &mut pips);
    state.apply(app.world_mut());
    n
}

/// AC — M-of-N pips carry the remaining vs lost color split.
#[test]
fn pips_render_m_of_n_split() {
    let mut app = harness();
    let row = {
        let mut commands = app.world_mut().commands();
        spawn_pips(&mut commands, 3, FilledPips::new(2), REMAINING, LOST, ())
    };
    app.world_mut().flush();

    let pips = pips_of(&mut app, row);
    assert_eq!(pips.len(), 3, "row must spawn exactly N pips");
    let colors: Vec<Color> = pips
        .iter()
        .map(|&p| app.world().get::<BackgroundColor>(p).map_or(LOST, |c| c.0))
        .collect();
    assert_eq!(
        colors,
        vec![REMAINING, REMAINING, LOST],
        "the first M pips are remaining, the rest lost",
    );
}

/// AC — updating M mutates the pip colors IN PLACE (stable pip entity ids).
#[test]
fn pips_update_mutates_same_entities() {
    let mut app = harness();
    let row = {
        let mut commands = app.world_mut().commands();
        spawn_pips(&mut commands, 3, FilledPips::new(0), REMAINING, LOST, ())
    };
    app.world_mut().flush();

    let before = pips_of(&mut app, row);
    assert_eq!(before.len(), 3);

    let n = drive_set_pips(&mut app, row, FilledPips::new(2), REMAINING, LOST);
    assert_eq!(n, 3, "set_pips must re-color every pip");

    let after = pips_of(&mut app, row);
    assert_eq!(
        after, before,
        "pip entity ids must be STABLE across the update (mutate, not respawn)",
    );
    let colors: Vec<Color> = after
        .iter()
        .map(|&p| app.world().get::<BackgroundColor>(p).map_or(LOST, |c| c.0))
        .collect();
    assert_eq!(
        colors,
        vec![REMAINING, REMAINING, LOST],
        "the new M-of-N split must be reflected after the update",
    );
}
