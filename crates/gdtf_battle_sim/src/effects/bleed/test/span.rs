use bevy::prelude::{Deref, DerefMut, MessageReader};

use super::support::{
    App, BleedingOut, Entity, IntoScheduleConfigs, LifeState, ResMut, Resource, Update, Wounds,
    bleed_app, bleed_rate,
};
use crate::effects::bleed::{BleedStarted, tick_bleed};

#[derive(Resource, Default, Deref, DerefMut)]
struct CapturedStarts(Vec<BleedStarted>);

fn consume_starts(mut reader: MessageReader<BleedStarted>, mut captured: ResMut<CapturedStarts>) {
    for started in reader.read() {
        captured.push(*started);
    }
}

fn span_app() -> App {
    let mut app = bleed_app();
    app.init_resource::<CapturedStarts>();
    app.add_systems(Update, consume_starts.after(tick_bleed));
    app
}

fn starts_for(app: &App, ganger: Entity) -> usize {
    app.world()
        .get_resource::<CapturedStarts>()
        .map_or(0, |c| c.iter().filter(|s| s.ganger == ganger).count())
}

#[test]
fn the_first_draining_tick_announces_once_and_mid_span_ticks_do_not() {
    let rate = bleed_rate();
    let start = rate.saturating_mul(6);

    let mut app = span_app();
    let ganger = app
        .world_mut()
        .spawn((Wounds::new(start), LifeState::Downed, BleedingOut))
        .id();

    app.update();
    assert_eq!(
        starts_for(&app, ganger),
        1,
        "the FIRST draining tick of a span emits exactly one BleedStarted",
    );

    app.update();
    app.update();
    assert_eq!(
        starts_for(&app, ganger),
        1,
        "mid-span draining ticks must emit NO further BleedStarted (once per span)",
    );
}

#[test]
fn a_halted_then_resumed_bleed_is_a_new_span_and_re_announces() {
    let rate = bleed_rate();
    let start = rate.saturating_mul(10);

    let mut app = span_app();
    let ganger = app
        .world_mut()
        .spawn((Wounds::new(start), LifeState::Downed, BleedingOut))
        .id();

    app.update();
    assert_eq!(starts_for(&app, ganger), 1, "span one announces once");

    app.world_mut().entity_mut(ganger).remove::<BleedingOut>();
    app.update();
    app.update();
    assert_eq!(
        starts_for(&app, ganger),
        1,
        "a stabilized (non-draining) ganger announces nothing",
    );

    app.world_mut().entity_mut(ganger).insert(BleedingOut);
    app.update();
    assert_eq!(
        starts_for(&app, ganger),
        2,
        "a resumed bleed is a NEW span and must re-announce (a second BleedStarted)",
    );
}
