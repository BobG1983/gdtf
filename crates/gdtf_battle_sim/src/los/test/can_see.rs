//! AC for [`can_see`](crate::los::can_see), leaf 3 of the GTW-13 FOV epic (GTW-339):
//! the conscious-observer gate (Alive sees / Downed + Dead see nothing), the Chebyshev
//! range edge (`==` inclusive, `+1` excluded), and the LOS-blocked-inside-range case.
//! Reuses the clear + walled fixtures of the [`has_los`](crate::los::has_los) tests.

use super::support::*;

/// A clear, in-range standing observer→target on the same level along the x axis.
///
/// `from` at `(2, 5, 0)` looking East at `to` at `(8, 5, 0)` — open cells between, so
/// [`has_los`] reports CLEAR (it flies past the aim to a Miss). The cell `(x, y)`
/// Chebyshev distance is `max(|2-8|, |5-5|) = 6`, the relation the range tests pin.
fn clear_pair() -> (Position, Stance, Facing, Position, Stance) {
    (
        position(2, 5, 0),
        stance(StanceKind::Standing),
        facing(Direction::East),
        position(8, 5, 0),
        stance(StanceKind::Standing),
    )
}

/// The Chebyshev cell distance of [`clear_pair`] — derived from the fixture positions
/// (relation, not a pinned magnitude): `max(|dx|, |dy|)` over the cell `(x, y)`.
fn clear_pair_chebyshev() -> u16 {
    let (from_pos, _, _, to_pos, _) = clear_pair();
    let dx = (from_pos.x - to_pos.x).unsigned_abs();
    let dy = (from_pos.y - to_pos.y).unsigned_abs();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the test fixture deltas are tiny; this conversion is exact"
    )]
    let max = dx.max(dy) as u16;
    max
}

/// At Chebyshev distance `== view_range` with a clear line, `can_see` is `true` (the
/// range edge is inclusive per the `<=` spec — AC2).
#[test]
fn at_range_edge_with_clear_los_is_true() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let occupancy = OccupancyGrid::new();
    let (from_pos, from_stance, from_facing, to_pos, to_stance) = clear_pair();

    let observer = Observer {
        position: &from_pos,
        stance:   &from_stance,
        facing:   &from_facing,
    };
    let target = Target {
        position: &to_pos,
        stance:   &to_stance,
    };

    // View range EXACTLY equal to the pair's Chebyshev distance — the edge cell.
    let view_range = ViewRange::new(clear_pair_chebyshev());
    let verdict = can_see(
        &observer,
        &target,
        LifeState::Alive,
        view_range,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        *verdict,
        "at Chebyshev distance == view_range with clear LOS, can_see must be true (<= is inclusive)"
    );
}

/// At `view_range + 1` (i.e. one cell short of the pair's distance) the same clear
/// line is `false` — the disc edge is strict beyond `view_range` (AC2).
#[test]
fn beyond_range_edge_is_false() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let occupancy = OccupancyGrid::new();
    let (from_pos, from_stance, from_facing, to_pos, to_stance) = clear_pair();

    let observer = Observer {
        position: &from_pos,
        stance:   &from_stance,
        facing:   &from_facing,
    };
    let target = Target {
        position: &to_pos,
        stance:   &to_stance,
    };

    // The pair sits at distance d; view_range = d - 1 puts the target at d == range+1.
    let view_range = ViewRange::new(clear_pair_chebyshev() - 1);
    let verdict = can_see(
        &observer,
        &target,
        LifeState::Alive,
        view_range,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        !*verdict,
        "at Chebyshev distance == view_range + 1, can_see must be false (the disc edge is exclusive beyond range)"
    );
}

/// The conscious-observer gate: a `Downed` observer sees nothing even with a clear,
/// in-range line (AC1).
#[test]
fn downed_observer_sees_nothing() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let occupancy = OccupancyGrid::new();
    let (from_pos, from_stance, from_facing, to_pos, to_stance) = clear_pair();

    let observer = Observer {
        position: &from_pos,
        stance:   &from_stance,
        facing:   &from_facing,
    };
    let target = Target {
        position: &to_pos,
        stance:   &to_stance,
    };

    // Range generous, line clear — only the Downed life state should fail the gate.
    let view_range = ViewRange::new(clear_pair_chebyshev());
    let verdict = can_see(
        &observer,
        &target,
        LifeState::Downed,
        view_range,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        !*verdict,
        "a Downed observer must see nothing regardless of range / LOS"
    );
}

/// The conscious-observer gate: a `Dead` observer sees nothing even with a clear,
/// in-range line (AC1).
#[test]
fn dead_observer_sees_nothing() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let occupancy = OccupancyGrid::new();
    let (from_pos, from_stance, from_facing, to_pos, to_stance) = clear_pair();

    let observer = Observer {
        position: &from_pos,
        stance:   &from_stance,
        facing:   &from_facing,
    };
    let target = Target {
        position: &to_pos,
        stance:   &to_stance,
    };

    let view_range = ViewRange::new(clear_pair_chebyshev());
    let verdict = can_see(
        &observer,
        &target,
        LifeState::Dead,
        view_range,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        !*verdict,
        "a Dead observer must see nothing regardless of range / LOS"
    );
}

/// An `Alive` observer with the SAME clear, in-range line returns `true` — the
/// positive control proving the Downed/Dead cases fail on the LIFE gate, not the
/// fixture (AC1 + the both-hold case of AC3).
#[test]
fn alive_observer_in_range_with_clear_los_is_true() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let occupancy = OccupancyGrid::new();
    let (from_pos, from_stance, from_facing, to_pos, to_stance) = clear_pair();

    let observer = Observer {
        position: &from_pos,
        stance:   &from_stance,
        facing:   &from_facing,
    };
    let target = Target {
        position: &to_pos,
        stance:   &to_stance,
    };

    let view_range = ViewRange::new(clear_pair_chebyshev());
    let verdict = can_see(
        &observer,
        &target,
        LifeState::Alive,
        view_range,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        *verdict,
        "an Alive observer, in range, with clear LOS must see the target"
    );
}

/// `can_see` is `false` when [`has_los`](crate::los::has_los) is BLOCKED even though
/// the target is well inside range — the LOS-blocked-inside-range case (AC3). The
/// walled fixture mirrors the `has_los` `high_wall_between_blocks` test: a HIGH wall
/// midway between a standing eye and a standing target.
#[test]
fn blocked_los_inside_range_is_false() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let occupancy = OccupancyGrid::new();
    let mut cover = CoverLedger::new();
    // A HIGH wall strictly between the standing eye and the standing target (reused
    // walled fixture: a HIGH round is not strictly higher than a HIGH wall).
    cover.insert(key(5, 5, 0), cover_entry(HeightBand::High));

    let (from_pos, from_stance, from_facing, to_pos, to_stance) = clear_pair();
    let observer = Observer {
        position: &from_pos,
        stance:   &from_stance,
        facing:   &from_facing,
    };
    let target = Target {
        position: &to_pos,
        stance:   &to_stance,
    };

    // Generous range so ONLY the blocked LOS can fail the gate.
    let view_range = ViewRange::new(clear_pair_chebyshev() + 10);
    let verdict = can_see(
        &observer,
        &target,
        LifeState::Alive,
        view_range,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        !*verdict,
        "a blocked line of sight inside range must make can_see false"
    );
}

/// The Chebyshev range term ignores the level (z) axis (clause 3): a one-storey climb
/// between two cells at the SAME `(x, y)` is at Chebyshev distance `0`, so it passes
/// even a `view_range` of `0` — the z axis belongs to the LOS probe, never the disc.
#[test]
fn chebyshev_ignores_level_axis() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let occupancy = OccupancyGrid::new();

    // Same (x, y), one storey apart: the ONLY difference is z. A clear climb with no
    // slab is CLEAR, so the verdict turns purely on the range disc honoring z=0.
    let from_pos = position(4, 4, 0);
    let from_stance = stance(StanceKind::Standing);
    let from_facing = facing(Direction::North);
    let to_pos = position(4, 4, 1);
    let to_stance = stance(StanceKind::Standing);

    let observer = Observer {
        position: &from_pos,
        stance:   &from_stance,
        facing:   &from_facing,
    };
    let target = Target {
        position: &to_pos,
        stance:   &to_stance,
    };

    // view_range == 0: if z entered the Chebyshev max this would be distance 1 > 0 and
    // fail; because the disc is x/y only, the distance is 0 and the gate passes.
    let view_range = ViewRange::new(0);
    let verdict = can_see(
        &observer,
        &target,
        LifeState::Alive,
        view_range,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        *verdict,
        "the Chebyshev range disc must ignore the level/z axis (z is the LOS probe's)"
    );
}
