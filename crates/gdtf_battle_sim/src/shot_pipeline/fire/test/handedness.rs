//! GTW-443 — the weapon-handedness fire gate on the real `fire()` path: a `TwoHanded`
//! weapon is refused (empty volley, zero mutation) once a hand-disabling injury drops the
//! shooter below two hands (C4), a `OneHanded` weapon stays usable at one hand AND the
//! `Wields` relationship to a 2H weapon is untouched (C5, F5), a `TwoHanded` weapon fires
//! normally at two hands (C6 — the gate is conditional), and an uninjured `OneHanded`
//! shooter is unaffected (C9 baseline).

use super::support::*;
use crate::weapon::Wields;

/// Spawn an UNARMED ganger (the full shooter + target component set, no weapon) at
/// `(5, 5, 0)` — the hand-count tests then equip a chosen-handedness weapon + optional
/// disabling injury. Mirrors `spawn_shooter` minus the weapon equip.
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

/// Run `fire()` once for `shooter` against `(8, 5, 0)` with the single mode, returning the
/// frozen [`Volley`]. Factored so each hand-count test spells the wide `fire()` call once.
fn run_fire(world: &mut World, shooter: Entity, mode: &FireModeSpec) -> Volley {
    let tuning = CombatTuning::default();
    let occupancy = OccupancyGrid::new();
    let surface = SurfaceGrid::new();
    let mut cover = CoverLedger::new();
    let mut slab = SlabLedger::new();
    let mut shot_r = rng();
    let mut sev_r = severity_rng();

    let mut state: SystemState<FireQueries> = SystemState::new(world);
    let Ok((mut shooters, mut targets, wears, mut pieces, wields, mut weapons, melee, mounted)) =
        state.get_mut(world)
    else {
        return Volley::empty();
    };
    fire(
        shooter,
        FireOrder {
            mode,
            target_cell: Cell::new(8, 5),
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
            occupancy:   &occupancy,
            surface:     &surface,
            cover:       &mut cover,
            slab:        &mut slab,
            brace_cells: &BraceStairCells::empty(),
        },
        &tuning,
        &mut shot_r,
        &mut sev_r,
        &injury_tables(),
        &injury_registry(),
        &mut injury_rng(),
    )
}

#[test]
fn two_handed_refused_at_one_hand_empty_volley_no_mutation() {
    // C4: a TwoHanded weapon with a hand-disabling injury (1 hand left) → an EMPTY volley
    // that mutates NOTHING (fail-closed): TU pool and magazine both unchanged.
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
    // Zero mutation: TU pool and magazine are exactly as spawned.
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
    // C5: a OneHanded weapon stays usable with one working hand → a NON-empty volley; and
    // the `Wields` relationship to the weapon stays intact (F5 — handedness gates FIRE
    // only, it never touches the relationship).
    let mut world = World::new();
    let mode = single_mode(0.2, 1);
    let shooter = spawn_unarmed(&mut world);
    equip_handed_weapon(&mut world, shooter, Handedness::OneHanded);
    give_disabled_hand(&mut world, shooter, BodyPart::LeftArm);

    // The Wields relationship resolves to a weapon BEFORE firing...
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

    // ...and AFTER firing the Wields relationship is UNTOUCHED (F5).
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
    // C6: the gate is CONDITIONAL — an uninjured (two-hands) shooter fires a TwoHanded
    // weapon normally (non-empty volley). Not a blanket 2H ban.
    let mut world = World::new();
    let mode = single_mode(0.2, 1);
    let shooter = spawn_unarmed(&mut world);
    equip_handed_weapon(&mut world, shooter, Handedness::TwoHanded);
    // No disabling injury → the uninjured two-hands default.

    let volley = run_fire(&mut world, shooter, &mode);
    assert!(
        !volley.reports.is_empty(),
        "a TwoHanded weapon fires normally at two hands (the gate is conditional)",
    );
}

#[test]
fn uninjured_one_handed_fires_normally() {
    // C9 (volley half): an uninjured OneHanded shooter fires normally — the hand-count
    // clause never spuriously gates it.
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
