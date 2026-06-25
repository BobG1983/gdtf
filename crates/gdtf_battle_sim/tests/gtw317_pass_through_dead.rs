//! GTW-317 PASS-THROUGH-DEAD integration tests: a round passes THROUGH a corpse.
//!
//! The march now consults the real `is_dead` predicate threaded through
//! `resolve_coarse` (`fire/compose.rs::resolve_round` builds it from the TARGET
//! query's CURRENT `LifeState`). A ganger at [`LifeState::Dead`] is transparent: the
//! round flies on to the next blocker (next occupant / cover / wall / nothing). A
//! live occupant — including a [`LifeState::Downed`] one — still stops the round;
//! only `Dead` is skipped (`docs/combat/resolution.md` §9 corpse-skip discipline).
//!
//! These tests exercise the REAL `fire()` volley over the two disjoint queries with
//! an injected seeded [`ShotRng`]/[`SeverityRng`] pair (GTW-14; render-free, zero pixels), mirroring
//! `tests/gtw304_landed_hit.rs` and the in-crate `fire/` tests. Two occupants are
//! placed in a straight East-facing line so a tight-cone burst flies (essentially)
//! the same ray each round; the front occupant's round-1 death is written to its
//! `LifeState` BEFORE round 2 runs, so round 2 reads the fresh corpse and passes
//! through it to the live occupant behind.
//!
//! Every `app.world_mut()` / `World` mutation is in a TEST BODY (`bevy-traps.md` #7
//! carve-out); no function here takes `&mut World` / `&World`.

use bevy::{
    ecs::system::SystemState,
    prelude::{Entity, World},
};
use gdtf_battle_sim::{
    Accuracy, Aiming, ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType,
    BaseSpread, BattleGrids, BattleSeed, BodyPart, BraceStairCells, Cell, CellLevel, CombatTuning,
    CoverLedger, DamageProfile, DamageType, Direction, Facing, FatalBias, FireMode, FireModeSpec,
    HandlingProfile, HeightBand, Hp, InflictedWounds, Kickback, Level, LifeState, Luck, Magazine,
    MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, OccupancyGrid, PieceQuery,
    Position, ReloadTu, SeverityRng, ShooterQuery, Shooting, ShotKind, ShotRng, SlabLedger, Stable,
    Stance, StanceKind, SurfaceGrid, Toughness, Tu, TuMax, Volley, WeaponBundle, WeaponDamage,
    WeaponName, WeaponPunch, WeaponQuery, WeaponShred, WearsQuery, WieldedBy, WieldsQuery, WornBy,
    Wounds, fire::FireOrder,
};

/// The shooter cell — well to the West so the East-facing line of occupants lies
/// straight ahead of the muzzle.
fn shooter_cell() -> CellLevel {
    CellLevel::new(Cell::new(2, 5), Level::new(0))
}

/// The FRONT occupant cell (the nearer of the two, struck first).
fn front_cell() -> CellLevel {
    CellLevel::new(Cell::new(8, 5), Level::new(0))
}

/// The BEHIND occupant cell — one cell further East along the same ray.
fn behind_cell() -> CellLevel {
    CellLevel::new(Cell::new(9, 5), Level::new(0))
}

/// A multi-round burst fire-mode (`shots` rounds, arbitrary non-pinned numbers).
const fn burst_mode(shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Burst,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.3),
        ModeShots::new(shots),
    )
}

/// Equip a ganger's six worn-armor-piece entities (GTW-323 / ADR-0004) at the thin
/// uniform stats, related via `WornBy` so `fire()` resolves the struck location through
/// `ganger → Wears → the BodyPart-tagged piece` (the relationship hook populates `Wears`
/// synchronously in a bare `World` spawn).
fn equip_thin_armor(world: &mut World, ganger: Entity) {
    for part in BodyPart::ALL {
        world.spawn((
            WornBy::new(ganger),
            part,
            ArmorFloor::new(0),
            ArmorProtection::new(0),
            ArmorIntegrity::new(1),
            ArmorHardness::new(0),
            ArmorType::DEFAULT,
        ));
    }
}

/// Spawn an armed, alive, loaded, aiming shooter at the shooter cell facing East
/// with a TIGHT cone (low `BaseSpread` + high `Accuracy` + aiming) so a multi-round
/// burst flies essentially the same ray each round. Carries the full `ShooterQuery`
/// (`With<Weapon>` + weapon stats) AND `TargetQuery` set (its own liveness reads
/// through the target query).
fn spawn_shooter(world: &mut World, mode: FireModeSpec) -> Entity {
    let bundle = WeaponBundle::new(
        WeaponName::new("probe-weapon".to_owned()),
        // A near-zero cone so every round of the burst stays on the central axis.
        BaseSpread::new(0.01),
        Accuracy::new(4.0),
        Kickback::new(0.0),
        FatalBias::new(0.0),
        // High damage + penetration so a landed round bites through the thin suit.
        DamageProfile::new(
            WeaponDamage::new(60),
            WeaponPunch::new(40),
            WeaponShred::new(20),
            DamageType::Kinetic,
        ),
        HandlingProfile::new(
            Magazine::new(10, MagazineSize::new(30), ReloadTu::new(12)),
            FireMode::new(vec![mode]),
            // Braced so recoil-climb does not walk later rounds off the line.
            Stable::new(true),
        ),
    );
    let shooter = world
        .spawn((
            Position::new(shooter_cell()),
            Facing::new(Direction::East),
            Stance::new(StanceKind::Standing),
            Aiming::new(true),
            Shooting::new(1.0),
            Tu::new(250),
            TuMax::new(100),
            (
                Hp::new(50),
                Wounds::new(10),
                LifeState::Alive,
                InflictedWounds::default(),
                Toughness::new(1.0),
                Luck::new(0.0),
            ),
        ))
        .id();
    // GTW-323 slice 2: the weapon rides on a related weapon entity (`Wields`); the
    // `WieldedBy` insert hook populates the ganger's `Wields` synchronously in a bare
    // `World` spawn so the very next `fire()` resolves it.
    world.spawn((WieldedBy::new(shooter), bundle));
    // GTW-323 slice 1: equip the shooter's worn-armor PIECE entities (it is a ganger too).
    equip_thin_armor(world, shooter);
    shooter
}

/// Spawn a standing ganger at `cell` with the given starting `wounds` and life
/// `state`, carrying the full `TargetQuery` battle-surface set. Returns its entity.
fn spawn_ganger(world: &mut World, cell: CellLevel, wounds: u8, state: LifeState) -> Entity {
    let ganger = world
        .spawn((
            Position::new(cell),
            Stance::new(StanceKind::Standing),
            Hp::new(40),
            Wounds::new(wounds),
            state,
            InflictedWounds::default(),
            Toughness::new(1.0),
            Luck::new(0.0),
        ))
        .id();
    // GTW-323: equip the ganger's worn-armor PIECE entities (the `fire()` armor path).
    equip_thin_armor(world, ganger);
    ganger
}

/// Place a STANDING (HIGH-band) occupant into the occupancy grid at `cell`.
fn place_occupant(occupancy: &mut OccupancyGrid, cell: CellLevel, entity: Entity) {
    occupancy.set_occupant(cell, Some(entity));
    occupancy.set_occupant_band(cell, Some(HeightBand::High));
}

/// Fire ONE seeded `mode` volley East at the front cell, reading the maintained
/// grids straight off the world. The two disjoint queries borrow the world mutably,
/// so the grids are snapshotted (cloned read views) first.
fn fire_volley(
    world: &mut World,
    shooter: Entity,
    mode: FireModeSpec,
    occupancy: &OccupancyGrid,
    seed: u64,
) -> Volley {
    let tuning = CombatTuning::default();
    let mut rng = ShotRng::from_root(BattleSeed::new(seed));
    let mut sev_rng = SeverityRng::from_root(BattleSeed::new(seed));
    let surface = SurfaceGrid::new();
    let mut cover = CoverLedger::new();
    let mut slab = SlabLedger::new();

    let mut state: SystemState<(
        ShooterQuery,
        gdtf_battle_sim::TargetQuery,
        WearsQuery,
        PieceQuery,
        WieldsQuery,
        WeaponQuery,
    )> = SystemState::new(world);
    // `get_mut` now returns a `Result` (Bevy 0.19); the params always validate
    // here, so an `Err` is a structural impossibility — assert it loudly rather
    // than silently producing an empty volley.
    let access = state.get_mut(world);
    assert!(access.is_ok(), "shooter/target queries must validate");
    let volley = match access {
        Ok((mut shooters, mut targets, wears, mut pieces, wields, mut weapons)) => {
            gdtf_battle_sim::fire(
                shooter,
                FireOrder {
                    mode:         &mode,
                    target_cell:  Cell::new(front_cell().x, front_cell().y),
                    target_level: Level::new(0),
                },
                &mut shooters,
                &mut targets,
                &wears,
                &mut pieces,
                &wields,
                &mut weapons,
                BattleGrids {
                    occupancy,
                    surface: &surface,
                    cover: &mut cover,
                    slab: &mut slab,
                    brace_cells: &BraceStairCells::empty(),
                },
                &tuning,
                &mut rng,
                &mut sev_rng,
            )
        }
        Err(_) => Volley {
            reports: Vec::new(),
            shots:   Vec::new(),
        },
    };
    state.apply(world);
    volley
}

/// Whether a [`Volley`] report struck the given ganger entity.
fn report_struck(volley: &Volley, entity: Entity) -> bool {
    volley
        .reports
        .iter()
        .any(|r| r.kind == ShotKind::Ganger(entity))
}

/// The number of rounds in `volley` that struck the given ganger entity.
fn struck_count(volley: &Volley, entity: Entity) -> usize {
    volley
        .reports
        .iter()
        .filter(|r| r.kind == ShotKind::Ganger(entity))
        .count()
}

/// AC #1 — a BURST whose round 1 KILLS the front ganger passes round 2+ THROUGH the
/// fresh corpse onto a LIVE ganger directly behind it.
///
/// The front ganger spawns with `Wounds = 1`, so any non-graze severity on round 1
/// saturates its life pool to `0` → `LifeState::Dead` BEFORE round 2 runs. We search
/// a small seed set for a seed whose round-1 report actually killed the front (a
/// graze on every seed would not), then assert that on that same seed the volley
/// also struck the BEHIND ganger — i.e. a later round passed through the corpse.
#[test]
fn burst_kills_front_then_passes_through_to_live_behind() {
    let seeds: [u64; 12] = [
        0x5A1C_AC75,
        0x0BAD_F00D,
        0xDEAD_BEEF,
        0xFEED_FACE,
        0x1234_5678,
        0xCAFE_B0BA,
        0x9E37_79B9,
        0xA11C_E5ED,
        0x0000_0001,
        0x7FFF_FFFF,
        0xABCD_1234,
        0x1357_9BDF,
    ];

    let mut proved = false;
    for seed in seeds {
        let mut world = World::new();
        let mode = burst_mode(3);
        let shooter = spawn_shooter(&mut world, mode);
        // Front: 1 Wound, so a single non-graze round 1 kills it.
        let front = spawn_ganger(&mut world, front_cell(), 1, LifeState::Alive);
        // Behind: healthy and alive, directly behind on the same ray.
        let behind = spawn_ganger(&mut world, behind_cell(), 6, LifeState::Alive);

        let mut occupancy = OccupancyGrid::new();
        place_occupant(&mut occupancy, front_cell(), front);
        place_occupant(&mut occupancy, behind_cell(), behind);

        let behind_wounds_before = world.get::<Wounds>(behind).map_or(0, |w| **w);

        let volley = fire_volley(&mut world, shooter, mode, &occupancy, seed);

        // Did round 1 actually kill the front? (Its first report on the front carries
        // life_after == Dead.) Only then is this seed a valid pass-through witness.
        let front_died_round_one = volley
            .reports
            .iter()
            .find(|r| r.kind == ShotKind::Ganger(front))
            .and_then(|r| r.applied)
            .is_some_and(|a| a.life_after == LifeState::Dead);
        if !front_died_round_one {
            continue;
        }

        // The front is Dead in the world after the volley.
        let front_life = world.get::<LifeState>(front).copied();
        assert_eq!(
            front_life,
            Some(LifeState::Dead),
            "seed {seed:#x}: the front ganger must be Dead after the killing burst",
        );

        // A later round passed THROUGH the corpse and affected the BEHIND ganger: it
        // is struck in the reports AND its wounds dropped (or it was struck at all).
        let behind_struck = report_struck(&volley, behind);
        let behind_wounds_after = world.get::<Wounds>(behind).map_or(0, |w| **w);
        assert!(
            behind_struck,
            "seed {seed:#x}: a round must pass THROUGH the corpse and strike the live ganger \
             behind it — got reports {:?}",
            volley.reports,
        );
        assert!(
            behind_wounds_after < behind_wounds_before
                || volley
                    .reports
                    .iter()
                    .any(|r| r.kind == ShotKind::Ganger(behind) && r.applied.is_some()),
            "seed {seed:#x}: the behind ganger must take an effect (wounds drop / applied damage)",
        );
        proved = true;
        break;
    }

    assert!(
        proved,
        "no seed produced a round-1 kill of the front ganger — widen the seed set; the \
         pass-through could not be witnessed",
    );
}

/// AC #2 — a BURST that kills the front ganger with NOTHING behind it: the leftover
/// rounds strike cover / wall / nothing, and the dead front ganger takes NO further
/// wounds (it is struck exactly once — the killing round — never again).
#[test]
fn burst_kills_front_with_nothing_behind_does_not_re_wound_corpse() {
    let seeds: [u64; 12] = [
        0x5A1C_AC75,
        0x0BAD_F00D,
        0xDEAD_BEEF,
        0xFEED_FACE,
        0x1234_5678,
        0xCAFE_B0BA,
        0x9E37_79B9,
        0xA11C_E5ED,
        0x0000_0001,
        0x7FFF_FFFF,
        0xABCD_1234,
        0x1357_9BDF,
    ];

    let mut proved = false;
    for seed in seeds {
        let mut world = World::new();
        let mode = burst_mode(3);
        let shooter = spawn_shooter(&mut world, mode);
        let front = spawn_ganger(&mut world, front_cell(), 1, LifeState::Alive);

        let mut occupancy = OccupancyGrid::new();
        place_occupant(&mut occupancy, front_cell(), front);
        // Nothing behind: no occupant past the front cell.

        let volley = fire_volley(&mut world, shooter, mode, &occupancy, seed);

        let front_died_round_one = volley
            .reports
            .iter()
            .find(|r| r.kind == ShotKind::Ganger(front))
            .and_then(|r| r.applied)
            .is_some_and(|a| a.life_after == LifeState::Dead);
        if !front_died_round_one {
            continue;
        }

        // The corpse is struck EXACTLY ONCE — the killing round. Every later round
        // passes through it (nothing behind → cover/wall/nothing), never re-wounding
        // the corpse. (resolve_and_apply's corpse-skip would no-op a corpse strike
        // anyway, but the march no longer even reports the corpse after death.)
        let front_strikes = struck_count(&volley, front);
        assert_eq!(
            front_strikes, 1,
            "seed {seed:#x}: the dead front ganger must be struck exactly once (the killing \
             round), never re-wounded — got {front_strikes} strikes in {:?}",
            volley.reports,
        );

        // Exactly one InflictedWound was recorded on the corpse (the killing round) —
        // no further wounds accreted.
        let recorded = world.get::<InflictedWounds>(front).map_or(0, |w| w.len());
        assert!(
            recorded <= 1,
            "seed {seed:#x}: the corpse must record at most the one killing wound, got {recorded}",
        );
        proved = true;
        break;
    }

    assert!(
        proved,
        "no seed produced a round-1 kill of the lone front ganger — widen the seed set",
    );
}

/// AC #3 — a PRE-EXISTING corpse (spawned `LifeState::Dead`) in front of a live
/// target: a single shot passes THROUGH the corpse to the live target behind it.
#[test]
fn single_shot_passes_through_preexisting_corpse_to_live_target() {
    let mut world = World::new();
    let mode = burst_mode(1); // a single round
    let shooter = spawn_shooter(&mut world, mode);
    // Front: spawned already DEAD (a pre-existing corpse), Wounds already 0.
    let corpse = spawn_ganger(&mut world, front_cell(), 0, LifeState::Dead);
    // Behind: a live target directly behind the corpse on the same ray.
    let live = spawn_ganger(&mut world, behind_cell(), 6, LifeState::Alive);

    let mut occupancy = OccupancyGrid::new();
    place_occupant(&mut occupancy, front_cell(), corpse);
    place_occupant(&mut occupancy, behind_cell(), live);

    let live_wounds_before = world.get::<Wounds>(live).map_or(0, |w| **w);

    // A tight cone + braced shooter aimed straight down the line: the one round flies
    // through the corpse onto the live target. (Seed pinned for a deterministic run.)
    let volley = fire_volley(&mut world, shooter, mode, &occupancy, 0xC0FF_EE17);

    assert_eq!(volley.reports.len(), 1, "exactly one round fired");
    // The single round did NOT stop on the corpse.
    assert!(
        !report_struck(&volley, corpse),
        "the single round must NOT strike the pre-existing corpse — got {:?}",
        volley.reports,
    );
    // It passed through and struck the live target behind.
    assert!(
        report_struck(&volley, live),
        "the single round must pass THROUGH the corpse and strike the live target — got {:?}",
        volley.reports,
    );
    let live_wounds_after = world.get::<Wounds>(live).map_or(0, |w| **w);
    let applied = volley
        .reports
        .iter()
        .any(|r| r.kind == ShotKind::Ganger(live) && r.applied.is_some());
    assert!(
        live_wounds_after < live_wounds_before || applied,
        "the live target behind the corpse must take an effect (wounds drop / applied damage)",
    );
    // The corpse took no wound (its record stays empty).
    let corpse_wounds = world.get::<InflictedWounds>(corpse).map_or(0, |w| w.len());
    assert_eq!(
        corpse_wounds, 0,
        "the pre-existing corpse must record NO wound — the round passed through it",
    );
}

/// AC #4 — DETERMINISM: the same [`BattleSeed`] reproduces a byte-equal [`Volley`]
/// (reports + shots), even with the new corpse-skip predicate active mid-burst.
#[test]
fn same_seed_reproduces_byte_equal_volley_with_corpse_skip() {
    let seed = 0xDEAD_BEEF_u64;

    // Build an identical world + fire an identical 3-round burst twice; the volleys
    // (reports AND shots) must be byte-equal. The front ganger has Wounds = 1 so a
    // killing round mid-burst engages the corpse-skip on the SECOND+ round — proving
    // the skip itself is RNG-free and replay-stable.
    let run = || {
        let mut world = World::new();
        let mode = burst_mode(3);
        let shooter = spawn_shooter(&mut world, mode);
        let front = spawn_ganger(&mut world, front_cell(), 1, LifeState::Alive);
        let behind = spawn_ganger(&mut world, behind_cell(), 6, LifeState::Alive);
        let mut occupancy = OccupancyGrid::new();
        place_occupant(&mut occupancy, front_cell(), front);
        place_occupant(&mut occupancy, behind_cell(), behind);
        fire_volley(&mut world, shooter, mode, &occupancy, seed)
    };

    assert_eq!(
        run(),
        run(),
        "the same battle seed must reproduce a byte-equal volley (reports + shots) with the \
         corpse-skip predicate active",
    );
}

/// AC #5 — a DOWNED (not Dead) occupant in the ray still STOPS the round: it is hit,
/// NOT passed through (only `LifeState::Dead` is transparent).
#[test]
fn downed_occupant_still_stops_the_round() {
    let mut world = World::new();
    let mode = burst_mode(1); // a single round
    let shooter = spawn_shooter(&mut world, mode);
    // Front: DOWNED (alive, incapacitated) — must still block the round.
    let downed = spawn_ganger(&mut world, front_cell(), 4, LifeState::Downed);
    // Behind: a live target — it must NOT be reached (the round stops on the downed).
    let behind = spawn_ganger(&mut world, behind_cell(), 6, LifeState::Alive);

    let mut occupancy = OccupancyGrid::new();
    place_occupant(&mut occupancy, front_cell(), downed);
    place_occupant(&mut occupancy, behind_cell(), behind);

    let behind_wounds_before = world.get::<Wounds>(behind).map_or(0, |w| **w);

    let volley = fire_volley(&mut world, shooter, mode, &occupancy, 0xD09E_D517);

    assert_eq!(volley.reports.len(), 1, "exactly one round fired");
    // The round stopped on the DOWNED occupant (it is hit, not passed through).
    assert!(
        report_struck(&volley, downed),
        "a Downed occupant must STOP the round (only Dead is transparent) — got {:?}",
        volley.reports,
    );
    // The behind target was NOT reached.
    assert!(
        !report_struck(&volley, behind),
        "the round must NOT pass through a Downed occupant to the target behind — got {:?}",
        volley.reports,
    );
    let behind_wounds_after = world.get::<Wounds>(behind).map_or(0, |w| **w);
    assert_eq!(
        behind_wounds_after, behind_wounds_before,
        "the target behind a Downed occupant must take NO effect",
    );
}
