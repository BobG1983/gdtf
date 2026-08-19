use super::support::*;
use crate::{
    los::{Observer, PeekOffset, Target, has_los},
    march::MarchGrids,
    occupancy::StairEyeOffset,
    pathfinder::Path,
};

fn on(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

fn one_step(start: CellLevel, dest: CellLevel) -> Path {
    Path::new(vec![start, dest], vec![Tu::new(1)], Tu::new(1))
}

fn wall(ledger: &mut CoverLedger, at: CellLevel, band: HeightBand) {
    ledger.insert(
        at,
        CoverEntry::seeded(
            CoverHp::new(10),
            band,
            ArmorProtection::new(0),
            ArmorHardness::new(0),
            TerrainPieceKind::Wall,
        ),
    );
}

// The same probe the gate flies, asked about one pair of poses in one world.
fn sighted(
    from: CellLevel,
    from_stance: Stance,
    to: CellLevel,
    to_stance: Stance,
    cover: &CoverLedger,
) -> bool {
    let occupancy = OccupancyGrid::new();
    let surface = SurfaceGrid::new();
    let tuning = CombatTuning::default();
    let (from_at, to_at) = (Position::new(from), Position::new(to));
    let observer = Observer {
        position:         &from_at,
        stance:           &from_stance,
        facing:           &NORTH,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };
    let target = Target {
        position: &to_at,
        stance:   &to_stance,
    };
    *has_los(
        &observer,
        &target,
        MarchGrids {
            occupancy: &occupancy,
            surface: &surface,
            cover,
        },
        &tuning,
        |_| false,
    )
}

fn verdict(
    start: CellLevel,
    dest: CellLevel,
    suppressor: CellLevel,
    cover: &CoverLedger,
) -> MoveVerdict {
    let at = Position::new(start);
    let pool = Tu::new(100);
    let pinned = Suppressed::new(SuppressorCell::new(suppressor));
    let terrain = BareTerrain::new();
    can_move(
        Mover::new(
            UNSPAWNED_MOVER,
            &at,
            &pool,
            &STANDING,
            &NORTH,
            Some(&pinned),
        ),
        &dest,
        &one_step(start, dest),
        cover,
        &terrain.sight(),
    )
}

#[test]
fn the_break_away_probe_runs_from_the_mover_to_the_shot_cell_and_not_back() {
    let (start, dest) = (on(1, 0), on(0, 0));
    let shot_cell = on(10, 0);
    let mut cover = CoverLedger::new();
    wall(&mut cover, on(5, 0), HeightBand::Mid);
    wall(&mut cover, shot_cell, HeightBand::Low);

    assert!(
        !sighted(dest, STANDING, shot_cell, STANDING, &cover),
        "this world must hide the shot cell from the mover, or the case cannot tell the two \
         directions apart",
    );
    assert!(
        sighted(shot_cell, STANDING, dest, STANDING, &cover),
        "and it must leave the mover in view from the shot cell, so the two rays disagree",
    );
    assert_eq!(
        verdict(start, dest, shot_cell, &cover),
        MoveVerdict::Allowed,
        "the gate must answer with the ray the mover flies, not the one flown back at it: a \
         probe run the other way round sees the mover and would refuse this walk",
    );
}

#[test]
fn the_shot_cell_is_aimed_at_as_a_standing_silhouette() {
    let (start, dest) = (on(1, 0), on(0, 0));
    let shot_cell = on(10, 0);
    let mut cover = CoverLedger::new();
    wall(&mut cover, on(9, 0), HeightBand::Mid);

    assert!(
        sighted(dest, STANDING, shot_cell, STANDING, &cover),
        "the ray to a standing silhouette clears this wall",
    );
    for crouched in [StanceKind::Crouching, StanceKind::Prone] {
        assert!(
            !sighted(dest, STANDING, shot_cell, Stance::new(crouched), &cover),
            "the ray to a {crouched:?} silhouette does not, so the stance the shot cell is given \
             decides this world",
        );
    }
    assert_eq!(
        verdict(start, dest, shot_cell, &cover),
        MoveVerdict::Suppressed,
        "the cell the fire came from records no stance, and the gate aims at it standing: any \
         lower silhouette would hide it behind this wall and free the mover",
    );
}
