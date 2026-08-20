use gdtf_battle_sim::{
    acts::{MeleeStruck, MoveRequested},
    ganger::{Direction, Faction, Stance, StanceKind},
    resolve_hit::HpDamage,
    test_support::SituationBuilder,
};

use super::harness::*;

/// The gang the player commands in these cases.
const fn player() -> Faction {
    Faction::new(PLAYER)
}

/// The gang standing outside the squad's field of view in these cases.
const fn enemy() -> Faction {
    Faction::new(ENEMY)
}

// Change a ganger's stance, which the log records as a posture act on its own cell.
fn crouch(app: &mut bevy::app::App, entity: bevy::prelude::Entity) {
    if let Some(mut stance) = app.world_mut().get_mut::<Stance>(entity) {
        *stance = Stance::new(StanceKind::Crouching);
    }
}

#[test]
fn an_entry_keeps_the_observers_it_had_when_the_fog_moves_off_it() {
    let mut app = battle_app(forced_reaction_tuning(0));
    let watch = ground(5, 5);
    let seen = ground(8, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            watcher(watch, PLAYER, Direction::East),
            tough_mover(seen, ENEMY, Direction::West),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(eyes), Some(watched)) = (ganger_at(&mut app, watch), ganger_at(&mut app, seen))
    else {
        unreachable!("setup spawns the watcher and the watched ganger at their fixture cells");
    };
    assert!(
        squad_sees(&app, seen),
        "precondition: the squad must be able to see {seen:?} before the act is logged",
    );

    crouch(&mut app, watched);
    settle(&mut app);

    let logged = logged_of(&app, "PostureChanged");
    let Some(fact) = logged.iter().find(|fact| fact.actor == watched) else {
        unreachable!("a stance change on a spawned ganger appends a posture entry: {logged:?}");
    };
    let seq = fact.seq;
    assert!(
        *fact.witnesses.observed_by(player()),
        "the act happened on a cell the squad was watching: entry {seq:?} recorded \
         {witnesses:?}",
        witnesses = fact.witnesses,
    );

    stand_at(&mut app, eyes, ground(40, 40));
    settle(&mut app);
    assert!(
        !squad_sees(&app, seen),
        "precondition: walking the squad away must leave {seen:?} unlit",
    );

    let after = logged_of(&app, "PostureChanged");
    let Some(again) = after.iter().find(|fact| fact.seq == seq) else {
        unreachable!("the entry is still retained: {after:?}");
    };
    assert!(
        *again.witnesses.observed_by(player()),
        "history is fixed — entry {seq:?} still records the squad as having watched it, \
         whatever the fog does afterwards; found {witnesses:?}",
        witnesses = again.witnesses,
    );
}

#[test]
fn an_enemy_gang_observes_an_act_the_player_squad_cannot_see() {
    let mut app = battle_app(forced_reaction_tuning(0));
    let watch = ground(5, 5);
    let hidden = ground(30, 30);
    let situation = SituationBuilder::new()
        .with_gangers([
            watcher(watch, PLAYER, Direction::East),
            tough_mover(hidden, ENEMY, Direction::West),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let Some(unseen) = ganger_at(&mut app, hidden) else {
        unreachable!("setup spawns the hidden ganger at its fixture cell");
    };
    assert!(
        !squad_sees(&app, hidden),
        "precondition: {hidden:?} stands far outside the squad's view range",
    );

    crouch(&mut app, unseen);
    settle(&mut app);

    let logged = logged_of(&app, "PostureChanged");
    let Some(fact) = logged.iter().find(|fact| fact.actor == unseen) else {
        unreachable!("a stance change appends a posture entry: {logged:?}");
    };
    assert!(
        *fact.witnesses.observed_by(enemy()),
        "only the player gang has a fog, so every other fielded gang observes every act: \
         entry {seq:?} recorded {witnesses:?}",
        seq = fact.seq,
        witnesses = fact.witnesses,
    );
    assert!(
        !*fact.witnesses.observed_by(player()),
        "the player squad could not see {hidden:?}, so it must not be recorded as watching: \
         {witnesses:?}",
        witnesses = fact.witnesses,
    );
}

#[test]
fn a_hit_the_squad_watches_land_is_reported_without_naming_the_attacker() {
    let mut app = battle_app(forced_reaction_tuning(0));
    let watch = ground(5, 5);
    let hidden = ground(30, 30);
    let situation = SituationBuilder::new()
        .with_gangers([
            watcher(watch, PLAYER, Direction::East),
            tough_mover(hidden, ENEMY, Direction::West),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(target), Some(attacker)) = (ganger_at(&mut app, watch), ganger_at(&mut app, hidden))
    else {
        unreachable!("setup spawns the watcher and the attacker at their fixture cells");
    };
    assert!(
        squad_sees(&app, watch) && !squad_sees(&app, hidden),
        "precondition: the squad watches {watch:?} and cannot see {hidden:?}",
    );

    app.world_mut()
        .write_message(MeleeStruck::new(attacker, target, HpDamage::new(1)));
    settle(&mut app);

    let struck = logged_of(&app, "Struck");
    let Some(fact) = struck.first() else {
        unreachable!("a melee strike appends a Struck entry: {struck:?}");
    };
    assert!(
        *fact.witnesses.observed_by(player()),
        "a Struck entry is keyed on the cell the hit LANDED on, not the cell the attacker \
         stood on: recorded {witnesses:?}",
        witnesses = fact.witnesses,
    );
    assert!(
        !*fact.witnesses.identifies_actor(player()),
        "the attacker stood in the dark, so the entry names nobody: {witnesses:?}",
        witnesses = fact.witnesses,
    );
}

#[test]
fn a_magazine_change_is_keyed_on_the_wielder_rather_than_the_gun() {
    let mut app = battle_app(forced_reaction_tuning(0));
    let watch = ground(5, 5);
    let situation = SituationBuilder::new()
        .with_gangers([watcher(watch, PLAYER, Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let Some(wielder) = ganger_at(&mut app, watch) else {
        unreachable!("setup spawns the wielder at its fixture cell");
    };
    assert!(
        squad_sees(&app, watch),
        "precondition: the squad watches the cell its own ganger stands on",
    );

    load_magazine(&mut app, wielder, 7);
    settle(&mut app);

    let changed = logged_of(&app, "MagazineChanged");
    let Some(fact) = changed.first() else {
        unreachable!("reloading a wielded gun appends a MagazineChanged entry: {changed:?}");
    };
    assert!(
        *fact.witnesses.observed_by(player()),
        "the entry's actor is the GUN, which stands on no cell — the witnesses come from \
         the wielder's cell instead: recorded {witnesses:?}",
        witnesses = fact.witnesses,
    );
}

#[test]
fn a_hidden_ganger_coming_into_view_appends_one_entered_view_entry() {
    let mut app = battle_app(forced_reaction_tuning(0));
    let watch = ground(5, 5);
    let hidden = ground(14, 5);
    let situation = SituationBuilder::new()
        .with_gangers([
            watcher(watch, PLAYER, Direction::East),
            tough_mover(hidden, ENEMY, Direction::West),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let (Some(eyes), Some(lurker)) = (ganger_at(&mut app, watch), ganger_at(&mut app, hidden))
    else {
        unreachable!("setup spawns the watcher and the lurker at their fixture cells");
    };
    assert!(
        !squad_sees(&app, hidden),
        "precondition: {hidden:?} starts outside the squad's view range",
    );
    assert!(
        logged_of(&app, "EnteredView").is_empty(),
        "precondition: nothing has come into view yet",
    );

    set_tu(&mut app, eyes, u8::MAX);
    app.world_mut()
        .write_message(MoveRequested::new(eyes, ground(10, 5)));
    for _ in 0..24 {
        app.update();
    }

    let entered: Vec<_> = logged_of(&app, "EnteredView")
        .into_iter()
        .filter(|fact| fact.actor == lurker)
        .collect();
    assert_eq!(
        entered.len(),
        1,
        "a hidden ganger coming into view appends exactly one entry, not one a frame — got \
         {count}: {entered:?}",
        count = entered.len(),
    );
    let Some(fact) = entered.first() else {
        unreachable!("the assertion above requires exactly one entry");
    };
    assert!(
        *fact.witnesses.observed_by(player()),
        "the squad is what brought it into view, so the squad observes the entry: \
         {witnesses:?}",
        witnesses = fact.witnesses,
    );
}

#[test]
fn a_walk_records_exactly_one_moved_to_entry() {
    let mut app = battle_app(forced_reaction_tuning(0));
    let start = ground(5, 5);
    let situation = SituationBuilder::new()
        .with_gangers([watcher(start, PLAYER, Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, situation);

    let Some(walker) = ganger_at(&mut app, start) else {
        unreachable!("setup spawns the mover at its fixture cell");
    };
    set_tu(&mut app, walker, u8::MAX);
    app.world_mut()
        .write_message(MoveRequested::new(walker, ground(9, 5)));
    for _ in 0..24 {
        app.update();
    }

    let arrivals: Vec<_> = logged_of(&app, "MovedTo")
        .into_iter()
        .filter(|fact| fact.actor == walker)
        .collect();
    let stepped = logged_of(&app, "Stepped").len();
    assert_eq!(
        arrivals.len(),
        1,
        "a four-cell walk logs one completed move against {stepped} steps — got {count} \
         MovedTo entries: {arrivals:?}",
        count = arrivals.len(),
    );
    assert!(
        stepped >= 4,
        "the fixture must actually walk, or the count above proves nothing — got {stepped} \
         steps",
    );
}
