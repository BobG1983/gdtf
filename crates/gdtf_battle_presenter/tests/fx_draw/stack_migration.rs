//! GTW-794 — the shot-damage + fall FCT pops migrated onto the shared lifetime-aware
//! [`FctSlotAllocator`](gdtf_battle_presenter::FctSlotAllocator): both pipelines now claim their
//! stacking slot from the ONE allocator, so a shot pop, a fall pop, and a consequence pop
//! resolving on the SAME cell stack above one another instead of colliding on slot `0`.
//!
//! KNOWN STRUCTURAL CONSTRAINT (established by GTW-793, unchanged here): the allocator counts
//! only pops that have MATERIALIZED through Bevy's `SpawnScene` schedule, so it structurally
//! cannot see pops spawned earlier in the SAME `Update` frame (all of a frame's brand-new pops
//! are invisible to it until a LATER frame). So — exactly as GTW-793's own acceptance criteria
//! were redefined — these tests prove non-collision ACROSS CONSECUTIVE FRAMES: a pop alive from
//! frame N, a new pop on the same cell a frame (or a few fly-frames) later, gets a DISTINCT slot.
//! True same-frame cross-pipeline fan-out is NOT achievable on the live-pop allocator and is not
//! attempted; the multi-pop WITHIN a single shot still fans out internally (AC3), which needs no
//! materialization because it is a local per-shot index layered on the allocator base.

use std::time::Duration;

use bevy::math::Vec3;
use gdtf_battle_presenter::cell_to_world;
use gdtf_battle_sim::{
    armor::BodyPart,
    falls::{FallOccurred, StoreysFallen},
    prelude::{BattleInProgress, Cell, CellLevel, Level, LifeState, Position, SimPos},
    resolve_coarse::ShotKind,
    sample_cone::ShotDir,
    severity::Severity,
    shot_fired::ShotFired,
    suppression::SuppressionApplied,
    weapon::DamageType,
};

use super::{harness::*, probes::*};

/// The cell every migrated-pipeline pop in these tests resolves on, and its storey.
const CELL: Cell = Cell::new(7, 3);
/// The storey the pops resolve on.
const LEVEL: Level = Level::new(0);

/// Builds a connecting ganger-hit [`ShotFired`] on [`CELL`], flying from one cell west so it is a
/// short flight to its impact. `hp` / `pen` / `severity` / `life` shape how many pops the shot's
/// report classifies into (an HP number, a wound/graze, a penetration verdict, a DOWN/DEAD tag).
fn connecting_shot(
    app: &mut bevy::app::App,
    struck: bevy::ecs::entity::Entity,
    hp: i32,
    pen: i32,
    severity: Severity,
    life: LifeState,
) -> ShotFired {
    let report = ganger_hit_report(struck, BodyPart::Torso, hp, pen, severity, life);
    ShotFired {
        shooter:      app.world_mut().spawn_empty().id(),
        muzzle:       SimPos::new((CELL.x - 1) as f32, CELL.y as f32, 0.0),
        trajectory:   ShotDir::from_direction(Vec3::new(1.0, 0.0, 0.0)),
        impact_cell:  CELL,
        impact_level: LEVEL,
        kind:         ShotKind::Ganger(struck),
        damage:       DamageType::Kinetic,
        report:       Some(report),
    }
}

/// GTW-794 (acceptance 1) — a SHOT-DAMAGE pop and a CONSEQUENCE pop resolving on the SAME cell
/// across consecutive frames occupy DISTINCT stack slots: the shot's `"-7"` number, landing a
/// few fly-frames after a still-alive `"SUPPRESSED"` consequence pop, is seeded ABOVE it (slot
/// `>= 1`) instead of reclaiming slot `0` and overlapping.
///
/// The consequence pop spawns first (frame 1) and materializes; the shot then flies to its impact
/// over several frames, and its pops spawn only when the bolt lands — by which point the allocator
/// counts the still-alive suppression pop and hands the shot's numbers a base above it. Read on the
/// shot pop's SPAWN frame (via `step_until_pop`, before `animate_floating_text` rises it), so the
/// `"-7"` pop's world `y` is its unshifted spawn `y` — a direct readout of its slot.
///
/// PIN-DISCRIMINATING: the pre-GTW-794 shot pipeline seeded its per-shot fan-out at a hardcoded
/// local `0`, so the `"-7"` would have spawned AT `cell_to_world(cell).y` (slot 0), colliding with
/// the suppression pop's base slot. The assertion that `"-7"` sits strictly BELOW that base `y`
/// fails against the old hardcoded-0 seed and passes only when the allocator base is consulted.
#[test]
fn a_shot_pop_stacks_above_a_live_consequence_pop_on_the_same_cell() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let at = CellLevel::new(CELL, LEVEL);
    let struck = wounded_ganger(&mut app, CELL, LEVEL, 2);

    // FRAME 1: the suppression family pops "SUPPRESSED" on the cell (slot 0 — no live pops). It
    // materializes on this frame's SpawnScene schedule, so it is alive + counted from frame 2 on.
    play(&mut app, SuppressionApplied::new(struck, at));
    app.update();
    assert!(
        fct_pop_count(&mut app) >= 1,
        "the suppression consequence pop must be live before the shot is fired",
    );

    // Fire a connecting hit at the SAME cell (a 7-HP graze: pops "-7" + "Grazed" + "Armor held").
    let shot = connecting_shot(&mut app, struck, 7, 0, Severity::None, LifeState::Alive);
    play(&mut app, shot);
    fire_with_zero_delta(&mut app);

    // Fly the bolt to its impact, reading on the exact frame the "-7" shot pop materializes so its
    // y is its unshifted spawn y (the suppression pop is still alive, counted by the allocator).
    let snapshot = step_until_pop(&mut app, "-7", Duration::from_millis(30), 30);
    assert!(
        snapshot.is_some(),
        "the shot's \"-7\" damage pop must appear once its bolt lands",
    );
    let Some(pops) = snapshot else { return };

    let base_y = cell_to_world(CELL, LEVEL).y;
    let shot_y = pop_y_for(&pops, "-7");
    assert!(
        shot_y.is_some(),
        "the \"-7\" shot-damage pop must be present in the impact-frame snapshot: {pops:?}",
    );
    let Some(shot_y) = shot_y else { return };
    assert!(
        pops.iter().any(|(t, _)| t == "SUPPRESSED"),
        "the consequence pop must still be live on the cell when the shot lands: {pops:?}",
    );
    assert!(
        shot_y < base_y - 1.0,
        "the shot-damage pop must be seeded ABOVE the live consequence pop (a distinct slot >= 1, \
         so its spawn y sits below the slot-0 base y {base_y}), got {shot_y} — the pre-GTW-794 \
         hardcoded local-0 seed would have placed it AT the base y, colliding: {pops:?}",
    );
}

/// GTW-794 (acceptance 3) — a single shot's OWN multiple pops still fan out internally
/// (`0, 1, 2, …`), SEEDED above whatever the allocator says is already live: a lethal 4-pop shot
/// landing on a cell that already carries a live consequence pop spawns its four numbers at four
/// DISTINCT, ascending slots, ALL above the pre-existing pop's slot `0`.
///
/// All four of a shot's pops spawn on the ONE impact frame (a single `spawn_pops_at_anchor` call),
/// so they share an identical rise history — their world `y`s differ purely by stacking slot. Read
/// on that spawn frame: four distinct `y`s prove the internal fan-out survived the migration, and
/// all four sitting BELOW the slot-0 base `y` proves the per-shot index is seeded on the allocator
/// base (the live consequence pop), not reset to a local `0`.
#[test]
fn a_single_shots_multi_pop_fan_out_ascends_seeded_above_a_live_pop() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let at = CellLevel::new(CELL, LEVEL);
    let struck = wounded_ganger(&mut app, CELL, LEVEL, 2);

    // FRAME 1: a live consequence pop on the cell (slot 0), materialized before the shot.
    play(&mut app, SuppressionApplied::new(struck, at));
    app.update();

    // A lethal, penetrating, Critical torso hit: "-9", "Torso Critical", "Armor pierced", "DEAD".
    let shot = connecting_shot(&mut app, struck, 9, 6, Severity::Critical, LifeState::Dead);
    play(&mut app, shot);
    fire_with_zero_delta(&mut app);

    let snapshot = step_until_pop(&mut app, "-9", Duration::from_millis(30), 30);
    assert!(
        snapshot.is_some(),
        "the lethal shot's pops must appear once its bolt lands",
    );
    let Some(pops) = snapshot else { return };

    let base_y = cell_to_world(CELL, LEVEL).y;
    let shot_texts = ["-9", "Torso Critical", "Armor pierced", "DEAD"];
    let mut ys = Vec::new();
    for text in shot_texts {
        let maybe_y = pop_y_for(&pops, text);
        assert!(
            maybe_y.is_some(),
            "the shot must pop \"{text}\" at its impact: {pops:?}",
        );
        let Some(y) = maybe_y else { continue };
        assert!(
            y < base_y - 1.0,
            "every one of the shot's pops must be seeded ABOVE the live consequence pop (below the \
             slot-0 base y {base_y}); \"{text}\" landed at {y}, colliding with slot 0: {pops:?}",
        );
        ys.push(y);
    }
    // The four pops must occupy four DISTINCT slots (the internal per-shot fan-out still ascends).
    for i in 0..ys.len() {
        for j in (i + 1)..ys.len() {
            assert!(
                (ys[i] - ys[j]).abs() > 1.0,
                "a single shot's pops must fan out to DISTINCT stacking slots (distinct ys); \
                 \"{}\"={} and \"{}\"={} collided: {pops:?}",
                shot_texts[i],
                ys[i],
                shot_texts[j],
                ys[j],
            );
        }
    }
}

/// GTW-794 (acceptance 2) — two FALLS co-occurring on one cell (in consecutive frames) STACK:
/// the second `"Fell"` pop, landing the frame after a still-alive first `"Fell"` pop, takes a
/// distinct slot instead of both reclaiming the former hardcoded slot `0`.
///
/// A zero-delta clock throughout keeps the frame-1 pop at its spawn `y` (it never rises or
/// expires) so the two pops' `y`s differ purely by stacking slot. Read after frame 2: the two
/// `"Fell"` pops must sit at DISTINCT `y`s.
///
/// PIN-DISCRIMINATING: `read_fall_occurred` used to hand every fall a hardcoded
/// `FctStackIndex::new(0)`, so two falls on one cell rendered at the SAME `y`. With the allocator,
/// the frame-2 fall counts the still-alive frame-1 pop and takes slot 1 — the two `y`s DIFFER.
#[test]
fn two_falls_on_one_cell_across_consecutive_frames_stack() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);
    // A zero-delta clock: the frame-1 "Fell" pop never rises or expires, so it stays alive (and at
    // its spawn y) to be counted, and any y difference is purely the stack-slot offset.
    app.world_mut()
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::ZERO,
        ));

    let from_level = Level::new(2);
    // Two DIFFERENT fallers landing on the SAME cell (each carries its landing Position there).
    let faller_a = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(CELL, LEVEL)))
        .id();
    let faller_b = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(CELL, LEVEL)))
        .id();

    // FRAME 1: faller A's "Fell" pop (slot 0 — no live pops), materialized on this frame.
    play(
        &mut app,
        FallOccurred::new(faller_a, from_level, LEVEL, StoreysFallen::new(2)),
    );
    app.update();
    let after_first: Vec<_> = fct_pops_with_y(&mut app)
        .into_iter()
        .filter(|(t, _)| t == "Fell")
        .collect();
    assert_eq!(
        after_first.len(),
        1,
        "exactly one \"Fell\" pop must be live after the first fall, got {after_first:?}",
    );

    // FRAME 2: faller B's "Fell" pop on the SAME cell. A's pop is still alive (zero-delta clock),
    // so the allocator counts it and hands B's pop slot 1.
    play(
        &mut app,
        FallOccurred::new(faller_b, from_level, LEVEL, StoreysFallen::new(1)),
    );
    app.update();
    app.world_mut()
        .insert_resource(bevy::time::TimeUpdateStrategy::Automatic);

    let fell_ys: Vec<f32> = fct_pops_with_y(&mut app)
        .into_iter()
        .filter(|(t, _)| t == "Fell")
        .map(|(_, y)| y)
        .collect();
    assert_eq!(
        fell_ys.len(),
        2,
        "both falls' \"Fell\" pops must be live after the second fall, got {fell_ys:?}",
    );
    assert!(
        (fell_ys[0] - fell_ys[1]).abs() > 1.0,
        "two falls co-occurring on one cell must take DISTINCT stack slots (distinct ys), got \
         {fell_ys:?} — the pre-GTW-794 hardcoded slot 0 put both at the SAME y",
    );
}
