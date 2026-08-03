use bevy::{ecs::entity::Entity, prelude::*, state::state::State};
use gdtf_app::test_support::{AppState, BattleScapeState, RunningState, WeaponContent};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    ganger::{Aiming, Facing, TuMax},
    injuries::InjuryRegistry,
    magazine::Magazine,
    prelude::{Cell, CellLevel, Direction, Faction, LifeState, Position, Stance, StanceKind, Tu},
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        Handedness, HandlingProfile, Kickback, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
        Shove, Stable, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch, WeaponShred,
    },
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

pub(crate) const BUDGET: u32 = 96;

pub(crate) fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

pub(crate) fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

pub(crate) fn battle_running_app() -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::WeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::MeleeWeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::equipment::attachments::AttachmentRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::terrain::def::TerrainDefRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::UuidThemeRegistry::default());

    let at_menu = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    assert!(at_menu, "the walk should reach RunningState::Menu");
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    let at_battle = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        at_battle,
        "the walk should reach BattleScapeState::BattleRunning; last was {:?}",
        battlescape_state(&app),
    );
    app
}

pub(crate) fn all_with<M: Component>(app: &mut App) -> Vec<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    q.iter(app.world()).collect()
}

pub(crate) fn single_with<M: Component>(app: &mut App) -> Option<Entity> {
    match all_with::<M>(app).as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

pub(crate) fn line_text<M: Component>(app: &mut App) -> Option<String> {
    let entity = single_with::<M>(app)?;
    app.world()
        .get::<Text>(entity)
        .map(|t| t.as_str().to_owned())
}

pub(crate) fn visibility<M: Component>(app: &mut App) -> Option<Visibility> {
    let entity = single_with::<M>(app)?;
    app.world().get::<Visibility>(entity).copied()
}

pub(crate) fn weapon_kit(name: &str, magazine: Magazine) -> WeaponBundle {
    WeaponBundle::new(
        WeaponName::new(name.to_owned()),
        BaseSpread::new(0.2),
        Accuracy::new(1.0),
        Kickback::new(0.1),
        FatalBias::new(0.0),
        DamageProfile::new(
            WeaponDamage::new(10),
            WeaponPunch::new(2),
            WeaponShred::new(1),
            DamageType::Kinetic,
        ),
        HandlingProfile::new(
            magazine,
            FireMode::new(vec![FireModeSpec::new(
                ModeKind::Single,
                ModeConeMult::new(1.0),
                ModeTuPercent::new(0.3),
                ModeShots::new(1),
            )]),
            Stable::new(false),
            Shove::new(false),
            Handedness::OneHanded,
        ),
    )
}

pub(crate) fn spawn_armed_and_select(app: &mut App, weapon: WeaponBundle) -> Entity {
    let ganger = app
        .world_mut()
        .spawn((
            Position::new(CellLevel::new(
                Cell::new(3, 3),
                gdtf_battle_sim::metric::Level::new(0),
            )),
            Faction::new(0),
            Facing::new(Direction::East),
            Stance::new(StanceKind::Standing),
            Aiming::new(false),
            Tu::new(100),
            TuMax::new(100),
            LifeState::Alive,
        ))
        .id();
    app.world_mut()
        .spawn((gdtf_battle_sim::weapon::WieldedBy::new(ganger), weapon));
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    ganger
}

pub(crate) fn spawn_unarmed_and_select(app: &mut App) -> Entity {
    let ganger = app
        .world_mut()
        .spawn((
            Position::new(CellLevel::new(
                Cell::new(4, 4),
                gdtf_battle_sim::metric::Level::new(0),
            )),
            Faction::new(0),
            Facing::new(Direction::East),
            Stance::new(StanceKind::Standing),
            Aiming::new(false),
            Tu::new(100),
            TuMax::new(100),
            LifeState::Alive,
        ))
        .id();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    ganger
}

pub(crate) fn content_node(app: &mut App) -> Option<Node> {
    let entity = single_with::<WeaponContent>(app)?;
    app.world().get::<Node>(entity).cloned()
}

pub(crate) fn node_of<M: Component>(app: &mut App) -> Option<Node> {
    let entity = single_with::<M>(app)?;
    app.world().get::<Node>(entity).cloned()
}

pub(crate) fn is_descendant_of(app: &App, descendant: Entity, ancestor: Entity) -> bool {
    let mut current = descendant;
    for _ in 0..32 {
        let Some(parent) = app.world().get::<ChildOf>(current) else {
            return false;
        };
        if parent.parent() == ancestor {
            return true;
        }
        current = parent.parent();
    }
    false
}
