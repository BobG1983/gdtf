//! GTW-572 — the once-per-span [`BleedStarted`] affliction-start fact: the FIRST draining
//! tick of a span emits exactly ONE start fact, mid-span ticks emit none, and a HALTED
//! span (stabilized) followed by a fresh one re-announces.

use bevy::prelude::{Deref, DerefMut, MessageReader};

use super::support::{
    App, BleedingOut, Entity, IntoScheduleConfigs, LifeState, ResMut, Resource, Update, Wounds,
    bleed_app, bleed_rate,
};
use crate::effects::bleed::{BleedStarted, tick_bleed};

/// Captures the [`BleedStarted`] start facts a reader system drained (the sibling
/// `Captured`-for-`Bleeding` idiom — private inner, rule 5).
#[derive(Resource, Default, Deref, DerefMut)]
struct CapturedStarts(Vec<BleedStarted>);

/// Drains the buffered [`BleedStarted`] facts into [`CapturedStarts`] for assertion.
fn consume_starts(mut reader: MessageReader<BleedStarted>, mut captured: ResMut<CapturedStarts>) {
    for started in reader.read() {
        captured.push(*started);
    }
}

/// The shared bleed harness plus the start-fact capture, chained after the clock so the
/// captured facts reflect the same tick.
fn span_app() -> App {
    let mut app = bleed_app();
    app.init_resource::<CapturedStarts>();
    app.add_systems(Update, consume_starts.after(tick_bleed));
    app
}

/// Count the captured [`BleedStarted`] facts carrying a given ganger.
fn starts_for(app: &App, ganger: Entity) -> usize {
    app.world()
        .get_resource::<CapturedStarts>()
        .map_or(0, |c| c.iter().filter(|s| s.ganger == ganger).count())
}

/// GTW-572 (the Q2 ruling): the FIRST draining tick of a bleed span emits exactly ONE
/// [`BleedStarted`]; the following mid-span draining ticks emit NO further start fact —
/// the per-tick `Bleeding` signal keeps firing, but the affliction announces once.
#[test]
fn the_first_draining_tick_announces_once_and_mid_span_ticks_do_not() {
    let rate = bleed_rate();
    let start = rate.saturating_mul(6);

    let mut app = span_app();
    let ganger = app
        .world_mut()
        .spawn((Wounds::new(start), LifeState::Downed, BleedingOut))
        .id();

    // The first draining tick — the span starts, exactly one start fact.
    app.update();
    assert_eq!(
        starts_for(&app, ganger),
        1,
        "the FIRST draining tick of a span emits exactly one BleedStarted",
    );

    // Two more draining ticks — mid-span, NO further start fact.
    app.update();
    app.update();
    assert_eq!(
        starts_for(&app, ganger),
        1,
        "mid-span draining ticks must emit NO further BleedStarted (once per span)",
    );
}

/// GTW-572: a HALTED span (the ganger is stabilized — the clock stops draining) ENDS the
/// span; when the bleeding later resumes (the stabilization is removed) the fresh span
/// re-announces with a NEW [`BleedStarted`].
#[test]
fn a_halted_then_resumed_bleed_is_a_new_span_and_re_announces() {
    let rate = bleed_rate();
    let start = rate.saturating_mul(10);

    let mut app = span_app();
    let ganger = app
        .world_mut()
        .spawn((Wounds::new(start), LifeState::Downed, BleedingOut))
        .id();

    // Span one starts.
    app.update();
    assert_eq!(starts_for(&app, ganger), 1, "span one announces once");

    // Stabilize — REMOVE the BleedingOut condition; the clock halts, so the next tick
    // drains nothing and ENDS the span.
    app.world_mut().entity_mut(ganger).remove::<BleedingOut>();
    app.update();
    app.update();
    assert_eq!(
        starts_for(&app, ganger),
        1,
        "a stabilized (non-draining) ganger announces nothing",
    );

    // Un-stabilize — RE-INSERT the condition; the bleed resumes: a FRESH span, a NEW start fact.
    app.world_mut().entity_mut(ganger).insert(BleedingOut);
    app.update();
    assert_eq!(
        starts_for(&app, ganger),
        2,
        "a resumed bleed is a NEW span and must re-announce (a second BleedStarted)",
    );
}
