//! GTW-572 — the RANGED armor-broken **emission pin**: the fire boundary's exhaustive
//! verdict bridge (`emit_report_signals`) writes the buffered
//! [`ArmorBroken`](crate::armor_wear::ArmorBroken) fact **exactly once** per round whose
//! ganger verdict's §6 wear crossed the struck worn piece protecting→broken
//! ([`ArmorWearOutcome::Broke`]), and writes **nothing** for a
//! [`Damaged`](ArmorWearOutcome::Damaged) reduction or an
//! [`Unaffected`](ArmorWearOutcome::Unaffected) (already-broken / bare-flesh) hit —
//! driven end-to-end on the REAL `FireRequested` dispatch path (the same seeded in-line
//! scenario the landed-hit report tests pin as a guaranteed ganger hit).
//!
//! WHY a producer pin: the consumer-side `ArmorBroken` tests hand-write the message into
//! the buffer, so reverting the fire.rs bridge would leave the armor-broken pop + combat
//! log line dead in live play while the whole suite stayed green — the exact defect
//! GTW-572 named. This file fails on that revert.

use super::support::*;
use crate::{
    armor::{ArmorFloor, ArmorIntegrity, ArmorType, BodyPart, WornBy},
    armor_wear::{ArmorBroken, ArmorWearOutcome},
};

/// Dress `ganger` in ONE worn piece per body part at the given starting integrity —
/// whichever §4 part the seeded round strikes, the wear resolves against a worn piece.
/// Arbitrary non-shipped stats (the mechanism rule): `protection > 0` so the §5 wear
/// (`min(protection, damage) + effPen + shred·mult`) is strictly positive on any landed
/// hit. The `WornBy` insert hook populates the ganger's `Wears` synchronously in a bare
/// `World` spawn (the `spawn_shooter` `WieldedBy` precedent), so the very next dispatch
/// resolves the pieces.
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

/// Drain the buffered [`ArmorBroken`] messages emitted this run (one `update()`, then
/// probe — the fire module's `drain_shots_fired` precedent).
fn drain_armor_broken(app: &mut App) -> Vec<ArmorBroken> {
    app.world_mut()
        .resource_mut::<Messages<ArmorBroken>>()
        .drain()
        .collect()
}

/// Drain the buffered [`ShotFired`] messages emitted this run — the wear-verdict probe's
/// source (a module-local copy of the fire module's private helper).
fn drain_shots_fired(app: &mut App) -> Vec<ShotFired> {
    app.world_mut()
        .resource_mut::<Messages<ShotFired>>()
        .drain()
        .collect()
}

/// The wear outcome frozen on the run's single ganger verdict (via the `ShotFired`
/// report), else `None` — the precondition probe each case discriminates on.
fn wear_verdict_of(shots: &[ShotFired]) -> Option<ArmorWearOutcome> {
    shots
        .first()
        .and_then(|shot| shot.report.as_ref())
        .and_then(|report| match &report.verdict {
            HitVerdict::Ganger(verdict) => Some(verdict.applied.wear),
            _ => None,
        })
}

/// Fire the seeded guaranteed-hit single shot (the `fire_scenario` in-line target) and
/// run one update — the shared driver for the three wear-outcome cases.
fn fire_once(app: &mut App, shooter: Entity) {
    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    app.update();
}

// A NEAR-BROKEN worn piece (integrity 1): the landed round's positive wear
// (≥ min(protection, damage) = 2 here) crosses it protecting→broken, and the fire
// bridge writes EXACTLY ONE buffered ArmorBroken — byte-equal to the verdict's own
// Broke payload (pure exposure of the fold's frozen verdict, no recompute).
#[test]
fn broke_crossing_writes_exactly_one_armor_broken() {
    let (mut app, shooter, target) = fire_scenario();
    dress_all_parts(app.world_mut(), target, ArmorIntegrity::new(1));

    fire_once(&mut app, shooter);

    // Precondition: the seeded in-line round LANDED on the dressed ganger and its §6
    // wear verdict is the Broke crossing (discriminating — a Damaged/Unaffected fold here
    // would invalidate the case, not vacuously pass it).
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

// A STURDY worn piece (integrity far above any per-hit wear): the landed round REDUCES
// the piece (a Damaged verdict — a real reduction happened, so this discriminates from a
// bare-flesh no-op) and the bridge writes NO ArmorBroken.
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

// An ALREADY-BROKEN worn piece (integrity 0): the struck location folds as bare flesh —
// an Unaffected wear verdict (the crossing fired long ago; nothing left to wear) — and
// the bridge writes NO ArmorBroken (the emit-only-on-the-crossing rule).
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
