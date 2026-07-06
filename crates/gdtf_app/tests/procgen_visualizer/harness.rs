//! Fixture registries, the driven visualizer app, and shared reveal counters.

use bevy::{
    ecs::{component::Component, entity::Entity},
    prelude::With,
    state::state::NextState,
};
use gdtf_app::test_support::{AppState, ProcgenViz, RunningState};
use gdtf_battle_sim::{
    armor::ArmorName,
    def::TerrainUuid,
    ganger::{
        Aim, Cool, GangMember, GangName, GangRegistry, GangRoster, GangerName, Grit, Luck,
        Reflexes, Speed, Strength, Toughness,
    },
    level::{
        GridHeight, GridLevels, GridSize, GridWidth, Prefab, PrefabName, PrefabRegistry,
        PrefabSpec, SpawnRole, ThemeDisplayName, ThemeUuid, UuidThemeDef, UuidThemeRegistry,
    },
    rng::BattleSeed,
    situation::Situation,
    weapon::WeaponName,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until, press_ui_button};
use gdtf_ui::theme::default_theme;

/// A bounded budget for driving into the visualizer state.
pub(crate) const BUDGET: u32 = 64;

/// A FIXED injected seed so the assembled level (and therefore the quad count) is reproducible
/// across runs — the visualizer reads a `Res<BattleSeed>` override when present.
pub(crate) const TEST_SEED: u64 = 0x600D_5EED;

/// The canonical test theme key the seeded situation + v2 prefabs share, so the visualizer's
/// theme-keyed candidate lookup resolves the player + enemy prefabs (GTW-492).
pub(crate) const fn viz_theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0149_2434_0000_0001))
}

/// The 30x30x4 board the visualizer assembles into — large enough to fit the 12x12 player +
/// enemy deployment fragments at opposite corners (the `procgen_battle` precedent).
pub(crate) fn viz_board() -> GridSize {
    GridSize::new(GridWidth::new(30), GridHeight::new(30), GridLevels::new(4)).unwrap_or_default()
}

/// A `12x12x1` fragment footprint (the deployment-zone size). Falls back to the default extent
/// on a bad span (it cannot be bad — 12/12/1 are under the maxima).
pub(crate) fn fragment() -> GridSize {
    GridSize::new(GridWidth::new(12), GridHeight::new(12), GridLevels::new(1)).unwrap_or_default()
}

/// Build a [`PrefabRegistry`] with a player + enemy v2 prefab (each a 12x12 fragment, authoring
/// no placements — the visualizer reads only the placement-quad SEQUENCE, never the per-piece
/// geometry) under BOTH test themes (keyed by `(theme, size, role)`), so the visualizer's
/// `assemble_placement` has a real player- and enemy-role prefab to place WHICHEVER theme is
/// selected (C1 — a theme switch still places). An empty registry would show zero quads and
/// redden the assertions loudly.
pub(crate) fn viz_prefab_registry() -> PrefabRegistry {
    let fp = fragment();
    let mut registry = PrefabRegistry::default();
    for theme in [viz_theme(), other_theme()] {
        registry.insert(Prefab::new(
            PrefabName::new("player_deployment".to_owned()),
            PrefabSpec::new(theme, fp, SpawnRole::Player, Vec::new()),
        ));
        registry.insert(Prefab::new(
            PrefabName::new("enemy_deployment".to_owned()),
            PrefabSpec::new(theme, fp, SpawnRole::Enemy, Vec::new()),
        ));
    }
    registry
}

/// A theme+size [`Situation`] the visualizer reads for its theme + grid-size: the 30x30x4 board
/// under [`viz_theme`], no terrain / gangers (the visualizer never reads those — it only runs
/// the space packer). GTW-492: the visualizer's `build` reads `Situation.theme` (the
/// UUID-keyed `ThemeUuid`) directly, so the theme MUST match the seeded prefabs' theme.
pub(crate) fn viz_situation() -> Situation {
    let mut situation = Situation::new();
    situation.grid_size = viz_board();
    situation.theme = viz_theme();
    situation
}

/// A SECOND test theme key (distinct from [`viz_theme`]) the theme-dropdown C7 test selects to —
/// it has its own [`UuidThemeDef`] in the registry, so a selection of it is honoured by the
/// regenerate (the chosen theme keys procgen's candidate lookup, GTW-498 C1).
pub(crate) const fn other_theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0149_2434_0000_0002))
}

/// Build a [`UuidThemeRegistry`] holding BOTH test themes (each a display-named definition over a
/// nil terrain palette — the visualizer never reads a theme's terrain, only its KEY drives the
/// space packer), so the theme dropdown (C1) is populated and a selection resolves.
pub(crate) fn viz_theme_registry() -> UuidThemeRegistry {
    let mut registry = UuidThemeRegistry::default();
    registry.insert(
        viz_theme(),
        UuidThemeDef {
            key:           viz_theme(),
            display_name:  ThemeDisplayName::new("Primary".to_owned()),
            default_floor: TerrainUuid::new(bevy::asset::uuid::Uuid::nil()),
            terrain:       Vec::new(),
        },
    );
    registry.insert(
        other_theme(),
        UuidThemeDef {
            key:           other_theme(),
            display_name:  ThemeDisplayName::new("Secondary".to_owned()),
            default_floor: TerrainUuid::new(bevy::asset::uuid::Uuid::nil()),
            terrain:       Vec::new(),
        },
    );
    registry
}

/// A test gang KEY for the player roster (C4).
pub(crate) fn player_gang_name() -> GangName {
    GangName::new("goliaths".to_owned())
}

/// A test gang KEY for the enemy roster (C4).
pub(crate) fn enemy_gang_name() -> GangName {
    GangName::new("eschers".to_owned())
}

/// One placeholder roster member (the eight attributes / weapon / armor keys are never read by
/// the visualizer — only the gang NAME + member COUNT label the deployment quad, C4).
pub(crate) fn member(name: &str) -> GangMember {
    GangMember {
        name:         GangerName::new(name.to_owned()),
        speed:        Speed::new(3.0),
        aim:          Aim::new(3.0),
        strength:     Strength::new(3.0),
        toughness:    Toughness::new(3.0),
        reflexes:     Reflexes::new(3.0),
        cool:         Cool::new(3.0),
        grit:         Grit::new(3.0),
        luck:         Luck::new(3.0),
        armor:        ArmorName::new("flak".to_owned()),
        weapon:       WeaponName::new("autopistol".to_owned()),
        melee_weapon: None,
    }
}

/// Build a [`GangRegistry`] with the player gang (TWO members) + enemy gang (ONE member), so the
/// C4 deployment-quad annotations carry distinct names + counts.
pub(crate) fn viz_gang_registry() -> GangRegistry {
    let mut registry = GangRegistry::default();
    registry.insert(
        player_gang_name(),
        GangRoster::new([member("ajax"), member("brawn")]),
    );
    registry.insert(enemy_gang_name(), GangRoster::new([member("vex")]));
    registry
}

/// Reads the current [`RunningState`] if active.
pub(crate) fn running_state(app: &bevy::app::App) -> Option<RunningState> {
    app.world()
        .get_resource::<bevy::state::state::State<RunningState>>()
        .map(|state| *state.get())
}

/// Builds a headless app driven into [`RunningState::DebugProcgenVisualizer`] with the screen
/// spawned + the model built: seed the theme + registry + situation + seed before the first
/// update, enter `Running` (rests on `Menu`), set the visualizer transition (the cfg-gated
/// button does this in the GUI; here we set it directly, the headless idiom), then pump updates
/// until the model is present.
pub(crate) fn viz_app() -> bevy::app::App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(viz_prefab_registry());
    // GTW-498: the input panel reads the theme + gang registries for its dropdown options.
    app.world_mut().insert_resource(viz_theme_registry());
    app.world_mut().insert_resource(viz_gang_registry());
    // The visualizer reads theme + grid-size from the loaded situation; insert it directly as
    // the persistent resource (the procgen_battle harness precedent).
    app.world_mut()
        .insert_resource(gdtf_app::test_support::LoadedSituation::new(viz_situation()));
    // A fixed seed so the assembled level is reproducible.
    app.world_mut().insert_resource(BattleSeed::new(TEST_SEED));

    // Enter Running (rests on Menu).
    app.update();
    // Drive Menu -> DebugProcgenVisualizer.
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::DebugProcgenVisualizer);
    // Apply the transition + run OnEnter (build model + spawn screen), then settle a frame.
    advance_until(
        &mut app,
        |app| {
            running_state(app) == Some(RunningState::DebugProcgenVisualizer)
                && app.world().get_resource::<ProcgenViz>().is_some()
        },
        BUDGET,
    );
    app.update();
    app
}

/// The single entity carrying marker `M`, if exactly one exists.
pub(crate) fn single_with<M: Component>(app: &mut bevy::app::App) -> Option<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    let found: Vec<Entity> = q.iter(app.world()).collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// Reads the model's revealed count as a `usize` (0 if the model is absent) — deref-ing the
/// `RevealedCount` newtype the accessor returns.
pub(crate) fn revealed(app: &bevy::app::App) -> usize {
    app.world()
        .get_resource::<ProcgenViz>()
        .map(|model| *model.revealed())
        .unwrap_or_default()
}

/// Reads the model's total quad count (0 if the model is absent).
pub(crate) fn total(app: &bevy::app::App) -> usize {
    app.world()
        .get_resource::<ProcgenViz>()
        .map(|model| model.quads().len())
        .unwrap_or_default()
}

/// Injects a fresh `Interaction::Pressed` on the one button carrying marker `M`, then updates
/// once — driving the real `Changed<Interaction>` control reader (the GTW-420 editor precedent:
/// `MinimalPlugins` has no `ui_focus_system` to clear the injected press, so it persists for the
/// reader the same frame).
pub(crate) fn press_button<M: Component>(app: &mut bevy::app::App) {
    let button = single_with::<M>(app).unwrap_or(Entity::PLACEHOLDER);
    press_ui_button(app, button);
    app.update();
}
