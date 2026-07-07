//! The [`apply_dot`](crate::effects::dot::apply_dot) applier pins: refresh-not-stack
//! (a second penetrating DOT hit RESETS the affliction) and the GTW-572 once-per-span
//! [`DotAfflicted`](crate::effects::dot::DotAfflicted) start fact (attach emits,
//! refresh doesn't).

use super::support::{
    App, DotAfflicted, DotApplied, Entity, Hp, IntoScheduleConfigs, LifeState, MessageReader,
    ResMut, Resource, Update, apply_app, apply_dot, dot, dot_of,
};

// === (e) refresh-not-stack: a second DOT attach RESETS turns (does not stack). ===

#[test]
fn apply_dot_refreshes_not_stacks() {
    let mut app = apply_app();
    let ganger = app.world_mut().spawn((Hp::new(50), LifeState::Alive)).id();

    // First attach: 4 HP/turn for 2 turns.
    app.world_mut()
        .write_message(DotApplied::new(ganger, dot(4, 2)));
    app.update();
    let first = dot_of(&app, ganger);
    assert_eq!(
        first,
        Some(dot(4, 2)),
        "the first DOT attach lands its full profile",
    );

    // Second attach on the SAME ganger with DIFFERENT values: 9 HP/turn for 5 turns. The
    // affliction is RESET to the new profile — NOT stacked (turns are 5, not 2+5=7; damage is
    // 9, not 4+9).
    app.world_mut()
        .write_message(DotApplied::new(ganger, dot(9, 5)));
    app.update();
    let refreshed = dot_of(&app, ganger);
    assert_eq!(
        refreshed,
        Some(dot(9, 5)),
        "refresh-not-stack: a second DOT attach RESETS turns + per-turn damage to the new \
         profile (not additive)",
    );
    // Explicitly assert the turns did NOT accumulate.
    if let Some(d) = refreshed {
        assert_eq!(
            d.remaining_turns.get(),
            5,
            "the refreshed turns are the NEW profile's 5, never the stacked 2+5",
        );
    }
}

// === (f) GTW-572: the once-per-span DotAfflicted start fact — attach emits, refresh doesn't. ===

/// Captures every [`DotAfflicted`] start fact across the run (the `Captured` idiom — a
/// consumer system, NOT a raw buffer peek, so the count is cumulative and independent of the
/// double-buffer swap timing). Private inner (rule 5).
#[derive(Resource, Default, bevy::prelude::Deref, bevy::prelude::DerefMut)]
struct CapturedAfflicted(Vec<DotAfflicted>);

/// Drains the buffered [`DotAfflicted`] facts into [`CapturedAfflicted`] for assertion.
fn consume_afflicted(
    mut reader: MessageReader<DotAfflicted>,
    mut captured: ResMut<CapturedAfflicted>,
) {
    for afflicted in reader.read() {
        captured.push(*afflicted);
    }
}

/// The CUMULATIVE count of captured [`DotAfflicted`] facts for `ganger` across the run.
fn afflicted_count_for(app: &App, ganger: Entity) -> usize {
    app.world()
        .get_resource::<CapturedAfflicted>()
        .map_or(0, |c| c.iter().filter(|a| a.ganger == ganger).count())
}

/// GTW-572 (the Q2 ruling): a FRESH DOT attach emits exactly ONE [`DotAfflicted`] start
/// fact (carrying the per-turn drain), and a mid-affliction REFRESH emits NONE — so the
/// combat log's affliction line fires once at affliction start, never per applying hit.
#[test]
fn apply_dot_emits_the_start_fact_once_on_attach_and_not_on_refresh() {
    let mut app = apply_app();
    app.init_resource::<CapturedAfflicted>();
    app.add_systems(Update, consume_afflicted.after(apply_dot));
    let ganger = app.world_mut().spawn((Hp::new(50), LifeState::Alive)).id();

    // The fresh ATTACH — one start fact, carrying the profile's per-turn drain.
    app.world_mut()
        .write_message(DotApplied::new(ganger, dot(4, 2)));
    app.update();
    assert_eq!(
        afflicted_count_for(&app, ganger),
        1,
        "a fresh DOT attach emits exactly one DotAfflicted start fact",
    );
    let carried = app
        .world()
        .get_resource::<CapturedAfflicted>()
        .and_then(|c| c.iter().find(|a| a.ganger == ganger).map(|a| *a.per_turn));
    assert_eq!(
        carried,
        Some(4),
        "the start fact carries the attached profile's per-turn drain",
    );

    // A REFRESH of the already-afflicted ganger — mid-affliction, NO new start fact (the
    // cumulative capture stays at one).
    app.world_mut()
        .write_message(DotApplied::new(ganger, dot(9, 5)));
    app.update();
    assert_eq!(
        afflicted_count_for(&app, ganger),
        1,
        "a refresh is mid-affliction — it must emit NO new DotAfflicted start fact",
    );
}
