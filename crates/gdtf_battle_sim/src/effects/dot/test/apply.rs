use super::support::{
    App, DotAfflicted, DotApplied, Entity, Hp, IntoScheduleConfigs, LifeState, MessageReader,
    ResMut, Resource, Update, apply_app, apply_dot, dot, dot_of,
};


#[test]
fn apply_dot_refreshes_not_stacks() {
    let mut app = apply_app();
    let ganger = app.world_mut().spawn((Hp::new(50), LifeState::Alive)).id();

    app.world_mut()
        .write_message(DotApplied::new(ganger, dot(4, 2)));
    app.update();
    let first = dot_of(&app, ganger);
    assert_eq!(
        first,
        Some(dot(4, 2)),
        "the first DOT attach lands its full profile",
    );

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
    if let Some(d) = refreshed {
        assert_eq!(
            d.remaining_turns.get(),
            5,
            "the refreshed turns are the NEW profile's 5, never the stacked 2+5",
        );
    }
}


#[derive(Resource, Default, bevy::prelude::Deref, bevy::prelude::DerefMut)]
struct CapturedAfflicted(Vec<DotAfflicted>);

fn consume_afflicted(
    mut reader: MessageReader<DotAfflicted>,
    mut captured: ResMut<CapturedAfflicted>,
) {
    for afflicted in reader.read() {
        captured.push(*afflicted);
    }
}

fn afflicted_count_for(app: &App, ganger: Entity) -> usize {
    app.world()
        .get_resource::<CapturedAfflicted>()
        .map_or(0, |c| c.iter().filter(|a| a.ganger == ganger).count())
}

#[test]
fn apply_dot_emits_the_start_fact_once_on_attach_and_not_on_refresh() {
    let mut app = apply_app();
    app.init_resource::<CapturedAfflicted>();
    app.add_systems(Update, consume_afflicted.after(apply_dot));
    let ganger = app.world_mut().spawn((Hp::new(50), LifeState::Alive)).id();

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

    app.world_mut()
        .write_message(DotApplied::new(ganger, dot(9, 5)));
    app.update();
    assert_eq!(
        afflicted_count_for(&app, ganger),
        1,
        "a refresh is mid-affliction — it must emit NO new DotAfflicted start fact",
    );
}
