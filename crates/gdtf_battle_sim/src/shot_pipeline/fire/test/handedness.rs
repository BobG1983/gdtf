use super::support::*;
use crate::{resolve_and_apply::WoundRoll, weapon::Wields};

fn spawn_unarmed(world: &mut World) -> Entity {
    let ganger = world
        .spawn((
            Position::new(CellLevel::new(Cell::new(5, 5), Level::new(0))),
            Facing::new(Direction::East),
            Stance::new(StanceKind::Standing),
            Aiming::new(false),
            Shooting::new(1.0),
            Tu::new(200),
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
    equip_uniform_armor(world, ganger, 0, 0, 1, 0);
    ganger
}

fn run_fire(world: &mut World, shooter: Entity, mode: &FireModeSpec) -> Volley {
    let tuning = CombatTuning::default();
    let occupancy = OccupancyGrid::new();
    let surface = SurfaceGrid::new();
    let mut cover = CoverLedger::new();
    let mut slab = SlabLedger::new();
    let mut shot_r = rng();
    let mut sev_r = severity_rng();

    let mut state: SystemState<FireQueries> = SystemState::new(world);
    let Ok((mut shooters, mut arms, mut bodies)) = state.get_mut(world) else {
        return Volley::empty();
    };
    fire(
        FireOrder {
            shooter,
            mode,
            target_cell: Cell::new(8, 5),
            target_level: Level::new(0),
        },
        &mut shooters,
        &mut arms,
        &mut bodies,
        BattleGrids {
            occupancy:   &occupancy,
            surface:     &surface,
            cover:       &mut cover,
            slab:        &mut slab,
            brace_cells: &BraceStairCells::empty(),
        },
        &mut shot_r,
        &mut WoundRoll {
            tuning:       &tuning,
            severity_rng: &mut sev_r,
            tables:       &injury_tables(),
            registry:     &injury_registry(),
            injury_rng:   &mut injury_rng(),
        },
    )
}

#[test]
fn two_handed_refused_at_one_hand_empty_volley_no_mutation() {
    let mut world = World::new();
    let mode = single_mode(0.2, 1);
    let shooter = spawn_unarmed(&mut world);
    equip_handed_weapon(&mut world, shooter, Handedness::TwoHanded);
    give_disabled_hand(&mut world, shooter, BodyPart::LeftArm);

    let volley = run_fire(&mut world, shooter, &mode);

    assert!(
        volley.reports.is_empty() && volley.shots.is_empty(),
        "a TwoHanded weapon at one hand must fire NOTHING",
    );
    assert_eq!(
        world.get::<Tu>(shooter).copied(),
        Some(Tu::new(200)),
        "no TU charged when the 2H weapon is refused",
    );
    assert_eq!(
        weapon_rounds(&world, shooter),
        Some(10),
        "no round spent when the 2H weapon is refused",
    );
}

#[test]
fn one_handed_usable_at_one_hand_and_wields_intact() {
    let mut world = World::new();
    let mode = single_mode(0.2, 1);
    let shooter = spawn_unarmed(&mut world);
    equip_handed_weapon(&mut world, shooter, Handedness::OneHanded);
    give_disabled_hand(&mut world, shooter, BodyPart::LeftArm);

    let wielded_before = world
        .get::<Wields>(shooter)
        .and_then(Wields::weapon)
        .is_some();
    assert!(wielded_before, "precondition: the shooter wields a weapon");

    let volley = run_fire(&mut world, shooter, &mode);
    assert!(
        !volley.reports.is_empty(),
        "a OneHanded weapon stays usable with one working hand",
    );

    let wielded_after = world
        .get::<Wields>(shooter)
        .and_then(Wields::weapon)
        .is_some();
    assert!(
        wielded_after,
        "handedness gates FIRE only — the Wields relationship is untouched (F5)",
    );
}

#[test]
fn two_handed_fires_at_two_hands() {
    let mut world = World::new();
    let mode = single_mode(0.2, 1);
    let shooter = spawn_unarmed(&mut world);
    equip_handed_weapon(&mut world, shooter, Handedness::TwoHanded);

    let volley = run_fire(&mut world, shooter, &mode);
    assert!(
        !volley.reports.is_empty(),
        "a TwoHanded weapon fires normally at two hands (the gate is conditional)",
    );
}

#[test]
fn uninjured_one_handed_fires_normally() {
    let mut world = World::new();
    let mode = single_mode(0.2, 1);
    let shooter = spawn_unarmed(&mut world);
    equip_handed_weapon(&mut world, shooter, Handedness::OneHanded);

    let volley = run_fire(&mut world, shooter, &mode);
    assert!(
        !volley.reports.is_empty(),
        "an uninjured OneHanded shooter fires normally",
    );
}
