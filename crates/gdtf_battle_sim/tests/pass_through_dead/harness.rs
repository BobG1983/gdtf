//! Shared GTW-317 line-of-fire fixture: the line cells, the burst mode, the
//! shooter / occupant spawners, the seeded volley driver, and the struck-report
//! readers.

use bevy::{
    ecs::system::SystemState,
    prelude::{Entity, World},
};
use gdtf_battle_sim::{
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType, BodyPart, WornBy,
    },
    cover::{CoverLedger, HeightBand},
    fire::{
        BattleGrids, FireOrder, MeleeQuery, MountedQuery, PieceQuery, ShooterQuery, Volley,
        WeaponQuery, WearsQuery, WieldsQuery,
    },
    ganger::{Aiming, Facing, Hp, Luck, Shooting, Toughness, TuMax, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::{InjuryRegistry, InjuryTables},
    magazine::{LoadedRounds, Magazine, ReloadTu},
    prelude::{
        Cell, CellLevel, Direction, Level, LifeState, OccupancyGrid, Position, Stance, StanceKind,
        Tu,
    },
    resolve_coarse::ShotKind,
    slab::BraceStairCells,
    surface::SurfaceGrid,
    test_support::{GangerEntityBuilder, empty_slab_ledger, injury_rng, severity_rng, shot_rng},
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        Handedness, HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots,
        ModeTuPercent, Shove, Stable, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch,
        WeaponShred, WieldedBy,
    },
};

/// The shooter cell — well to the West so the East-facing line of occupants lies
/// straight ahead of the muzzle.
pub(crate) fn shooter_cell() -> CellLevel {
    CellLevel::new(Cell::new(2, 5), Level::new(0))
}

/// The FRONT occupant cell (the nearer of the two, struck first).
pub(crate) fn front_cell() -> CellLevel {
    CellLevel::new(Cell::new(8, 5), Level::new(0))
}

/// The BEHIND occupant cell — one cell further East along the same ray.
pub(crate) fn behind_cell() -> CellLevel {
    CellLevel::new(Cell::new(9, 5), Level::new(0))
}

/// A multi-round burst fire-mode (`shots` rounds, arbitrary non-pinned numbers).
pub(crate) const fn burst_mode(shots: u16) -> FireModeSpec {
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
pub(crate) fn equip_thin_armor(world: &mut World, ganger: Entity) {
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
pub(crate) fn spawn_shooter(world: &mut World, mode: FireModeSpec) -> Entity {
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
            Magazine::new(
                LoadedRounds::new(10),
                MagazineSize::new(30),
                ReloadTu::new(12),
            ),
            FireMode::new(vec![mode]),
            // Braced so recoil-climb does not walk later rounds off the line.
            Stable::new(true),
            Shove::new(false),
            Handedness::OneHanded,
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
pub(crate) fn line_ganger(
    world: &mut World,
    cell: CellLevel,
    wounds: u8,
    state: LifeState,
) -> Entity {
    let ganger = GangerEntityBuilder::new()
        .at(cell)
        .stance(StanceKind::Standing)
        .combat_vitals(40, wounds)
        .life_state(state)
        .spawn(world);
    // GTW-323: equip the ganger's worn-armor PIECE entities (the `fire()` armor path).
    equip_thin_armor(world, ganger);
    ganger
}

/// Place a STANDING (HIGH-band) occupant into the occupancy grid at `cell`.
pub(crate) fn place_occupant(occupancy: &mut OccupancyGrid, cell: CellLevel, entity: Entity) {
    occupancy.set_occupant(cell, Some(entity));
    occupancy.set_occupant_band(cell, Some(HeightBand::High));
}

/// Fire ONE seeded `mode` volley East at the front cell, reading the maintained
/// grids straight off the world. The two disjoint queries borrow the world mutably,
/// so the grids are snapshotted (cloned read views) first.
pub(crate) fn fire_volley(
    world: &mut World,
    shooter: Entity,
    mode: FireModeSpec,
    occupancy: &OccupancyGrid,
    seed: u64,
) -> Volley {
    /// The `fire()` query tuple, aliased so the `SystemState` type stays under clippy's
    /// `type_complexity` gate (the GTW-543 mounted-weapon `MountedQuery` addition tipped it over).
    /// Declared FIRST in the fn so it precedes the `let`s (`items_after_statements`).
    type FireQueries<'w, 's> = (
        ShooterQuery<'w, 's>,
        gdtf_battle_sim::fire::TargetQuery<'w, 's>,
        WearsQuery<'w, 's>,
        PieceQuery<'w, 's>,
        WieldsQuery<'w, 's>,
        WeaponQuery<'w, 's>,
        MeleeQuery<'w, 's>,
        MountedQuery<'w, 's>,
    );
    let tuning = CombatTuning::default();
    let mut rng = shot_rng(seed);
    let mut sev_rng = severity_rng(seed);
    let mut injury_rng = injury_rng(seed);
    let surface = SurfaceGrid::new();
    let mut cover = CoverLedger::new();
    let mut slab = empty_slab_ledger();

    let mut state: SystemState<FireQueries> = SystemState::new(world);
    // `get_mut` now returns a `Result` (Bevy 0.19); the params always validate
    // here, so an `Err` is a structural impossibility — assert it loudly rather
    // than silently producing an empty volley.
    let access = state.get_mut(world);
    assert!(access.is_ok(), "shooter/target queries must validate");
    let volley = match access {
        Ok((mut shooters, mut targets, wears, mut pieces, wields, mut weapons, melee, mounted)) => {
            gdtf_battle_sim::fire::fire(
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
                &melee,
                &mounted,
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
                &InjuryTables::default(),
                &InjuryRegistry::default(),
                &mut injury_rng,
            )
        }
        Err(_) => Volley {
            reports: Vec::new(),
            shots:   Vec::new(),
            splash:  Vec::new(),
        },
    };
    state.apply(world);
    volley
}

/// Whether a [`Volley`] report struck the given ganger entity.
pub(crate) fn report_struck(volley: &Volley, entity: Entity) -> bool {
    volley
        .reports
        .iter()
        .any(|r| r.kind == ShotKind::Ganger(entity))
}

/// The applied-damage block of the first report in `volley` whose GEOMETRY struck
/// `entity`, else `None` — read through the GTW-573 per-kind verdict (a corpse-skip /
/// defensive fold carries no ganger verdict, so it reads `None` here).
pub(crate) fn applied_on(
    volley: &Volley,
    entity: Entity,
) -> Option<gdtf_battle_sim::resolve_and_apply::AppliedDamage> {
    volley
        .reports
        .iter()
        .find(|r| r.kind == ShotKind::Ganger(entity))
        .and_then(|r| match &r.verdict {
            gdtf_battle_sim::resolve_and_apply::HitVerdict::Ganger(verdict) => {
                Some(verdict.applied)
            }
            _ => None,
        })
}

/// The number of rounds in `volley` that struck the given ganger entity.
pub(crate) fn struck_count(volley: &Volley, entity: Entity) -> usize {
    volley
        .reports
        .iter()
        .filter(|r| r.kind == ShotKind::Ganger(entity))
        .count()
}
