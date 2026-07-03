//! GTW-278 / GTW-274 — the battlescape status panel + inspect panel (twins), driven
//! through the REAL app stack.
//!
//! These headless `GdtfTestAppBuilder` integration tests drive the genuine state machine
//! down to `BattleScapeState::BattleRunning`, where the real status-panel + inspect-panel
//! plugins spawn their shared stat blocks and their update systems repaint them under the
//! `BattleInProgress` gate. They cover:
//!
//! - **AC1 (status)** — the status panel renders the selected ganger's NAME +
//!   TU/HP `ProgressBar`s + Wounds `Pips` + wound-name list; the removed `LifeState` / `Weapon`
//!   lines are GONE; selecting a different ganger MUTATES in place.
//! - **Portrait** — the portrait node carries a `TextureAtlas` at the DETERMINISTIC index
//!   for the ganger's name (computed in-test from the same rule); a different name mutates
//!   the index on the SAME node.
//! - **AC2 (hover)** — `InspectTarget` over a ganger → its stat block; over a cover/object →
//!   the object block; over bare floor → the panel is hidden.

use bevy::{
    ecs::entity::Entity,
    image::TextureAtlas,
    prelude::*,
    state::state::State,
    text::TextColor,
    ui::{Display, Val, widget::ImageNode},
};
use gdtf_app::test_support::{
    AppState, BattleScapeState, InspectObjectBar, InspectObjectBlock, InspectObjectHardness,
    InspectObjectHeight, InspectObjectProtection, InspectObjectText, InspectPanelRoot,
    InspectStatBlockHost, RunningState, StatHpBar, StatHpLabel, StatInjuryLine, StatInjuryList,
    StatName, StatPortrait, StatTuBar, StatTuLabel, StatWoundLine, StatWoundList, StatWoundsPips,
    portrait_index_for_name,
};
use gdtf_battle_input::{InputSystems, InspectTarget, SelectedShooter, pick_hovered_cell};
use gdtf_battle_sim::{
    ArmorHardness, ArmorProtection, BodyPart, Cell, CellLevel, CoverEntry, CoverHp, CoverLedger,
    Destroyed, Faction, GainedInjury, GangerName, HeightBand, Hp, HpMax, InflictedInjuries,
    InflictedWound, InflictedWounds, InjuryName, InspectText, Level, LifeState, OccupancyGrid,
    PlayerFaction, Position, Severity, SquadVisibility, Stance, StanceKind, TerrainKind, Tu, TuMax,
    Wounds, WoundsMax, injuries::InjuryRegistry, tuning::CombatTuning, weapon::WeaponRegistry,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, GdtfTestAppBuilder, advance_until};
use gdtf_ui::{
    Pip, ProgressBarFill,
    theme::{GdtfTheme, default_theme},
};

/// A budget large enough to drive the deep walk into the battlescape, bounded so a machine
/// that never reaches the predicate fails instead of hanging.
const BUDGET: u32 = 96;

// ---------------------------------------------------------------------------------
// Harness — drive the real stack to BattleRunning, where the panels are live.
// ---------------------------------------------------------------------------------

/// Reads the current [`BattleScapeState`] if it is active.
fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Reads the current [`RunningState`] if it is active.
fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Builds the headless walk app, injecting the persistent `Load` resources (theme +
/// tuning + an empty weapon registry) the machine needs to traverse `Load`. No
/// `LoadedSituation` → the empty `Situation::default()` battle is set up, which still makes
/// `BattleInProgress` + `OccupancyGrid` present in `BattleRunning`.
fn walk_app() -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
    // GTW-505: the Load->Intro gate also requires a MeleeWeaponRegistry (empty-default
    // seed stands in for the asset-less resolve, mirroring the WeaponRegistry seed above).
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::MeleeWeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::AttachmentRegistry::default());
    // GTW-269: the Load gate also requires an ArmorRegistry; empty clears it.
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
    app
}

/// Drives the app from `Running`/`Menu` down to `BattleScapeState::BattleRunning`.
fn drive_to_battle_running(app: &mut App) -> bool {
    let at_menu = advance_until(
        app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    if !at_menu {
        return false;
    }
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    advance_until(
        app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    )
}

/// Drives the walk to `BattleRunning` and returns the app, asserting the descent succeeded.
fn battle_running_app() -> App {
    let mut app = walk_app();
    assert!(
        drive_to_battle_running(&mut app),
        "the walk should reach BattleScapeState::BattleRunning within {BUDGET} updates; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    app
}

/// A larger budget for the real `DefaultPlugins` async asset loads + the full state descent
/// (the `real_battle_panel.rs` precedent) when the portrait atlas must actually load.
const LOAD_BUDGET: u32 = 512;

/// Drives the REAL `DefaultPlugins` asset stack (`GdtfLoadTestAppBuilder`, a live
/// `AssetServer` rooted at the workspace `assets/`) from `Load` down to `BattleRunning`,
/// asserting the descent. UNLIKE [`battle_running_app`] (`MinimalPlugins`, no `AssetServer`),
/// here `TopDownRendererPlugin`'s `Startup` `load_topdown_atlases` runs for real, so the
/// `OnEnter(BattleRunning)` panel spawn reads a present `TopDownAtlases` and the portrait
/// node carries a `TextureAtlas` over the portraits sheet — the harness the portrait
/// node-wiring assertions need (the `atlas_load.rs` / `real_battle_panel.rs` pattern).
fn load_battle_running_app() -> App {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    let at_menu = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        LOAD_BUDGET,
    );
    assert!(
        at_menu,
        "the real Load + descent must reach RunningState::Menu within {LOAD_BUDGET} updates",
    );
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    let at_battle = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        LOAD_BUDGET,
    );
    assert!(
        at_battle,
        "the real battle must reach BattleScapeState::BattleRunning within {LOAD_BUDGET} \
         updates; last BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    app
}

/// Whether `entity` has an ancestor carrying marker `R` (walks the `ChildOf` chain up).
fn descends_from<R: Component>(app: &App, entity: Entity) -> bool {
    let mut current = entity;
    loop {
        if app.world().get::<R>(current).is_some() {
            return true;
        }
        match app.world().get::<bevy::prelude::ChildOf>(current) {
            Some(parent) => current = parent.parent(),
            None => return false,
        }
    }
}

/// All entities carrying marker `M`.
fn all_with<M: Component>(app: &mut App) -> Vec<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    q.iter(app.world()).collect()
}

/// The single entity carrying marker `M`, scoped to the STATUS panel (NOT a descendant of
/// the inspect panel root). The two panels share the stat-block markers, so this discriminates
/// the status panel's widget from the inspect panel's.
fn single_with<M: Component>(app: &mut App) -> Option<Entity> {
    let found: Vec<Entity> = all_with::<M>(app)
        .into_iter()
        .filter(|&e| !descends_from::<InspectPanelRoot>(app, e))
        .collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// The single entity carrying INSPECT-EXCLUSIVE marker `M` (only the inspect panel carries it,
/// so no scoping is needed — assert there is exactly one).
fn single_global<M: Component>(app: &mut App) -> Option<Entity> {
    match all_with::<M>(app).as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// The single entity carrying SHARED stat-block marker `M`, scoped to the INSPECT panel (a
/// descendant of the inspect panel root) — discriminates the inspect stat block's widget from
/// the status panel's.
fn single_inspect<M: Component>(app: &mut App) -> Option<Entity> {
    let found: Vec<Entity> = all_with::<M>(app)
        .into_iter()
        .filter(|&e| descends_from::<InspectPanelRoot>(app, e))
        .collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// Reads the rendered `Text` of the single entity carrying marker `M`.
fn line_text<M: Component>(app: &mut App) -> Option<String> {
    let entity = single_with::<M>(app)?;
    app.world()
        .get::<Text>(entity)
        .map(|t| t.as_str().to_owned())
}

/// The layout [`Display`] of the single entity carrying INSPECT-EXCLUSIVE marker `M` — the
/// GTW-295 discriminating signal for a sub-block's show (`Display::Flex`) / hide
/// (`Display::None`, removed from layout). `None` if the node is missing.
fn display_of<M: Component>(app: &mut App) -> Option<Display> {
    let entity = single_global::<M>(app)?;
    app.world().get::<Node>(entity).map(|n| n.display)
}

/// The rendered `Text` of the single entity carrying INSPECT-EXCLUSIVE marker `M`.
fn global_line_text<M: Component>(app: &mut App) -> Option<String> {
    let entity = single_global::<M>(app)?;
    app.world()
        .get::<Text>(entity)
        .map(|t| t.as_str().to_owned())
}

/// The fill PERCENT of the `ProgressBar` rooted at `track` — reads the `ProgressBarFill`
/// child's `Node.width`. `None` if missing.
fn bar_fill_at(app: &App, track: Entity) -> Option<f32> {
    let kids: Vec<Entity> = app
        .world()
        .get::<Children>(track)
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    for kid in kids {
        if app.world().get::<ProgressBarFill>(kid).is_some()
            && let Some(node) = app.world().get::<Node>(kid)
            && let Val::Percent(p) = node.width
        {
            return Some(p);
        }
    }
    None
}

/// The fill PERCENT of the status panel's `ProgressBar` carrying track-marker `M`.
fn bar_fill_percent<M: Component>(app: &mut App) -> Option<f32> {
    let track = single_with::<M>(app)?;
    bar_fill_at(app, track)
}

/// The number of VISIBLE filled pips (background != the lost color, visibility not Hidden)
/// in the single pips row carrying marker `M`. We count visible pips whose visibility is not
/// Hidden — a discriminating proxy for `WoundsMax` shown / `Wounds` filled.
fn visible_pip_count<M: Component>(app: &mut App) -> usize {
    let Some(row) = single_with::<M>(app) else {
        return 0;
    };
    let kids: Vec<Entity> = app
        .world()
        .get::<Children>(row)
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    kids.into_iter()
        .filter(|&kid| {
            app.world().get::<Pip>(kid).is_some()
                && app.world().get::<Visibility>(kid) != Some(&Visibility::Hidden)
        })
        .count()
}

/// The portrait node's current atlas index (the `TextureAtlas.index` on the single
/// `StatPortrait` `ImageNode`). `None` if the portrait has no atlas (sheet unloaded).
fn portrait_index(app: &mut App) -> Option<usize> {
    let portrait = single_with::<StatPortrait>(app)?;
    app.world()
        .get::<ImageNode>(portrait)
        .and_then(|n| n.texture_atlas.as_ref().map(|a: &TextureAtlas| a.index))
}

/// A ganger's vitals for a test spawn — grouped into one struct (the `too_many_arguments`
/// idiom).
struct GangerSetup {
    cell:       Cell,
    level:      Level,
    name:       GangerName,
    faction:    Faction,
    stance:     StanceKind,
    tu:         Tu,
    tu_max:     TuMax,
    hp:         Hp,
    hp_max:     HpMax,
    wounds:     Wounds,
    wounds_max: WoundsMax,
    life:       LifeState,
    inflicted:  InflictedWounds,
    injuries:   InflictedInjuries,
}

/// Spawns a ganger with the components the stat block reads, SELECTS it, and returns its
/// entity. (The cursor-click selection is covered in `gdtf_battle_input`; the panel only
/// reads `*SelectedShooter` + the on-entity components.)
fn spawn_and_select(app: &mut App, setup: GangerSetup) -> Entity {
    let ganger = app
        .world_mut()
        .spawn((
            Position::new(CellLevel::new(setup.cell, setup.level)),
            setup.name,
            setup.faction,
            Stance::new(setup.stance),
            setup.tu,
            setup.tu_max,
            setup.hp,
            setup.hp_max,
            setup.wounds,
            setup.wounds_max,
            setup.life,
            setup.inflicted,
            setup.injuries,
        ))
        .id();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    ganger
}

/// A reasonable default ganger setup the caller overrides per test.
fn default_setup() -> GangerSetup {
    GangerSetup {
        cell:       Cell::new(5, 6),
        level:      Level::new(0),
        name:       GangerName::new("Vex Harker".to_owned()),
        faction:    Faction::new(1),
        stance:     StanceKind::Crouching,
        tu:         Tu::new(7),
        tu_max:     TuMax::new(10),
        hp:         Hp::new(8),
        hp_max:     HpMax::new(16),
        wounds:     Wounds::new(2),
        wounds_max: WoundsMax::new(3),
        life:       LifeState::Alive,
        inflicted:  InflictedWounds::default(),
        injuries:   InflictedInjuries::default(),
    }
}

// ---------------------------------------------------------------------------------
// AC1 — status panel renders the shared stat block; LifeState/Weapon GONE.
// ---------------------------------------------------------------------------------

/// AC1 — with a selected ganger, the stat block shows its name + a NON-zero TU/HP bar fill +
/// the right Wounds pips; and there is NO life-state / weapon text line (the removed lines).
#[test]
fn status_panel_renders_the_selected_ganger_stat_block() {
    let mut app = battle_running_app();
    spawn_and_select(&mut app, default_setup());
    app.update();

    let name = line_text::<StatName>(&mut app).unwrap_or_default();
    assert!(name.contains("Vex Harker"), "name title: {name}");

    let tu = bar_fill_percent::<StatTuBar>(&mut app).unwrap_or(0.0);
    assert!((tu - 70.0).abs() < 0.5, "TU bar = 7/10 = 70% (got {tu})");

    let hp = bar_fill_percent::<StatHpBar>(&mut app).unwrap_or(0.0);
    assert!((hp - 50.0).abs() < 0.5, "HP bar = 8/16 = 50% (got {hp})");

    // WoundsMax = 3 visible pips, Wounds = 2 filled (we assert the visible count == 3).
    assert_eq!(
        visible_pip_count::<StatWoundsPips>(&mut app),
        3,
        "WoundsMax (3) pips must be visible",
    );

    // The removed LifeState + Weapon lines: NO text line anywhere reads them.
    let texts: Vec<String> = {
        let mut q = app.world_mut().query::<&Text>();
        q.iter(app.world()).map(|t| t.as_str().to_owned()).collect()
    };
    assert!(
        !texts
            .iter()
            .any(|t| t.contains("State:") || t.starts_with("Weapon:")),
        "the LifeState + Weapon lines must be GONE (texts: {texts:?})",
    );
}

/// GTW-310 — the HP bar and TU bar each carry a numeric `cur/max` label that reads the
/// seeded ganger's `Hp`/`HpMax` and `Tu`/`TuMax` exactly, and a value change MUTATES the
/// label in place (no respawn). The displayed number must equal current/max.
#[test]
fn stat_block_shows_numeric_hp_and_tu() {
    let mut app = battle_running_app();
    // default_setup: Tu 7/10, Hp 8/16.
    spawn_and_select(&mut app, default_setup());
    app.update();

    let tu = line_text::<StatTuLabel>(&mut app).unwrap_or_default();
    assert_eq!(tu, "7/10", "the TU label reads Tu/TuMax (got {tu})");
    let hp = line_text::<StatHpLabel>(&mut app).unwrap_or_default();
    assert_eq!(hp, "8/16", "the HP label reads Hp/HpMax (got {hp})");

    // A changed selection mutates the SAME label entities in place.
    let tu_label_before = single_with::<StatTuLabel>(&mut app);
    let hp_label_before = single_with::<StatHpLabel>(&mut app);
    let mut other = default_setup();
    other.name = GangerName::new("Alex Mercer".to_owned());
    other.tu = Tu::new(3);
    other.tu_max = TuMax::new(12);
    other.hp = Hp::new(5);
    other.hp_max = HpMax::new(20);
    spawn_and_select(&mut app, other);
    app.update();

    assert_eq!(
        single_with::<StatTuLabel>(&mut app),
        tu_label_before,
        "the TU label entity is stable across a selection change (mutate, no respawn)",
    );
    assert_eq!(
        single_with::<StatHpLabel>(&mut app),
        hp_label_before,
        "the HP label entity is stable across a selection change (mutate, no respawn)",
    );
    assert_eq!(
        line_text::<StatTuLabel>(&mut app).unwrap_or_default(),
        "3/12",
        "the TU label mutated to the new ganger's Tu/TuMax",
    );
    assert_eq!(
        line_text::<StatHpLabel>(&mut app).unwrap_or_default(),
        "5/20",
        "the HP label mutated to the new ganger's Hp/HpMax",
    );
}

/// AC1 — an unwounded ganger hides the wound-name list; a wounded ganger shows it with the
/// matching "{tier} — {location}" entry.
#[test]
fn wound_list_reflects_inflicted_wounds() {
    let mut app = battle_running_app();

    // Unwounded -> the list container is Hidden.
    spawn_and_select(&mut app, default_setup());
    app.update();
    let list = single_with::<StatWoundList>(&mut app);
    assert!(list.is_some(), "the stat block carries a wound list");
    if let Some(list) = list {
        assert_eq!(
            app.world().get::<Visibility>(list),
            Some(&Visibility::Hidden),
            "an unwounded ganger hides the wound-name list",
        );
    }

    // Wounded -> the list is shown and a line reads the wound.
    let mut wounded = default_setup();
    wounded.name = GangerName::new("Alex Mercer".to_owned());
    wounded.inflicted = InflictedWounds::new(vec![InflictedWound::new(
        Severity::Minor,
        BodyPart::LeftArm,
    )]);
    spawn_and_select(&mut app, wounded);
    app.update();

    if let Some(list) = single_with::<StatWoundList>(&mut app) {
        assert_ne!(
            app.world().get::<Visibility>(list),
            Some(&Visibility::Hidden),
            "a wounded ganger shows the wound-name list",
        );
    }
    // Some wound line reads "Minor — Left Arm".
    let lines: Vec<String> = {
        let mut q = app
            .world_mut()
            .query_filtered::<&Text, With<StatWoundLine>>();
        q.iter(app.world()).map(|t| t.as_str().to_owned()).collect()
    };
    assert!(
        lines
            .iter()
            .any(|l| l.contains("Minor") && l.contains("Left Arm")),
        "a wound line must read the inflicted wound (lines: {lines:?})",
    );
}

/// GTW-439 (C3) — the injury-name list is driven by the DURABLE `InflictedInjuries` ledger
/// (the persistent per-ganger list), NOT the transient `InjuryInflicted` message: an
/// uninjured ganger HIDES the list; a ganger whose ledger carries a `GainedInjury` SHOWS it
/// with a line reading that injury's authored `inspect_text`. PIN-DISCRIMINATING — the line
/// must read the exact authored text (a list driven by the wrong source, or not driven at
/// all, fails the content + visibility asserts).
#[test]
fn injury_list_reflects_inflicted_injuries() {
    let mut app = battle_running_app();

    // No injuries -> the list container is Hidden.
    spawn_and_select(&mut app, default_setup());
    app.update();
    let list = single_with::<StatInjuryList>(&mut app);
    assert!(list.is_some(), "the stat block carries an injury list");
    if let Some(list) = list {
        assert_eq!(
            app.world().get::<Visibility>(list),
            Some(&Visibility::Hidden),
            "an uninjured ganger hides the injury-name list",
        );
    }

    // Injured -> the list is shown and a line reads the durable inspect_text.
    let mut injured = default_setup();
    injured.name = GangerName::new("Alex Mercer".to_owned());
    let mut ledger = InflictedInjuries::default();
    ledger.gain(GainedInjury::new(
        InjuryName::new("Lost Eye".to_owned()),
        BodyPart::Head,
        Severity::Critical,
        Vec::new(),
        InspectText::new("Lost Eye -- -2 Aim, -1 Cool".to_owned()),
    ));
    injured.injuries = ledger;
    spawn_and_select(&mut app, injured);
    app.update();

    if let Some(list) = single_with::<StatInjuryList>(&mut app) {
        assert_ne!(
            app.world().get::<Visibility>(list),
            Some(&Visibility::Hidden),
            "an injured ganger shows the injury-name list",
        );
    }
    // Some injury line reads the authored inspect_text verbatim (the persistent ledger source).
    let lines: Vec<String> = {
        let mut q = app
            .world_mut()
            .query_filtered::<&Text, With<StatInjuryLine>>();
        q.iter(app.world()).map(|t| t.as_str().to_owned()).collect()
    };
    assert!(
        lines.iter().any(|l| l == "Lost Eye -- -2 Aim, -1 Cool"),
        "an injury line must read the gained injury's authored inspect_text (lines: {lines:?})",
    );
}

/// AC1 — selecting a DIFFERENT ganger MUTATES the same stat block (stable entity ids — the
/// portrait/name nodes are the SAME entities, their content changes).
#[test]
fn selection_change_mutates_in_place() {
    let mut app = battle_running_app();
    spawn_and_select(&mut app, default_setup());
    app.update();
    let portrait_before = single_with::<StatPortrait>(&mut app);
    let name_before = single_with::<StatName>(&mut app);

    let mut other = default_setup();
    other.name = GangerName::new("Alex Mercer".to_owned());
    other.tu = Tu::new(2);
    spawn_and_select(&mut app, other);
    app.update();

    assert_eq!(
        single_with::<StatPortrait>(&mut app),
        portrait_before,
        "the portrait node entity is stable across a selection change (mutate, no respawn)",
    );
    assert_eq!(
        single_with::<StatName>(&mut app),
        name_before,
        "the name node entity is stable across a selection change",
    );
    let name = line_text::<StatName>(&mut app).unwrap_or_default();
    assert!(name.contains("Alex Mercer"), "the name mutated: {name}");
}

/// AC4-parity — no selection shows the empty state (name = "No ganger selected", bars empty)
/// and never stale data.
#[test]
fn no_selection_shows_empty_state() {
    let mut app = battle_running_app();
    app.world_mut().insert_resource(SelectedShooter::cleared());
    app.update();
    let name = line_text::<StatName>(&mut app).unwrap_or_default();
    assert!(name.contains("No ganger selected"), "empty state: {name}");
    let tu = bar_fill_percent::<StatTuBar>(&mut app).unwrap_or(-1.0);
    assert!(
        tu.abs() < 0.5,
        "the TU bar is empty (0%) with no selection (got {tu})"
    );
}

// ---------------------------------------------------------------------------------
// Portrait — deterministic atlas index from the ganger name.
// ---------------------------------------------------------------------------------

/// The portrait node carries a `TextureAtlas` pointing at the portraits sheet, at the
/// DETERMINISTIC index for the ganger's name (computed in-test from the same rule), and
/// selecting a different-named ganger MUTATES the index on the SAME node (no respawn).
///
/// Driven on the REAL `DefaultPlugins` asset stack ([`load_battle_running_app`]) so
/// `load_topdown_atlases` actually runs and the `OnEnter(BattleRunning)` panel spawn gives
/// the portrait node a `TextureAtlas` over the portraits sheet — the assertions run
/// UNCONDITIONALLY (no `if let Some` guard), so a reverted `update_portrait` (or a missing
/// atlas) would FAIL this test rather than silently skip it.
#[test]
fn portrait_index_is_deterministic_for_the_selected_ganger() {
    let mut app = load_battle_running_app();

    let vex_name = GangerName::new("Vex Harker".to_owned());
    let expected_vex = portrait_index_for_name(Some(&vex_name));
    spawn_and_select(&mut app, default_setup());
    app.update();

    let portrait_node = single_with::<StatPortrait>(&mut app);
    assert!(
        portrait_node.is_some(),
        "the stat block carries a portrait node"
    );

    // The node carries a TextureAtlas over the portraits sheet (the real atlas loaded).
    let index = portrait_index(&mut app);
    assert!(
        index.is_some(),
        "the portrait node must carry a TextureAtlas (the portraits sheet loaded)",
    );
    let Some(index) = index else { return };
    assert_eq!(
        index, expected_vex,
        "the portrait index is the name's deterministic face"
    );

    // A different-named ganger MUTATES the index on the SAME node.
    let mut alex = default_setup();
    let alex_name = GangerName::new("Alex Mercer".to_owned());
    alex.name = alex_name.clone();
    let expected_alex = portrait_index_for_name(Some(&alex_name));
    // The two authored names must derive DISTINCT faces, else the mutation is unobservable
    // (the derivation determinism itself is unit-tested in `stat_block/test.rs`).
    assert_ne!(
        expected_alex, expected_vex,
        "the two test names must map to distinct portrait faces for the mutation to be visible",
    );
    spawn_and_select(&mut app, alex);
    app.update();
    assert_eq!(
        single_with::<StatPortrait>(&mut app),
        portrait_node,
        "the portrait node entity is stable (mutate, no respawn)",
    );
    assert_eq!(
        portrait_index(&mut app),
        Some(expected_alex),
        "the portrait index mutated to the new name's deterministic face",
    );
}

// ---------------------------------------------------------------------------------
// AC2 (hover) — the inspect panel inspects what the cursor hovers.
// ---------------------------------------------------------------------------------

/// A test-controlled desired hover cell, copied into `InspectTarget` AFTER the headless picker
/// runs (which would otherwise clobber an injected value to `None`).
#[derive(Resource, Clone, Copy, Default)]
struct DesiredHover(Option<CellLevel>);

/// Copies [`DesiredHover`] into [`InspectTarget`]'s live hovered cell — registered
/// `.after(pick_hovered_cell)` in `InputSystems::Gather`, so it is the LAST hovered-cell writer
/// of the frame and the `.after(Gather)` inspect-panel update reads it. Writes ONLY the hovered
/// cell (via `set_hovered`), leaving any pin untouched (GTW-300). The cursor→cell pick is the one
/// external this stubs (it is tested in `gdtf_battle_input`); everything downstream is the real
/// path.
fn force_hover(desired: Res<DesiredHover>, mut target: ResMut<InspectTarget>) {
    target.set_hovered(desired.0);
}

/// Builds the battle app with the hover-forcing seam wired in.
fn hover_app() -> App {
    let mut app = battle_running_app();
    app.world_mut().insert_resource(DesiredHover::default());
    app.add_systems(
        Update,
        force_hover
            .in_set(InputSystems::Gather)
            .after(pick_hovered_cell),
    );
    app
}

/// Sets the desired hover cell and steps one update so the real `update_inspect_panel` reads it.
fn hover(app: &mut App, cell: Option<CellLevel>) {
    app.world_mut().insert_resource(DesiredHover(cell));
    app.update();
}

/// Replaces the squad fog so EXACTLY the given cells are currently squad-VISIBLE (GTW-378).
///
/// The fog gate now suppresses the inspect panel for a fog-hidden enemy occupant, so the
/// hover tests that inspect an ENEMY ganger must seed its cell as squad-VISIBLE (the
/// real default-situation fog leaves arbitrary spawn cells UNSEEN). The VISIBLE set is its
/// own EXPLORED superset (the accrual invariant), matching `recompute_visibility`'s output.
fn make_cells_visible(app: &mut App, cells: &[CellLevel]) {
    let visible: bevy::platform::collections::HashSet<CellLevel> = cells.iter().copied().collect();
    let explored = visible.clone();
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible, explored));
}

/// AC2 — hovering a GANGER cell shows the panel + its ganger stat block.
#[test]
fn hovering_a_ganger_shows_its_stat_block() {
    let mut app = hover_app();
    let cell = CellLevel::new(Cell::new(4, 4), Level::new(0));

    // Spawn a ganger and put it on the occupancy grid at `cell`.
    let ganger = app
        .world_mut()
        .spawn((
            Position::new(cell),
            GangerName::new("Vex Harker".to_owned()),
            Faction::new(1),
            Stance::new(StanceKind::Standing),
            Tu::new(5),
            TuMax::new(10),
            Hp::new(10),
            HpMax::new(10),
            Wounds::new(3),
            WoundsMax::new(3),
            LifeState::Alive,
            InflictedWounds::default(),
        ))
        .id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    // GTW-378 — the inspect panel now refuses a fog-hidden enemy; make the cell squad-VISIBLE
    // so this AC2 case (a SEEN enemy shows its stat block) holds under the new fog gate.
    make_cells_visible(&mut app, &[cell]);

    hover(&mut app, Some(cell));

    // The panel root is visible and the ganger stat-block host is visible.
    let root = single_global::<InspectPanelRoot>(&mut app);
    assert!(root.is_some(), "the inspect panel must exist");
    if let Some(root) = root {
        assert_ne!(
            app.world().get::<Visibility>(root),
            Some(&Visibility::Hidden),
            "hovering a ganger shows the inspect panel",
        );
    }
    let host = single_global::<InspectStatBlockHost>(&mut app);
    assert!(
        host.is_some(),
        "the inspect panel must carry a ganger stat block"
    );
    // GTW-295 — the sub-blocks show/hide via Node.display (Flex/None), so a hidden block is
    // removed from layout and never balloons the panel.
    assert_eq!(
        display_of::<InspectStatBlockHost>(&mut app),
        Some(Display::Flex),
        "hovering a ganger shows the ganger stat block (Display::Flex)",
    );
    assert_eq!(
        display_of::<InspectObjectBlock>(&mut app),
        Some(Display::None),
        "hovering a ganger removes the object block from layout (Display::None)",
    );

    // The ganger stat block's name reads the hovered ganger (the shared stat block, scoped
    // to the inspect panel).
    if let Some(name) = single_inspect::<StatName>(&mut app) {
        let text = app
            .world()
            .get::<Text>(name)
            .map(|t| t.as_str().to_owned())
            .unwrap_or_default();
        assert!(
            text.contains("Vex Harker"),
            "inspect stat block names the ganger: {text}"
        );
    }

    // GTW-310 — the SHARED stat block's numeric HP/TU labels also render in the INSPECT
    // panel (one change to the shared block updates both panels). The hovered ganger has
    // Tu 5/10 and Hp 10/10.
    if let Some(tu_label) = single_inspect::<StatTuLabel>(&mut app) {
        let text = app
            .world()
            .get::<Text>(tu_label)
            .map(|t| t.as_str().to_owned())
            .unwrap_or_default();
        assert_eq!(text, "5/10", "inspect stat block TU label reads Tu/TuMax");
    }
    if let Some(hp_label) = single_inspect::<StatHpLabel>(&mut app) {
        let text = app
            .world()
            .get::<Text>(hp_label)
            .map(|t| t.as_str().to_owned())
            .unwrap_or_default();
        assert_eq!(text, "10/10", "inspect stat block HP label reads Hp/HpMax");
    }
}

/// GTW-378 (regression) — hovering a FOG-HIDDEN enemy (an enemy occupant on a cell that is
/// NOT currently squad-VISIBLE) does NOT populate the inspect panel (the info-leak); making
/// the SAME cell squad-VISIBLE then DOES show it.
///
/// Pin-discriminating: it fails if the panel populates in fog (the leak the GTW-378 fix
/// closes) AND it fails if the visible-cell positive control never shows (proving the gate is
/// not merely hiding everything). Hovers a single enemy ganger across the two fog states.
#[test]
fn fog_hidden_enemy_is_not_inspected_but_a_seen_one_is() {
    let mut app = hover_app();

    // The player faction the real default-situation setup seeded.
    let Some(player) = app
        .world()
        .get_resource::<PlayerFaction>()
        .copied()
        .map(|p| **p)
    else {
        return;
    };
    let enemy_faction = Faction::new(player.wrapping_add(1));
    let cell = CellLevel::new(Cell::new(4, 4), Level::new(0));
    place_ganger(&mut app, cell, enemy_faction);

    // FOG case — the cell is NOT squad-VISIBLE (empty VISIBLE set): the panel must stay hidden.
    make_cells_visible(&mut app, &[]);
    hover(&mut app, Some(cell));
    let root = single_global::<InspectPanelRoot>(&mut app);
    assert!(root.is_some(), "the inspect panel must exist");
    assert_eq!(
        root.and_then(|root| app.world().get::<Visibility>(root)),
        Some(&Visibility::Hidden),
        "hovering a FOG-HIDDEN enemy must NOT populate the inspect panel (GTW-378 info-leak)",
    );

    // VISIBLE case (positive control) — make the SAME cell squad-VISIBLE: the panel now shows.
    make_cells_visible(&mut app, &[cell]);
    hover(&mut app, Some(cell));
    if let Some(root) = single_global::<InspectPanelRoot>(&mut app) {
        assert_ne!(
            app.world().get::<Visibility>(root),
            Some(&Visibility::Hidden),
            "hovering a SEEN enemy still shows the inspect panel (GTW-378 positive control)",
        );
    }
    assert_eq!(
        display_of::<InspectStatBlockHost>(&mut app),
        Some(Display::Flex),
        "a squad-VISIBLE enemy lays out the ganger stat block (GTW-378 positive control)",
    );
}

/// Spawns a ganger of `faction` at `cell`, puts it on the occupancy grid, and returns it.
fn place_ganger(app: &mut App, cell: CellLevel, faction: Faction) -> Entity {
    let ganger = app
        .world_mut()
        .spawn((
            Position::new(cell),
            GangerName::new("Vex Harker".to_owned()),
            faction,
            Stance::new(StanceKind::Standing),
            Tu::new(5),
            TuMax::new(10),
            Hp::new(10),
            HpMax::new(10),
            Wounds::new(3),
            WoundsMax::new(3),
            LifeState::Alive,
            InflictedWounds::default(),
        ))
        .id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    ganger
}

/// The `TextColor` of the inspect panel's name line, if present.
fn inspect_name_color(app: &mut App) -> Option<TextColor> {
    let name = single_inspect::<StatName>(app)?;
    app.world().get::<TextColor>(name).copied()
}

/// AC2 — the inspect panel tints the NAME line by the hovered ganger's `Faction`: an ENEMY
/// (faction != `PlayerFaction`) gets a red-ish tint, a PLAYER-faction ganger gets the normal
/// theme text color. This is the AC2 panel/name COLOR tint (distinct from the "Gang N" text
/// line). Pin-discriminating: with no tint applied, the enemy name would stay the theme color
/// and the assertion fails.
#[test]
fn hovering_a_ganger_tints_the_name_by_faction() {
    let mut app = hover_app();

    // The player faction the real default-situation setup seeded (gang 0).
    let player_res = app
        .world()
        .get_resource::<PlayerFaction>()
        .copied()
        .map(|p| **p);
    assert!(player_res.is_some(), "a live battle inserts PlayerFaction");
    // The normal (player) name color is the runtime theme's body-text color.
    let normal_res = app
        .world()
        .get_resource::<GdtfTheme>()
        .map(|t| *t.text.text_color);
    assert!(normal_res.is_some(), "a live battle has the loaded theme");
    let (Some(player), Some(normal)) = (player_res, normal_res) else {
        return;
    };

    // An ENEMY ganger (a faction the player does NOT control) → red-ish tint, NOT the theme.
    let enemy_faction = Faction::new(player.wrapping_add(1));
    let enemy_cell = CellLevel::new(Cell::new(4, 4), Level::new(0));
    let player_cell = CellLevel::new(Cell::new(6, 6), Level::new(0));
    place_ganger(&mut app, enemy_cell, enemy_faction);
    // GTW-378 — both hovered cells must be squad-VISIBLE for the panel (and its name tint) to
    // populate under the new fog gate; this test inspects a SEEN enemy + a player ganger.
    make_cells_visible(&mut app, &[enemy_cell, player_cell]);
    hover(&mut app, Some(enemy_cell));
    let enemy_color = inspect_name_color(&mut app);
    assert!(
        enemy_color.is_some(),
        "the inspect stat block carries a name color"
    );
    if let Some(color) = enemy_color {
        assert_ne!(
            color.0, normal,
            "an enemy ganger's name is tinted (NOT the normal theme color)",
        );
    }

    // A PLAYER-faction ganger → the normal theme color (no enemy tint). The cell was made
    // squad-VISIBLE above (GTW-378); a player-faction occupant is trivially visible regardless.
    place_ganger(&mut app, player_cell, Faction::new(player));
    hover(&mut app, Some(player_cell));
    if let Some(color) = inspect_name_color(&mut app) {
        assert_eq!(
            color.0, normal,
            "a player-faction ganger's name uses the normal theme color",
        );
    }
}

/// AC2 — hovering a non-floor OBJECT (cover) cell shows the panel + the object block (hardness
/// + integrity), and hides the ganger stat block.
#[test]
fn hovering_an_object_shows_the_object_block() {
    let mut app = hover_app();
    let cell = CellLevel::new(Cell::new(7, 7), Level::new(0));
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_terrain(cell, TerrainKind::Cover);
    // Seed a KNOWN cover entry so the object block's stat lines are discriminating: half
    // integrity, a distinct hardness / protection, a MID height band.
    app.world_mut().resource_mut::<CoverLedger>().insert(
        cell,
        CoverEntry {
            current_hp:       CoverHp::new(4),
            max_hp:           CoverHp::new(8),
            height_band:      HeightBand::Mid,
            armor_protection: ArmorProtection::new(2),
            armor_hardness:   ArmorHardness::new(3),
            destroyed:        Destroyed::new(false),
        },
    );

    hover(&mut app, Some(cell));

    let block = single_global::<InspectObjectBlock>(&mut app);
    assert!(
        block.is_some(),
        "the inspect panel must carry an object block"
    );
    // GTW-295 — the sub-blocks show/hide via Node.display: the object block is Flex (visible,
    // in layout), the ganger block is None (removed from layout, so the panel stays compact).
    assert_eq!(
        display_of::<InspectObjectBlock>(&mut app),
        Some(Display::Flex),
        "hovering cover shows the object block (Display::Flex)",
    );
    assert_eq!(
        display_of::<InspectStatBlockHost>(&mut app),
        Some(Display::None),
        "hovering an object removes the ganger stat block from layout (Display::None)",
    );

    // The object block has an integrity bar with a half fill (4/8 of the seeded entry).
    let bar = single_global::<InspectObjectBar>(&mut app);
    assert!(
        bar.is_some(),
        "the object block must carry an integrity bar"
    );
    if let Some(bar) = bar {
        let integrity = bar_fill_at(&app, bar).unwrap_or(-1.0);
        assert!(
            (integrity - 50.0).abs() < 0.5,
            "the object integrity bar reads 4/8 == 50% (got {integrity})"
        );
    }

    // GTW-295 AC3 — the object block carries the full readable cover stat set, populated from
    // the seeded CoverEntry (title + labeled hardness / protection / height-band lines).
    assert_eq!(
        global_line_text::<InspectObjectText>(&mut app).as_deref(),
        Some("Cover"),
        "the object block titles the kind",
    );
    let hardness = global_line_text::<InspectObjectHardness>(&mut app).unwrap_or_default();
    assert!(
        hardness.contains('3'),
        "the Hardness line reads the seeded armor_hardness (3): {hardness}",
    );
    let protection = global_line_text::<InspectObjectProtection>(&mut app).unwrap_or_default();
    assert!(
        protection.contains('2'),
        "the Protection line reads the seeded armor_protection (2): {protection}",
    );
    let height = global_line_text::<InspectObjectHeight>(&mut app).unwrap_or_default();
    assert!(
        height.contains("Mid"),
        "the Height-band line reads the seeded height_band (Mid): {height}",
    );
}

/// AC2 — hovering BARE FLOOR (no occupant, Open terrain) hides the whole panel.
#[test]
fn hovering_bare_floor_hides_the_panel() {
    let mut app = hover_app();
    let cell = CellLevel::new(Cell::new(20, 20), Level::new(0));
    // Ensure it is open floor with no occupant (the default grid).
    hover(&mut app, Some(cell));

    let root = single_global::<InspectPanelRoot>(&mut app);
    assert!(root.is_some(), "the inspect panel must exist");
    if let Some(root) = root {
        assert_eq!(
            app.world().get::<Visibility>(root),
            Some(&Visibility::Hidden),
            "hovering bare floor hides the whole inspect panel",
        );
    }
    // And nothing hovered hides it too.
    hover(&mut app, None);
    if let Some(root) = single_global::<InspectPanelRoot>(&mut app) {
        assert_eq!(
            app.world().get::<Visibility>(root),
            Some(&Visibility::Hidden),
            "nothing hovered hides the panel",
        );
    }
}
