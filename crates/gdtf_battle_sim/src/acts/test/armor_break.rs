use super::support::*;
use crate::{
    armor::{ArmorFloor, ArmorIntegrity, ArmorType, BodyPart, WornBy},
    armor_wear::{ArmorBroken, ArmorWearOutcome},
};

fn dress_all_parts(world: &mut World, ganger: Entity, integrity: ArmorIntegrity) {
    for part in BodyPart::ALL {
        world.spawn((
            WornBy::new(ganger),
            part,
            ArmorFloor::new(1),
            ArmorProtection::new(2),
            integrity,
            ArmorHardness::new(4),
            ArmorType::DEFAULT,
        ));
    }
}

fn drain_armor_broken(app: &mut App) -> Vec<ArmorBroken> {
    app.world_mut()
        .resource_mut::<Messages<ArmorBroken>>()
        .drain()
        .collect()
}

fn drain_shots_fired(app: &mut App) -> Vec<ShotFired> {
    app.world_mut()
        .resource_mut::<Messages<ShotFired>>()
        .drain()
        .collect()
}

fn wear_verdict_of(shots: &[ShotFired]) -> Option<ArmorWearOutcome> {
    shots
        .first()
        .and_then(|shot| shot.report.as_ref())
        .and_then(|report| match &report.verdict {
            HitVerdict::Ganger(verdict) => Some(verdict.applied.wear),
            _ => None,
        })
}

fn fire_once(app: &mut App, shooter: Entity) {
    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    app.update();
}

#[test]
fn broke_crossing_writes_exactly_one_armor_broken() {
    let (mut app, shooter, target) = fire_scenario();
    dress_all_parts(app.world_mut(), target, ArmorIntegrity::new(1));

    fire_once(&mut app, shooter);

    let shots = drain_shots_fired(&mut app);
    let Some(ArmorWearOutcome::Broke(expected)) = wear_verdict_of(&shots) else {
        unreachable!(
            "precondition: the seeded round lands on the near-broken piece and freezes a \
             Broke wear verdict, got {shots:?}"
        );
    };

    let breaks = drain_armor_broken(&mut app);
    assert_eq!(
        breaks.len(),
        1,
        "a Broke-crossing ganger verdict writes exactly one buffered ArmorBroken: {breaks:?}",
    );
    assert_eq!(
        breaks.first(),
        Some(&expected),
        "the buffered ArmorBroken is the verdict's own Broke payload (ganger + struck part), \
         never a recompute",
    );
    assert!(
        breaks.first().is_some_and(|broke| broke.ganger == target),
        "the ArmorBroken names the struck target ganger",
    );
}

#[test]
fn worn_reduction_writes_no_armor_broken() {
    let (mut app, shooter, target) = fire_scenario();
    dress_all_parts(app.world_mut(), target, ArmorIntegrity::new(10_000));

    fire_once(&mut app, shooter);

    let shots = drain_shots_fired(&mut app);
    assert!(
        matches!(wear_verdict_of(&shots), Some(ArmorWearOutcome::Damaged(_))),
        "precondition: the landed round WEARS the sturdy piece without breaking it \
         (a real reduction, not a bare-flesh no-op), got {shots:?}",
    );
    assert!(
        drain_armor_broken(&mut app).is_empty(),
        "a Damaged (reduced, still protecting) verdict writes NO ArmorBroken",
    );
}

#[test]
fn unaffected_hit_writes_no_armor_broken() {
    let (mut app, shooter, target) = fire_scenario();
    dress_all_parts(app.world_mut(), target, ArmorIntegrity::new(0));

    fire_once(&mut app, shooter);

    let shots = drain_shots_fired(&mut app);
    assert!(
        matches!(wear_verdict_of(&shots), Some(ArmorWearOutcome::Unaffected)),
        "precondition: a hit on an already-broken piece folds as bare flesh (Unaffected), \
         got {shots:?}",
    );
    assert!(
        drain_armor_broken(&mut app).is_empty(),
        "an Unaffected (already-broken / bare-flesh) verdict writes NO ArmorBroken — the \
         crossing never re-emits",
    );
}
