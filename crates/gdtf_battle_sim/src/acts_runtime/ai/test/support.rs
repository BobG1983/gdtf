//! Shared fixtures for the minimal-enemy-AI tests (GTW-70) — a headless live-ish harness
//! (`MinimalPlugins` + [`SimActsPlugin`], which now bundles the brain) seeded with the
//! battle-lifetime resources a live battle has, plus combatant + occupant spawn helpers.

pub(super) use bevy::prelude::{App, Entity, Messages, MinimalPlugins, World};

pub(super) use crate::{
    acts::{FireRequested, MoveRequested, SimActsPlugin},
    ai::{ActCadence, EnemyActCooldown},
    battle::PlayerFaction,
    cover::{CoverLedger, HeightBand},
    ganger::{
        Aiming, Direction, Facing, Faction, Hp, LifeState, Luck, Position, Shooting, Stance,
        StanceKind, Toughness, Tu, TuMax, Wounds,
    },
    inflicted_wound::InflictedWounds,
    injuries::{InjuryRegistry, InjuryTables},
    magazine::{Magazine, ReloadTu},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    rng::{BattleSeed, InjuryRng, LootRng, ProcgenRng, SeverityRng, ShotRng},
    slab::{BraceStairCells, SlabLedger},
    surface::SurfaceGrid,
    terrain::floor::FloorCostGrid,
    tuning::CombatTuning,
    turn::ActiveFaction,
    vertical::VerticalLinkGraph,
    visibility::{OmniscientFog, SquadVisibility},
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        Handedness, HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots,
        ModeTuPercent, Shove, Stable, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch,
        WeaponShred, WieldedBy,
    },
};

/// A fixed seed for the per-test RNG streams (arbitrary, not tuned).
const SEED: u64 = 0x5A1C_AC75;

/// Gang `0` is the player team; gang `1` is the enemy (the AI acts for this one).
pub(super) const PLAYER: Faction = Faction::new(0);
/// The enemy team — the faction the brain drives.
pub(super) const ENEMY: Faction = Faction::new(1);

/// The single-mode TU% the test weapon carries — moderate (a fraction of the TU pool per
/// shot) so a combatant can afford a couple of shots before its pool runs dry. Arbitrary
/// test magnitude, never pinned to shipped tuning.
const MODE_TU_PERCENT: f32 = 0.2;

/// Build a headless brain harness: [`MinimalPlugins`] + [`SimActsPlugin`] (which now bundles
/// the [`enemy_ai_turn`](crate::ai::enemy_ai_turn) brain alongside the act dispatchers) +
/// every sim resource a live battle has — the grids, the five seeded RNG streams, the
/// [`CombatTuning`], a uniform [`FloorCostGrid`], a full-vision [`SquadVisibility`], the AI
/// [`OmniscientFog`] move fog, the [`PlayerFaction`], and the [`ActiveFaction`] seeded to the
/// ENEMY (so the brain acts immediately on the first update). No
/// [`BattleInProgress`](crate::battle::BattleInProgress) gate is installed (this harness
/// omits the [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin)
/// that owns the `Simulate` `configure_sets`), so the bundled systems run ungated under their
/// own `run_if`s — the `acts` test precedent.
pub(super) fn brain_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(SimActsPlugin);
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(CoverLedger::new());
    app.insert_resource(SlabLedger::new());
    app.insert_resource(BraceStairCells::empty());
    app.insert_resource(VerticalLinkGraph::default());
    // Full-vision player fog + the omniscient AI move fog (both the whole grid extent). The
    // brain plans on the OmniscientFog; the full-vision SquadVisibility keeps dispatch_move's
    // player-fog fallback permissive too.
    let omniscient = SquadVisibility::omniscient(&OccupancyGrid::new());
    app.insert_resource(omniscient.clone());
    app.insert_resource(OmniscientFog::new(omniscient));
    let seed = BattleSeed::new(SEED);
    app.insert_resource(ShotRng::from_root(seed));
    app.insert_resource(SeverityRng::from_root(seed));
    app.insert_resource(LootRng::from_root(seed));
    app.insert_resource(InjuryRng::from_root(seed));
    app.insert_resource(ProcgenRng::from_root(seed));
    // GTW-438: `dispatch_fire` reads `Res<InjuryTables>` + `Res<InjuryRegistry>` — seed
    // empty ones (the AI fire path rolls no asserted injury; the roll still draws).
    app.insert_resource(InjuryTables::default());
    app.insert_resource(InjuryRegistry::default());
    let tuning = CombatTuning::default();
    app.insert_resource(FloorCostGrid::new(tuning.move_costs.open, []));
    app.insert_resource(tuning);
    app.insert_resource(PlayerFaction::new(PLAYER));
    app.insert_resource(ActiveFaction::new(ENEMY));
    // GTW-461: seed the act cooldown ready-to-act, with a ZERO cadence — so the existing
    // per-tick brain scenarios behave EXACTLY as before (recharge-to-0 = ready next tick =
    // the act-every-tick path). The dedicated cadence test (`cadence.rs`) overrides
    // ActCadence to a real positive value via `set_cadence` to prove the one-act-per-step
    // pacing.
    app.insert_resource(EnemyActCooldown::ready());
    app.insert_resource(ActCadence::new(0));
    app
}

/// Override the brain's [`ActCadence`] — the number of ticks the brain waits between enemy
/// acts (GTW-461). Used by the cadence test to set a real positive cadence on a
/// [`brain_app`]; the default harness seeds `0` (act-every-tick, the prior behaviour).
pub(super) fn set_cadence(app: &mut App, ticks: u32) {
    app.insert_resource(ActCadence::new(ticks));
}

/// The number of ticks remaining on the enemy-act cooldown (GTW-461) — `0` if the resource
/// is absent (no `unwrap`).
pub(super) fn cooldown_ticks(app: &App) -> u32 {
    app.world()
        .get_resource::<EnemyActCooldown>()
        .map_or(0, |cooldown| **cooldown)
}

/// A `(cell, level)` key on the ground floor.
pub(super) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// Spawn a full combatant ganger at `at`, of `faction`, facing `facing`, with a `tu` pool
/// (full — `tu_max == tu`) and a magazine of `ammo` rounds on a related weapon entity.
///
/// Carries the complete shooter + target component set `fire()` reads (the `acts`
/// `spawn_shooter` set) PLUS the [`Faction`] the brain splits enemies/targets on, and relates
/// a single weapon entity via [`WieldedBy`] (whose hook populates the ganger's `Wields`
/// synchronously in a bare-world spawn, so the very next dispatch resolves it). Arbitrary
/// magnitudes — never shipped tuning.
pub(super) fn spawn_combatant(
    world: &mut World,
    at: CellLevel,
    faction: Faction,
    facing: Direction,
    tu: u8,
    ammo: u16,
) -> Entity {
    let mode = FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(MODE_TU_PERCENT),
        ModeShots::new(1),
    );
    let mag_size = MagazineSize::new(30);
    let reload_tu = ReloadTu::new(12);
    let bundle = WeaponBundle::new(
        WeaponName::new("test-weapon".to_owned()),
        BaseSpread::new(0.05),
        Accuracy::new(2.0),
        Kickback::new(0.2),
        FatalBias::new(0.0),
        DamageProfile::new(
            WeaponDamage::new(40),
            WeaponPunch::new(20),
            WeaponShred::new(10),
            DamageType::Kinetic,
        ),
        HandlingProfile::new(
            Magazine::new(ammo, mag_size, reload_tu),
            FireMode::new(vec![mode]),
            Stable::new(true),
            Shove::new(false),
            Handedness::OneHanded,
        ),
    );
    let ganger = world
        .spawn((
            Position::new(at),
            Facing::new(facing),
            Stance::new(StanceKind::Standing),
            Aiming::new(false),
            Shooting::new(1.0),
            Tu::new(tu),
            TuMax::new(tu),
            faction,
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
    world.spawn((WieldedBy::new(ganger), bundle));
    ganger
}

/// Spawn a combatant exactly as [`spawn_combatant`], but wielding a weapon of the given
/// [`Handedness`] (GTW-443 C7) — the only difference is the weapon's handedness, so the
/// AI's shared `can_fire` hand-count gate can be exercised on the brain path.
pub(super) fn spawn_combatant_handed(
    world: &mut World,
    at: CellLevel,
    faction: Faction,
    facing: Direction,
    handedness: Handedness,
) -> Entity {
    let mode = FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(MODE_TU_PERCENT),
        ModeShots::new(1),
    );
    let bundle = WeaponBundle::new(
        WeaponName::new("test-weapon".to_owned()),
        BaseSpread::new(0.05),
        Accuracy::new(2.0),
        Kickback::new(0.2),
        FatalBias::new(0.0),
        DamageProfile::new(
            WeaponDamage::new(40),
            WeaponPunch::new(20),
            WeaponShred::new(10),
            DamageType::Kinetic,
        ),
        HandlingProfile::new(
            Magazine::new(6, MagazineSize::new(30), ReloadTu::new(12)),
            FireMode::new(vec![mode]),
            Stable::new(true),
            Shove::new(false),
            handedness,
        ),
    );
    let ganger = world
        .spawn((
            Position::new(at),
            Facing::new(facing),
            Stance::new(StanceKind::Standing),
            Aiming::new(false),
            Shooting::new(1.0),
            Tu::new(100),
            TuMax::new(100),
            faction,
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
    world.spawn((WieldedBy::new(ganger), bundle));
    ganger
}

/// Insert an [`InflictedInjuries`](crate::injuries::InflictedInjuries) ledger disabling
/// one hand (a `DisableHand` keyed to `part`) on `ganger` (GTW-443 C7) — so its derived
/// hand count drops, and the shared `can_fire` refuses a two-handed weapon.
pub(super) fn give_disabled_hand(world: &mut World, ganger: Entity, part: crate::armor::BodyPart) {
    use crate::injuries::{GainedInjury, InflictedInjuries, InjuryEffect, InjuryName, InspectText};
    let mut ledger = InflictedInjuries::default();
    ledger.gain(GainedInjury::new(
        InjuryName::new("disabled-hand".to_owned()),
        part,
        crate::severity::Severity::Major,
        vec![InjuryEffect::DisableHand],
        InspectText::new("a disabled hand".to_owned()),
    ));
    world.entity_mut(ganger).insert(ledger);
}

/// Register `entity` as a grid occupant at `at` with a HIGH silhouette band — so the LOS
/// march can strike it (a placed occupant carries its band from the first frame, GTW-304).
pub(super) fn place_occupant(app: &mut App, at: CellLevel, entity: Entity) {
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(at, Some(entity));
        grid.set_occupant_band(at, Some(HeightBand::High));
    }
}

/// The currently-active [`Faction`].
pub(super) fn active_of(app: &App) -> Faction {
    **app.world().resource::<ActiveFaction>()
}

/// A ganger's current [`Tu`] (0 if absent — no `unwrap`).
pub(super) fn tu_of(app: &App, entity: Entity) -> u8 {
    app.world().get::<Tu>(entity).map_or(0, |tu| **tu)
}

/// Drain the [`FireRequested`] messages emitted this run.
pub(super) fn drain_fires(app: &mut App) -> Vec<FireRequested> {
    app.world_mut()
        .resource_mut::<Messages<FireRequested>>()
        .drain()
        .collect()
}

/// Drain the [`MoveRequested`] messages emitted this run.
pub(super) fn drain_moves(app: &mut App) -> Vec<MoveRequested> {
    app.world_mut()
        .resource_mut::<Messages<MoveRequested>>()
        .drain()
        .collect()
}
