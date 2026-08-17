pub(super) use bevy::prelude::{App, Entity, Messages, MinimalPlugins, World};

pub(super) use crate::{
    acts::{FireRequested, MoveRequested, SimActsPlugin},
    cover::HeightBand,
    ganger::{
        Aiming, Direction, Facing, Faction, Fight, Hp, LifeState, Luck, Position, Shooting, Stance,
        StanceKind, Toughness, Tu, TuMax, Wounds,
    },
    inflicted_wound::InflictedWounds,
    magazine::{LoadedRounds, Magazine, ReloadTu},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    rng::{BattleSeed, FightRng},
    test_support::insert_sim_resources,
    turn::ActiveFaction,
    visibility::{OmniscientFog, SquadVisibility},
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        Handedness, HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots,
        ModeTuPercent, Shove, Stable, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch,
        WeaponShred, WieldedBy,
    },
};

const SEED: u64 = 0x5A1C_AC75;

pub(super) const PLAYER: Faction = Faction::new(0);
pub(super) const ENEMY: Faction = Faction::new(1);

const MODE_TU_PERCENT: f32 = 0.2;

pub(super) fn brain_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(SimActsPlugin);
    insert_sim_resources(&mut app, BattleSeed::new(SEED));
    app.insert_resource(FightRng::from_root(BattleSeed::new(SEED)));
    let omniscient = SquadVisibility::omniscient(&OccupancyGrid::new());
    app.insert_resource(omniscient.clone());
    app.insert_resource(OmniscientFog::new(omniscient));
    app.insert_resource(ActiveFaction::new(ENEMY));
    app
}

pub(super) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

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
            Magazine::new(LoadedRounds::new(ammo), mag_size, reload_tu),
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
            Fight::new(1.0),
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
            Magazine::new(
                LoadedRounds::new(6),
                MagazineSize::new(30),
                ReloadTu::new(12),
            ),
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
            Fight::new(1.0),
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

pub(super) fn place_occupant(app: &mut App, at: CellLevel, entity: Entity) {
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(at, Some(entity));
        grid.set_occupant_band(at, Some(HeightBand::High));
    }
}

pub(super) fn active_of(app: &App) -> Faction {
    **app.world().resource::<ActiveFaction>()
}

pub(super) fn tu_of(app: &App, entity: Entity) -> u8 {
    app.world().get::<Tu>(entity).map_or(0, |tu| **tu)
}

pub(super) fn drain_fires(app: &mut App) -> Vec<FireRequested> {
    app.world_mut()
        .resource_mut::<Messages<FireRequested>>()
        .drain()
        .collect()
}

pub(super) fn drain_moves(app: &mut App) -> Vec<MoveRequested> {
    app.world_mut()
        .resource_mut::<Messages<MoveRequested>>()
        .drain()
        .collect()
}
