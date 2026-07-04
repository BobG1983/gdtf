//! GTW-434: headless behavioral tests for the DEV-ONLY procgen STEP/AUTO visualizer.
//!
//! These run on the `MinimalPlugins` [`GdtfTestAppBuilder`] (the real state stack, `UiPlugin`,
//! and — in a debug build — the real `ProcgenVizScenePlugin` wired through `ScenesPlugin`).
//! They seed the persistent `Load` resources the visualizer reads (a theme, a theme+size-only
//! `LoadedSituation`, a [`PrefabRegistry`] with a player + enemy v2 prefab under the
//! situation's [`ThemeUuid`], and a FIXED `BattleSeed` so the assembled level is reproducible),
//! drive into [`RunningState::DebugProcgenVisualizer`](gdtf_app::test_support::RunningState), and
//! assert on the WORLD + the real visualizer model / entities — never on rendering (the
//! screenshot, C5, is the QA stage).
//!
//! GTW-492 (T07b): the visualizer drives the UUID-keyed v2 procgen pipeline, so the fixture
//! seeds a [`PrefabRegistry`] of [`Prefab`] (keyed by the situation's [`ThemeUuid`]).
//!
//! Coverage (C1/C2/C3):
//!
//! - [`step_reveals_one_more_quad`] — a press on the STEP button advances the model's revealed
//!   count by EXACTLY one (C1).
//! - [`auto_reveals_every_quad`] — a press on the AUTO button reveals the WHOLE placement
//!   sequence at once (C1).
//! - [`revealed_quads_carry_role_tints`] — after AUTO, the revealed quad entities carry the
//!   right tints: index 0 (player) = green, index 1 (enemy) = red, the rest = neutral (C3).
//!
//! The whole visualizer is `#[cfg(debug_assertions)]`-gated; these tests run under the dev /
//! gate suite (debug), where the feature is compiled in.

#![cfg(debug_assertions)]

use bevy::{
    ecs::{component::Component, entity::Entity},
    prelude::With,
    state::state::NextState,
};
use gdtf_app::test_support::{
    AppState, AutoButton, BoardQuad, EnemyGangDropdown, GenerateButton, HeightField, LevelsField,
    PlayerGangDropdown, PrefabQuad, ProcgenViz, ProcgenVizRoot, QuadTint, RunningState, SeedField,
    SizeStatusText, StepButton, ThemeDropdown, VizConfig, WidthField,
};
use gdtf_battle_sim::{
    Aim, ArmorName, Cool, GangMember, GangName, GangRegistry, GangRoster, GangerName, Grit, Luck,
    Reflexes, Speed, Strength, TerrainUuid, ThemeDisplayName, Toughness, UuidThemeDef,
    UuidThemeRegistry, WeaponName,
    level::{
        GridHeight, GridLevels, GridSize, GridWidth, Prefab, PrefabName, PrefabRegistry,
        PrefabSpec, SpawnRole, ThemeUuid,
    },
    rng::BattleSeed,
    situation::Situation,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until, press_ui_button};
use gdtf_ui::{
    CommittedNumericValue, DropdownSelectionChanged, NumericFieldCommitted, theme::default_theme,
};

/// A bounded budget for driving into the visualizer state.
const BUDGET: u32 = 64;

/// A FIXED injected seed so the assembled level (and therefore the quad count) is reproducible
/// across runs — the visualizer reads a `Res<BattleSeed>` override when present.
const TEST_SEED: u64 = 0x600D_5EED;

/// The canonical test theme key the seeded situation + v2 prefabs share, so the visualizer's
/// theme-keyed candidate lookup resolves the player + enemy prefabs (GTW-492).
const fn viz_theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0149_2434_0000_0001))
}

/// The 30x30x4 board the visualizer assembles into — large enough to fit the 12x12 player +
/// enemy deployment fragments at opposite corners (the `procgen_battle` precedent).
fn viz_board() -> GridSize {
    GridSize::new(GridWidth::new(30), GridHeight::new(30), GridLevels::new(4)).unwrap_or_default()
}

/// A `12x12x1` fragment footprint (the deployment-zone size). Falls back to the default extent
/// on a bad span (it cannot be bad — 12/12/1 are under the maxima).
fn fragment() -> GridSize {
    GridSize::new(GridWidth::new(12), GridHeight::new(12), GridLevels::new(1)).unwrap_or_default()
}

/// Build a [`PrefabRegistry`] with a player + enemy v2 prefab (each a 12x12 fragment, authoring
/// no placements — the visualizer reads only the placement-quad SEQUENCE, never the per-piece
/// geometry) under BOTH test themes (keyed by `(theme, size, role)`), so the visualizer's
/// `assemble_placement` has a real player- and enemy-role prefab to place WHICHEVER theme is
/// selected (C1 — a theme switch still places). An empty registry would show zero quads and
/// redden the assertions loudly.
fn viz_prefab_registry() -> PrefabRegistry {
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
fn viz_situation() -> Situation {
    let mut situation = Situation::new();
    situation.grid_size = viz_board();
    situation.theme = viz_theme();
    situation
}

/// A SECOND test theme key (distinct from [`viz_theme`]) the theme-dropdown C7 test selects to —
/// it has its own [`UuidThemeDef`] in the registry, so a selection of it is honoured by the
/// regenerate (the chosen theme keys procgen's candidate lookup, GTW-498 C1).
const fn other_theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0149_2434_0000_0002))
}

/// Build a [`UuidThemeRegistry`] holding BOTH test themes (each a display-named definition over a
/// nil terrain palette — the visualizer never reads a theme's terrain, only its KEY drives the
/// space packer), so the theme dropdown (C1) is populated and a selection resolves.
fn viz_theme_registry() -> UuidThemeRegistry {
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
fn player_gang_name() -> GangName {
    GangName::new("goliaths".to_owned())
}

/// A test gang KEY for the enemy roster (C4).
fn enemy_gang_name() -> GangName {
    GangName::new("eschers".to_owned())
}

/// One placeholder roster member (the eight attributes / weapon / armor keys are never read by
/// the visualizer — only the gang NAME + member COUNT label the deployment quad, C4).
fn member(name: &str) -> GangMember {
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
fn viz_gang_registry() -> GangRegistry {
    let mut registry = GangRegistry::default();
    registry.insert(
        player_gang_name(),
        GangRoster::new([member("ajax"), member("brawn")]),
    );
    registry.insert(enemy_gang_name(), GangRoster::new([member("vex")]));
    registry
}

/// Reads the current [`RunningState`] if active.
fn running_state(app: &bevy::app::App) -> Option<RunningState> {
    app.world()
        .get_resource::<bevy::state::state::State<RunningState>>()
        .map(|state| *state.get())
}

/// Builds a headless app driven into [`RunningState::DebugProcgenVisualizer`] with the screen
/// spawned + the model built: seed the theme + registry + situation + seed before the first
/// update, enter `Running` (rests on `Menu`), set the visualizer transition (the cfg-gated
/// button does this in the GUI; here we set it directly, the headless idiom), then pump updates
/// until the model is present.
fn viz_app() -> bevy::app::App {
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
fn single_with<M: Component>(app: &mut bevy::app::App) -> Option<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    let found: Vec<Entity> = q.iter(app.world()).collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// Reads the model's revealed count as a `usize` (0 if the model is absent) — deref-ing the
/// `RevealedCount` newtype the accessor returns.
fn revealed(app: &bevy::app::App) -> usize {
    app.world()
        .get_resource::<ProcgenViz>()
        .map(|model| *model.revealed())
        .unwrap_or_default()
}

/// Reads the model's total quad count (0 if the model is absent).
fn total(app: &bevy::app::App) -> usize {
    app.world()
        .get_resource::<ProcgenViz>()
        .map(|model| model.quads().len())
        .unwrap_or_default()
}

/// Injects a fresh `Interaction::Pressed` on the one button carrying marker `M`, then updates
/// once — driving the real `Changed<Interaction>` control reader (the GTW-420 editor precedent:
/// `MinimalPlugins` has no `ui_focus_system` to clear the injected press, so it persists for the
/// reader the same frame).
fn press_button<M: Component>(app: &mut bevy::app::App) {
    let button = single_with::<M>(app).unwrap_or(Entity::PLACEHOLDER);
    press_ui_button(app, button);
    app.update();
}

/// C2: the visualizer is reachable + spawns the screen root, and the model built a NON-EMPTY
/// placement sequence (so the STEP / AUTO assertions have quads to reveal). This is the
/// precondition the discriminating tests rely on.
#[test]
fn visualizer_builds_a_placement_sequence() {
    let mut app = viz_app();

    assert_eq!(
        running_state(&app),
        Some(RunningState::DebugProcgenVisualizer),
        "the app must rest in RunningState::DebugProcgenVisualizer after the transition",
    );
    assert!(
        single_with::<ProcgenVizRoot>(&mut app).is_some(),
        "OnEnter must spawn exactly one visualizer screen root (C2)",
    );
    // The single DARK whole-level board quad the light per-prefab quads draw over (C2).
    // `single_with` is `Some` only when EXACTLY one exists — so a dropped board quad (none)
    // or a duplicated one reddens this. The light quads are "tinted quads over a single dark
    // whole-level quad", so that dark quad must be present exactly once.
    assert!(
        single_with::<BoardQuad>(&mut app).is_some(),
        "OnEnter must spawn exactly one dark whole-level board quad (C2)",
    );
    // The real registry has a player + enemy prefab, so the placement sequence is at least the
    // player + enemy quads (fill may add more). A non-empty sequence proves procgen ran.
    assert!(
        total(&app) >= 2,
        "the visualizer must assemble a placement sequence of at least the player + enemy \
         quads (procgen ran against the real registry); total was {}",
        total(&app),
    );
    // Nothing is revealed yet (the reveal starts at zero — STEP / AUTO drive it).
    assert_eq!(revealed(&app), 0, "the reveal count must start at zero");
}

/// C1: a press on the STEP button advances the revealed count by EXACTLY one.
///
/// Pin: a dropped `Changed<Interaction>` read, a no-op step, or an over-advance reddens the
/// `before + 1` assertion.
#[test]
fn step_reveals_one_more_quad() {
    let mut app = viz_app();
    let before = revealed(&app);
    assert_eq!(before, 0, "precondition: nothing revealed at entry");

    press_button::<StepButton>(&mut app);

    assert_eq!(
        revealed(&app),
        before + 1,
        "a STEP press must reveal exactly one more quad (C1)",
    );

    // A second STEP advances by one again (the control is repeatable).
    press_button::<StepButton>(&mut app);
    assert_eq!(
        revealed(&app),
        before + 2,
        "a second STEP press must reveal one more quad again (C1)",
    );
}

/// C1: a press on the AUTO button reveals the WHOLE placement sequence at once.
///
/// Pin: a partial reveal (revealed != total) or a no-op reddens the assertion.
#[test]
fn auto_reveals_every_quad() {
    let mut app = viz_app();
    let total = total(&app);
    assert!(total >= 2, "precondition: a non-empty placement sequence");
    assert_eq!(revealed(&app), 0, "precondition: nothing revealed at entry");

    press_button::<AutoButton>(&mut app);

    assert_eq!(
        revealed(&app),
        total,
        "an AUTO press must reveal the whole placement sequence at once (C1)",
    );
}

/// C3: after AUTO reveals every quad, the revealed quad ENTITIES carry the right tint roles —
/// index 0 (player) = green, index 1 (enemy) = red, every other (fill) = neutral.
///
/// Asserts on the REAL `PrefabQuad` components (not a reimplementation): a wrong projection
/// (player not green, enemy not red, or a fill tinted as a spawn) reddens it.
#[test]
fn revealed_quads_carry_role_tints() {
    let mut app = viz_app();
    press_button::<AutoButton>(&mut app);

    // Collect every per-prefab quad's (index, tint) from the real entities — deref-ing the
    // `RevealIndex` newtype the accessor returns to its `usize` position.
    let mut quads: Vec<(usize, QuadTint)> = {
        let mut q = app.world_mut().query::<&PrefabQuad>();
        q.iter(app.world())
            .map(|quad| (*quad.index(), quad.tint()))
            .collect()
    };
    quads.sort_by_key(|(index, _)| *index);

    assert!(
        quads.len() >= 2,
        "the visualizer must have spawned a quad per placement entry (>= player + enemy); \
         found {}",
        quads.len(),
    );

    // Index 0 is the player spawn (green); index 1 the enemy spawn (red); the rest neutral.
    assert_eq!(
        quads.first().map(|(_, tint)| *tint),
        Some(QuadTint::Player),
        "the FIRST quad (index 0) is the player spawn — tinted green (C3)",
    );
    assert_eq!(
        quads.get(1).map(|(_, tint)| *tint),
        Some(QuadTint::Enemy),
        "the SECOND quad (index 1) is the enemy spawn — tinted red (C3)",
    );
    for (index, tint) in quads.iter().skip(2) {
        assert_eq!(
            *tint,
            QuadTint::Neutral,
            "every fill quad (index {index}) carries the neutral tint (C3)",
        );
    }
}

// ---------------------------------------------------------------------------------------------
// GTW-498 — the configurable inputs + Generate (C1–C5), driven on the REAL widget-message path.
// ---------------------------------------------------------------------------------------------

/// Write a real [`NumericFieldCommitted`]`<`[`u8`]`>` for the field carrying marker `M` (the
/// grid-axis commit the keyboard observer raises on Enter / blur), then update once — driving the
/// real `apply_size_commit` listener.
fn commit_u8_field<M: Component>(app: &mut bevy::app::App, value: u8) {
    let field = single_with::<M>(app).unwrap_or(Entity::PLACEHOLDER);
    app.world_mut().write_message(NumericFieldCommitted::new(
        field,
        CommittedNumericValue::new(value),
    ));
    app.update();
}

/// Write a real [`NumericFieldCommitted`]`<`[`u64`]`>` for the seed field, then update once —
/// driving the real `apply_seed_commit` listener.
fn commit_seed(app: &mut bevy::app::App, value: u64) {
    let field = single_with::<SeedField>(app).unwrap_or(Entity::PLACEHOLDER);
    app.world_mut().write_message(NumericFieldCommitted::new(
        field,
        CommittedNumericValue::new(value),
    ));
    app.update();
}

/// Write a real [`DropdownSelectionChanged`]`<`[`ThemeUuid`]`>` for the theme dropdown, then
/// update once — driving the real `apply_theme_selection` listener.
fn select_theme(app: &mut bevy::app::App, theme: ThemeUuid) {
    let control = single_with::<ThemeDropdown>(app).unwrap_or(Entity::PLACEHOLDER);
    app.world_mut()
        .write_message(DropdownSelectionChanged::new(control, theme));
    app.update();
}

/// Write a real [`DropdownSelectionChanged`]`<`[`GangName`]`>` for the gang dropdown carrying
/// marker `M`, then update once — driving the real `apply_gang_selection` listener.
fn select_gang<M: Component>(app: &mut bevy::app::App, gang: GangName) {
    let control = single_with::<M>(app).unwrap_or(Entity::PLACEHOLDER);
    app.world_mut()
        .write_message(DropdownSelectionChanged::new(control, gang));
    app.update();
}

/// The current [`VizConfig`], if present (the `OnEnter` insert guarantees it; callers `assert!`
/// on the `Some` so a dropped config reddens loudly without an `expect`).
fn config(app: &bevy::app::App) -> Option<VizConfig> {
    app.world().get_resource::<VizConfig>().cloned()
}

/// The current config's selected theme, if the config is present.
fn config_theme(app: &bevy::app::App) -> Option<ThemeUuid> {
    config(app).map(|c| c.theme())
}

/// Whether the current config's size combo validates (`false` if the config is absent).
fn config_size_valid(app: &bevy::app::App) -> bool {
    config(app).is_some_and(|c| c.grid_size().is_ok())
}

/// Whether the (single) Generate button currently carries the
/// [`DisabledButton`](gdtf_ui::DisabledButton) marker — `false` if it is enabled or absent.
fn generate_disabled(app: &mut bevy::app::App) -> bool {
    single_with::<GenerateButton>(app)
        .is_some_and(|e| app.world().get::<gdtf_ui::DisabledButton>(e).is_some())
}

/// The board's `(width, height)` cell dimensions of the current model.
fn board_dims(app: &bevy::app::App) -> (u32, u32) {
    app.world()
        .get_resource::<ProcgenViz>()
        .map(ProcgenViz::board_dimensions)
        .unwrap_or_default()
}

/// The display label of the model's quad at `index`, if any.
fn quad_label(app: &bevy::app::App, index: usize) -> Option<String> {
    app.world()
        .get_resource::<ProcgenViz>()
        .and_then(|model| model.quad_label_at(index))
}

/// C6: the INITIAL state (before any Generate) seeds the config from the loaded situation +
/// default seed — so the panel opens on the same inputs the prior GTW-434 build used.
///
/// Pin: a wrong initial theme / size (e.g. defaulting instead of reading the situation) reddens.
#[test]
fn initial_config_seeds_from_situation() {
    let app = viz_app();
    let Some(config) = config(&app) else {
        unreachable!("the VizConfig must be inserted OnEnter (C6)")
    };
    assert_eq!(
        config.theme(),
        viz_theme(),
        "the initial config theme must be the loaded situation's theme (C6)",
    );
    let Ok(size) = config.grid_size() else {
        unreachable!("the initial size (from the 30x30x4 situation) is valid (C6)")
    };
    assert_eq!(
        (*size.width(), *size.height(), *size.levels()),
        (30, 30, 4),
        "the initial config size must be the loaded situation's grid-size (C6)",
    );
    // The injected BattleSeed override seeds the config seed (the reproducible-harness path).
    assert_eq!(
        *config.seed(),
        TEST_SEED,
        "the initial config seed must be the injected BattleSeed override (C3/C6)",
    );
}

/// C1/C2/C5: editing theme + size, then pressing Generate, regenerates the model from those
/// inputs — the regenerated board reflects the chosen size, and a DIFFERENT theme is honoured.
///
/// Pin: an input that does not drive a regenerate (the feature-completeness rule), or a Generate
/// that ignores the config, reddens the board-dimension assertion.
#[test]
fn generate_rebuilds_from_theme_and_size() {
    let mut app = viz_app();
    let before = board_dims(&app);
    assert_eq!(before, (30, 30), "precondition: the initial board is 30x30");

    // Choose a different theme + a different board (still large enough to fit the 12x12 player +
    // enemy fragments at opposite corners) on the real widget-message path.
    select_theme(&mut app, other_theme());
    commit_u8_field::<WidthField>(&mut app, 40);
    commit_u8_field::<HeightField>(&mut app, 44);
    commit_u8_field::<LevelsField>(&mut app, 2);

    // The selections updated the config but NOT yet the rendered level (apply-on-Generate, C5).
    assert_eq!(
        config_theme(&app),
        Some(other_theme()),
        "selecting a theme updates the config (C1)",
    );
    assert_eq!(
        board_dims(&app),
        before,
        "selecting inputs must NOT regenerate until Generate is pressed (C5)",
    );

    press_button::<GenerateButton>(&mut app);

    assert_eq!(
        board_dims(&app),
        (40, 44),
        "Generate must rebuild the model from the chosen size (C2/C5)",
    );
    // The model is non-empty (the chosen theme keyed real player + enemy prefabs — C1).
    assert!(
        total(&app) >= 2,
        "the regenerated level under the chosen theme must place player + enemy (C1)",
    );
    // Generate resets the reveal to zero (C5).
    assert_eq!(
        revealed(&app),
        0,
        "Generate must reset the reveal to zero (C5)"
    );
}

/// C3/C5: regeneration is DETERMINISTIC per seed — two DIFFERENT seeds yield DIFFERENT placements
/// while the SAME seed reproduces the placement.
///
/// Pin: a seed that does not drive the RNG (a constant placement), or a non-deterministic build,
/// reddens. Asserts a structural fact (the ordered quad rectangles differ / match), never a
/// tunable magnitude.
#[test]
fn generate_is_deterministic_per_seed() {
    /// The ordered placement-RECTANGLE fingerprint of the current model — the player + enemy
    /// quad rectangles, which the seed's anchor draw moves (a structural signature, not a
    /// tunable magnitude).
    fn fingerprint(app: &bevy::app::App) -> Vec<(u32, u32, u32, u32)> {
        let n = total(app);
        (0..n)
            .filter_map(|i| {
                app.world()
                    .get_resource::<ProcgenViz>()
                    .and_then(|model| model.quad_rect_at(i))
            })
            .collect()
    }

    let mut app = viz_app();

    // Generate across a SPREAD of seeds. The player anchor is an RNG draw, so the placement
    // moves with the seed — across several seeds NOT ALL placements can be identical (proving
    // the seed drives the placement, C3), without assuming any single pair differs.
    let seeds: [u64; 6] = [0x1111, 0x2222, 0x3333, 0x4444, 0x5555, 0x6666];
    let mut prints: Vec<Vec<(u32, u32, u32, u32)>> = Vec::new();
    for seed in seeds {
        commit_seed(&mut app, seed);
        press_button::<GenerateButton>(&mut app);
        prints.push(fingerprint(&app));
    }
    assert!(
        prints.iter().all(|p| !p.is_empty()),
        "precondition: every regenerated level is non-empty",
    );
    let distinct: std::collections::HashSet<_> = prints.iter().cloned().collect();
    assert!(
        distinct.len() > 1,
        "different seeds must yield different placements — the seed drives procgen (C3); all \
         {} seeds produced the SAME placement {:?}",
        seeds.len(),
        prints.first(),
    );

    // The SAME seed reproduces the placement (re-generate the first seed, expect its print).
    commit_seed(&mut app, seeds[0]);
    press_button::<GenerateButton>(&mut app);
    assert_eq!(
        fingerprint(&app),
        prints[0],
        "the SAME seed must reproduce the placement (C3)",
    );
}

/// C2: an INVALID size combo is rejected without panic — the Generate button is DISABLED, the
/// status readout shows the error, and a Generate press leaves the model unchanged (so an
/// invalid size never regenerates).
///
/// Drives a real `NumericFieldCommitted<u8>` of ZERO for the width axis (a zero axis is what
/// `GridSize::new` rejects), then asserts the disable + skip path on the REAL listeners. Pin: a
/// missing disable, a status that does not surface the error, or a Generate that rebuilds anyway
/// reddens.
#[test]
fn invalid_size_is_rejected_without_panic() {
    let mut app = viz_app();
    let before = board_dims(&app);
    let before_total = total(&app);

    // A zero WIDTH makes the combo invalid (GridSize::new rejects a zero axis) WITHOUT panicking.
    commit_u8_field::<WidthField>(&mut app, 0);

    assert!(
        !config_size_valid(&app),
        "a zero-width combo must be rejected by GridSize::new — surfaced as Err, never a panic (C2)",
    );
    // The status readout surfaces the rejection (not `OK`).
    let Some(status) = single_with::<SizeStatusText>(&mut app) else {
        unreachable!("the panel spawns exactly one status text")
    };
    let status_text = app
        .world()
        .get::<bevy::prelude::Text>(status)
        .map(|t| t.0.clone())
        .unwrap_or_default();
    assert!(
        !status_text.starts_with("OK"),
        "the status readout must surface the invalid size (not OK); was {status_text:?}",
    );
    // Generate is DISABLED for the invalid combo.
    assert!(
        generate_disabled(&mut app),
        "Generate must be DISABLED for an invalid size combo (C2)",
    );
    // A Generate press leaves the model unchanged (the disabled button + the in-system re-check
    // both guard it — an invalid size NEVER regenerates).
    press_button::<GenerateButton>(&mut app);
    assert_eq!(
        board_dims(&app),
        before,
        "an invalid size must leave the model board unchanged (C2)",
    );
    assert_eq!(
        total(&app),
        before_total,
        "an invalid size must leave the model quads unchanged (C2)",
    );

    // Restoring a valid width re-enables Generate (the disable is not sticky).
    commit_u8_field::<WidthField>(&mut app, 30);
    assert!(
        config_size_valid(&app),
        "a restored valid width makes the combo valid again (C2)",
    );
    assert!(
        !generate_disabled(&mut app),
        "Generate must RE-ENABLE once the size combo is valid again (C2)",
    );
}

/// C5: after a Generate, the EXISTING STEP / AUTO controls step through the NEW model — STEP
/// reveals one more, AUTO reveals them all.
///
/// Pin: a Generate that does not reset the reveal, or quads that were not re-spawned for the new
/// model, reddens (STEP / AUTO would act on a stale model or stale entities).
#[test]
fn step_and_auto_drive_the_regenerated_model() {
    let mut app = viz_app();
    // Regenerate to a different board (still large enough to fit the 12x12 fragments) so the new
    // model is distinct from the OnEnter one.
    commit_u8_field::<WidthField>(&mut app, 40);
    commit_u8_field::<HeightField>(&mut app, 40);
    press_button::<GenerateButton>(&mut app);

    let total_after = total(&app);
    assert!(
        total_after >= 2,
        "the regenerated model has quads to reveal"
    );
    assert_eq!(revealed(&app), 0, "Generate reset the reveal to zero (C5)");

    press_button::<StepButton>(&mut app);
    assert_eq!(
        revealed(&app),
        1,
        "STEP reveals one more quad on the NEW model (C5)",
    );

    press_button::<AutoButton>(&mut app);
    assert_eq!(
        revealed(&app),
        total_after,
        "AUTO reveals every quad of the NEW model (C5)",
    );

    // The re-spawned per-prefab quad entities match the new model's quad count (re-spawn ran).
    let mut q = app.world_mut().query::<&PrefabQuad>();
    let spawned = q.iter(app.world()).count();
    assert_eq!(
        spawned, total_after,
        "the per-prefab quad ENTITIES were re-spawned for the new model (C5)",
    );
}

/// C4: chosen player / enemy gangs LABEL the player (index 0) and enemy (index 1) deployment
/// quads with the gang name + member count, after a Generate.
///
/// Pin: a gang that does not annotate the quad, the wrong quad annotated, or a missing member
/// count, reddens. Asserts the label CONTAINS the gang name (a structural fact, not an exact
/// layout string).
#[test]
fn chosen_gangs_label_the_deployment_quads() {
    let mut app = viz_app();
    select_gang::<PlayerGangDropdown>(&mut app, player_gang_name());
    select_gang::<EnemyGangDropdown>(&mut app, enemy_gang_name());

    let Some(config) = config(&app) else {
        unreachable!("the VizConfig must be present (C4)")
    };
    assert_eq!(
        config.player_gang(),
        Some(&player_gang_name()),
        "selecting a player gang updates the config (C4)",
    );

    press_button::<GenerateButton>(&mut app);

    let player_label = quad_label(&app, 0).unwrap_or_default();
    let enemy_label = quad_label(&app, 1).unwrap_or_default();
    assert!(
        player_label.contains("goliaths") && player_label.contains('2'),
        "the player deployment quad (index 0) must be labelled with the chosen gang name + its \
         member count (C4); was {player_label:?}",
    );
    assert!(
        enemy_label.contains("eschers") && enemy_label.contains('1'),
        "the enemy deployment quad (index 1) must be labelled with the chosen gang name + its \
         member count (C4); was {enemy_label:?}",
    );
}

/// C4: an EMPTY gang registry does not panic — the visualizer is still reachable, Generate still
/// works, and the deployment quads fall back to prefab-name labels (no chosen gang).
#[test]
fn empty_gang_registry_does_not_panic() {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(viz_prefab_registry());
    app.world_mut().insert_resource(viz_theme_registry());
    // EMPTY gang registry — the no-gang fallback (C4).
    app.world_mut().insert_resource(GangRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_app::test_support::LoadedSituation::new(viz_situation()));
    app.world_mut().insert_resource(BattleSeed::new(TEST_SEED));

    app.update();
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::DebugProcgenVisualizer);
    advance_until(
        &mut app,
        |app| {
            running_state(app) == Some(RunningState::DebugProcgenVisualizer)
                && app.world().get_resource::<ProcgenViz>().is_some()
        },
        BUDGET,
    );
    app.update();

    // Generate works with no gangs chosen + an empty registry (never panics, C4).
    press_button::<GenerateButton>(&mut app);
    assert!(
        total(&app) >= 2,
        "the visualizer regenerates with an empty gang registry (C4 no-gang fallback)",
    );
    // The player quad falls back to its prefab name (the player_deployment prefab), not a gang.
    let player_label = quad_label(&app, 0).unwrap_or_default();
    assert!(
        player_label.contains("player_deployment"),
        "with no chosen gang the player quad labels by prefab name (C4); was {player_label:?}",
    );
}
