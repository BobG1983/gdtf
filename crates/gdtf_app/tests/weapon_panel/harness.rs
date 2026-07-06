//! App-driving harness, query vocabulary, armed/unarmed fixtures, and the shared UI-tree-walk helpers.

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

/// A budget large enough to drive the deep walk into the battlescape, bounded so a machine
/// that never reaches the predicate fails instead of hanging.
pub(crate) const BUDGET: u32 = 96;

/// Reads the current [`BattleScapeState`] if active.
pub(crate) fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Reads the current [`RunningState`] if active.
pub(crate) fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Drives the real stack to `BattleScapeState::BattleRunning`, where the weapon panel is live.
pub(crate) fn battle_running_app() -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::WeaponRegistry::default());
    // GTW-505: the Load->Intro gate also requires a MeleeWeaponRegistry (empty-default
    // seed for this ganger-free / hand-seeded harness, mirroring the WeaponRegistry seed).
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::MeleeWeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::equipment::attachments::AttachmentRegistry::default());
    // GTW-269: setup_battle_on_request now reads an ArmorRegistry to armor each ganger,
    // failing closed (no BattleReady) without one. This panel harness builds a
    // ganger-free default battle, so an empty registry suffices — it just must be
    // present for the setup to run and reach BattleRunning.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    // GTW-415: the Load→Intro gate also requires a GangRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    // GTW-489: the NEW gate-blocking PrefabRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry::default());
    // GTW-487: the NEW gate-blocking TerrainDefRegistry + UuidThemeRegistry.
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

/// All entities carrying marker `M`.
pub(crate) fn all_with<M: Component>(app: &mut App) -> Vec<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    q.iter(app.world()).collect()
}

/// The single entity carrying marker `M`, or `None` if not exactly one.
pub(crate) fn single_with<M: Component>(app: &mut App) -> Option<Entity> {
    match all_with::<M>(app).as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// The rendered `Text` of the single entity carrying marker `M`.
pub(crate) fn line_text<M: Component>(app: &mut App) -> Option<String> {
    let entity = single_with::<M>(app)?;
    app.world()
        .get::<Text>(entity)
        .map(|t| t.as_str().to_owned())
}

/// The [`Visibility`] of the single entity carrying marker `M`.
pub(crate) fn visibility<M: Component>(app: &mut App) -> Option<Visibility> {
    let entity = single_with::<M>(app)?;
    app.world().get::<Visibility>(entity).copied()
}

/// A weapon kit (the `Magazine` grouping + the rest) — built with an EXPLICIT magazine so
/// the test pins a known `loaded`/`size`. Arbitrary magnitudes (not shipped tuning).
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

/// Spawns an armed ganger wielding `weapon` on a related WEAPON entity (`Wields`,
/// GTW-323 slice 3 — the panel reads the weapon's `WeaponName`/`Magazine` off the weapon
/// entity through `ganger → Wields → weapon`, not the ganger), SELECTS the ganger, and
/// returns its entity. The `WieldedBy` insert hook populates the ganger's `Wields`
/// synchronously in a bare `World` spawn. (The cursor-click selection is covered in
/// `gdtf_battle_input`; the panel only reads `*SelectedShooter` + the wielded weapon.)
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

/// Spawns an UNARMED ganger (no weapon components), SELECTS it, and returns its entity.
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

/// The content column's [`Node`], if present.
pub(crate) fn content_node(app: &mut App) -> Option<Node> {
    let entity = single_with::<WeaponContent>(app)?;
    app.world().get::<Node>(entity).cloned()
}

/// The [`Node`] of the single entity carrying marker `M`, if present.
pub(crate) fn node_of<M: Component>(app: &mut App) -> Option<Node> {
    let entity = single_with::<M>(app)?;
    app.world().get::<Node>(entity).cloned()
}

/// `true` when `descendant` has `ancestor` somewhere on its parent chain (walking
/// [`ChildOf`](bevy::prelude::ChildOf) upward). Used to assert the Stance Panel is laid out
/// INSIDE the bottom bar (D4) rather than being a free-floating sibling overlay.
pub(crate) fn is_descendant_of(app: &App, descendant: Entity, ancestor: Entity) -> bool {
    let mut current = descendant;
    // Bounded walk (a UI tree is shallow) so a malformed cycle can never hang the test.
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
