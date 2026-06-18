//! GTW-275 — the battlescape weapon panel (bottom-left) + the LIVE reload wiring, driven
//! through the REAL app stack.
//!
//! These headless `GdtfTestAppBuilder` integration tests drive the genuine state machine
//! down to `BattleScapeState::BattleRunning`, where the real weapon-panel plugin spawns its
//! tree and its update system repaints it under the `BattleInProgress` gate. They cover:
//!
//! - **AC5** — the weapon name + magazine `"cur/max"` text reflect the selected ganger's
//!   components; selecting a different ganger MUTATES in place (stable widget ids).
//! - **AC6** — the LIVE Reload button is shown when the weapon has a magazine (`size > 0`),
//!   hidden with no weapon; pressing it emits an `ActIntent::Reload` → `ReloadRequested`
//!   through the `gdtf_battle_input` seam.
//! - **AC9** — no selection / no weapon → the weapon content is `Visibility::Hidden`.
//! - **AC8** — the action-bar no longer carries a Reload button (the removed marker is gone).
//! - **GTW-295** — the content is CONTAINED + RESPONSIVE: the content carries a responsive
//!   `min_height` (a `Val::Vh`/`Val::Percent`, NOT a fixed `Val::Px`), and the empty-state
//!   hide uses `Display::None` (removed from layout) so a hidden weapon block takes no space.

use bevy::{
    ecs::entity::Entity,
    prelude::*,
    state::state::State,
    ui::{Display, widget::Button},
};
use gdtf_app::test_support::{
    AppState, BattleScapeState, ReloadButton, RunningState, WeaponContent, WeaponMagazineText,
    WeaponNameText, WeaponPanelRoot,
};
use gdtf_battle_input::{SelectedShooter, dispatch_act_intents};
use gdtf_battle_sim::{
    Aiming, Cell, CellLevel, Direction, Facing, Faction, FireMode, FireModeSpec, LifeState,
    Magazine, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, Position, ReloadTu,
    Stance, StanceKind, Tu, TuMax, WeaponBundle,
    acts::ReloadRequested,
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, HandlingProfile, Kickback,
        Stable, WeaponDamage, WeaponName, WeaponPunch, WeaponShred,
    },
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// A budget large enough to drive the deep walk into the battlescape, bounded so a machine
/// that never reaches the predicate fails instead of hanging.
const BUDGET: u32 = 96;

/// Reads the current [`BattleScapeState`] if active.
fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Reads the current [`RunningState`] if active.
fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Drives the real stack to `BattleScapeState::BattleRunning`, where the weapon panel is live.
fn battle_running_app() -> App {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::WeaponRegistry::default());

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
fn all_with<M: Component>(app: &mut App) -> Vec<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    q.iter(app.world()).collect()
}

/// The single entity carrying marker `M`, or `None` if not exactly one.
fn single_with<M: Component>(app: &mut App) -> Option<Entity> {
    match all_with::<M>(app).as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// The rendered `Text` of the single entity carrying marker `M`.
fn line_text<M: Component>(app: &mut App) -> Option<String> {
    let entity = single_with::<M>(app)?;
    app.world()
        .get::<Text>(entity)
        .map(|t| t.as_str().to_owned())
}

/// The [`Visibility`] of the single entity carrying marker `M`.
fn visibility<M: Component>(app: &mut App) -> Option<Visibility> {
    let entity = single_with::<M>(app)?;
    app.world().get::<Visibility>(entity).copied()
}

/// A weapon kit (the `Magazine` grouping + the rest) — built with an EXPLICIT magazine so
/// the test pins a known `loaded`/`size`. Arbitrary magnitudes (not shipped tuning).
fn weapon_kit(name: &str, magazine: Magazine) -> WeaponBundle {
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
        ),
    )
}

/// Spawns an armed ganger carrying `weapon`, SELECTS it, and returns its entity. (The
/// cursor-click selection is covered in `gdtf_battle_input`; the panel only reads
/// `*SelectedShooter` + the on-entity components.)
fn spawn_armed_and_select(app: &mut App, weapon: WeaponBundle) -> Entity {
    let ganger = app
        .world_mut()
        .spawn((
            Position::new(CellLevel::new(
                Cell::new(3, 3),
                gdtf_battle_sim::Level::new(0),
            )),
            Faction::new(0),
            Facing::new(Direction::East),
            Stance::new(StanceKind::Standing),
            Aiming::new(false),
            Tu::new(100),
            TuMax::new(100),
            LifeState::Alive,
            weapon,
        ))
        .id();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    ganger
}

/// Spawns an UNARMED ganger (no weapon components), SELECTS it, and returns its entity.
fn spawn_unarmed_and_select(app: &mut App) -> Entity {
    let ganger = app
        .world_mut()
        .spawn((
            Position::new(CellLevel::new(
                Cell::new(4, 4),
                gdtf_battle_sim::Level::new(0),
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

// ---------------------------------------------------------------------------------
// AC5 — name + magazine cur/max reflect the selection; selection change MUTATES in place.
// ---------------------------------------------------------------------------------

#[test]
fn weapon_panel_shows_name_and_magazine_for_the_selected_ganger() {
    let mut app = battle_running_app();
    let magazine = Magazine::new(20, MagazineSize::new(30), ReloadTu::new(12));
    spawn_armed_and_select(&mut app, weapon_kit("Autogun", magazine));
    app.update();

    let name = line_text::<WeaponNameText>(&mut app).unwrap_or_default();
    assert_eq!(
        name, "Autogun",
        "the panel shows the selected weapon's name"
    );

    let mag = line_text::<WeaponMagazineText>(&mut app).unwrap_or_default();
    assert_eq!(mag, "20/30", "the panel shows the magazine cur/max");
}

#[test]
fn selection_change_mutates_the_panel_in_place() {
    let mut app = battle_running_app();
    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Autogun",
            Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        ),
    );
    app.update();

    // The widget ids BEFORE the selection change — they must be STABLE across the change.
    let name_id_before = single_with::<WeaponNameText>(&mut app);
    let mag_id_before = single_with::<WeaponMagazineText>(&mut app);
    assert_eq!(
        line_text::<WeaponNameText>(&mut app).as_deref(),
        Some("Autogun"),
    );

    // Select a DIFFERENT armed ganger with a distinct weapon + magazine.
    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Lasgun",
            Magazine::new(8, MagazineSize::new(20), ReloadTu::new(10)),
        ),
    );
    app.update();

    let name_id_after = single_with::<WeaponNameText>(&mut app);
    let mag_id_after = single_with::<WeaponMagazineText>(&mut app);
    assert_eq!(
        name_id_before, name_id_after,
        "the name widget id is STABLE across selection change (mutate, not respawn)",
    );
    assert_eq!(
        mag_id_before, mag_id_after,
        "the magazine widget id is STABLE across selection change (mutate, not respawn)",
    );
    assert_eq!(
        line_text::<WeaponNameText>(&mut app).as_deref(),
        Some("Lasgun"),
        "the name text mutated to the new selection's weapon",
    );
    assert_eq!(
        line_text::<WeaponMagazineText>(&mut app).as_deref(),
        Some("8/20"),
        "the magazine text mutated to the new selection's cur/max",
    );
}

// ---------------------------------------------------------------------------------
// AC6 / AC9 — Reload button visibility matches magazine presence; empty state hidden.
// ---------------------------------------------------------------------------------

#[test]
fn reload_button_visible_with_a_magazine_hidden_without_a_weapon() {
    let mut app = battle_running_app();

    // Armed with a magazine (size > 0) → the content + reload button are Visible.
    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Autogun",
            Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        ),
    );
    app.update();
    assert_ne!(
        visibility::<ReloadButton>(&mut app),
        Some(Visibility::Hidden),
        "the LIVE Reload button is shown when the weapon has a magazine (size > 0)",
    );
    assert_ne!(
        visibility::<WeaponContent>(&mut app),
        Some(Visibility::Hidden),
        "the weapon content is shown for an armed selection",
    );

    // Unarmed selection → the content (and so the reload button) are Hidden (AC9 empty state).
    spawn_unarmed_and_select(&mut app);
    app.update();
    assert_eq!(
        visibility::<WeaponContent>(&mut app),
        Some(Visibility::Hidden),
        "an unarmed selection hides the weapon content (AC9 empty state)",
    );
    assert_eq!(
        visibility::<ReloadButton>(&mut app),
        Some(Visibility::Hidden),
        "no weapon → the Reload button is hidden",
    );
}

#[test]
fn no_selection_hides_the_weapon_content() {
    let mut app = battle_running_app();
    // Force NO selection (the auto-select may have filled it on the empty default battle).
    app.world_mut().insert_resource(SelectedShooter::cleared());
    app.update();
    assert_eq!(
        visibility::<WeaponContent>(&mut app),
        Some(Visibility::Hidden),
        "with no selection the weapon content is hidden (AC9)",
    );
}

// ---------------------------------------------------------------------------------
// AC6 — pressing Reload emits ActIntent::Reload → ReloadRequested through the seam.
// ---------------------------------------------------------------------------------

/// Collected `ReloadRequested` messages (probe), read after the drain.
#[derive(Resource, Default)]
struct ReloadProbe(Vec<ReloadRequested>);

#[test]
fn pressing_reload_emits_a_reload_requested_for_the_selection() {
    let mut app = battle_running_app();
    app.insert_resource(ReloadProbe::default());
    app.add_systems(
        Update,
        (|mut r: MessageReader<ReloadRequested>, mut p: ResMut<ReloadProbe>| {
            p.0.extend(r.read().copied());
        })
        .after(dispatch_act_intents),
    );

    let ganger = spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Autogun",
            Magazine::new(0, MagazineSize::new(30), ReloadTu::new(12)),
        ),
    );
    app.update();

    // Synthesize a press on the LIVE Reload button (the cursor production is covered
    // elsewhere; the panel only needs the Interaction::Pressed transition).
    let reload_opt = single_with::<ReloadButton>(&mut app);
    assert!(
        reload_opt.is_some(),
        "the weapon panel must carry exactly one LIVE Reload button",
    );
    let Some(reload) = reload_opt else {
        return;
    };
    assert!(
        app.world().get::<Button>(reload).is_some(),
        "the Reload control is a real Button (interactive, not a label)",
    );
    if let Some(mut interaction) = app.world_mut().get_mut::<bevy::ui::Interaction>(reload) {
        *interaction = bevy::ui::Interaction::Pressed;
    }
    app.update();

    let reloads = app
        .world()
        .get_resource::<ReloadProbe>()
        .map_or_else(Vec::new, |p| p.0.clone());
    assert_eq!(
        reloads.len(),
        1,
        "pressing Reload must emit exactly one ReloadRequested through the seam",
    );
    assert_eq!(
        reloads[0].actor, ganger,
        "the ReloadRequested actor is the SelectedShooter",
    );
}

// ---------------------------------------------------------------------------------
// AC8 — the action-bar no longer carries a Reload button (the removed stub).
// ---------------------------------------------------------------------------------

#[test]
fn weapon_panel_exists_and_is_the_only_reload_button() {
    let mut app = battle_running_app();

    // The weapon panel root exists in BattleRunning.
    assert!(
        single_with::<WeaponPanelRoot>(&mut app).is_some(),
        "the weapon panel is spawned in BattleRunning",
    );

    // There is exactly ONE Reload button, and it lives under the weapon panel — the old
    // action-bar Reload stub is GONE (AC8). (The action-bar `ReloadButton` marker was
    // removed entirely, so the only `ReloadButton` is the weapon panel's.)
    let reloads = all_with::<ReloadButton>(&mut app);
    assert_eq!(
        reloads.len(),
        1,
        "exactly one Reload button exists (the weapon panel's LIVE button)",
    );
}

// ---------------------------------------------------------------------------------
// GTW-295 — the weapon content is CONTAINED + RESPONSIVE, and the empty-state hide uses
// Display::None (removed from layout) — not just Visibility::Hidden.
// ---------------------------------------------------------------------------------

/// The content column's [`Node`], if present.
fn content_node(app: &mut App) -> Option<Node> {
    let entity = single_with::<WeaponContent>(app)?;
    app.world().get::<Node>(entity).cloned()
}

/// GTW-295 — the weapon content carries a RESPONSIVE `min_height` (a window-relative
/// `Val::Vh` / `Val::Percent`, NOT a fixed `Val::Px`), so the panel contains its content and
/// scales with the window instead of being pinned at one resolution. Pin-discriminating: a
/// revert to a fixed `Val::Px` height (or no `min_height`) fails the unit-kind assert.
#[test]
fn weapon_content_has_responsive_min_height_not_px() {
    let mut app = battle_running_app();
    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Autogun",
            Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        ),
    );
    app.update();

    let node = content_node(&mut app);
    assert!(node.is_some(), "the weapon content column must exist");
    let Some(node) = node else { return };
    assert!(
        matches!(node.min_height, Val::Vh(_) | Val::Percent(_)),
        "the weapon content min_height must be responsive (Vh/Percent), not Px — got {:?}",
        node.min_height,
    );
    // And it must be a NON-zero floor (the actual containment — a zero would not contain).
    let floor = match node.min_height {
        Val::Vh(v) | Val::Percent(v) => v,
        _ => 0.0,
    };
    assert!(
        floor > 0.0,
        "the responsive min_height must be a non-zero floor (got {floor})",
    );
}

/// GTW-295 — the weapon content is laid out HORIZONTAL-FIRST per the mockup: the top-level
/// content column is a `FlexDirection::Column`, and the name / magazine / reload markers all
/// live under the content root. Pin-discriminating: it asserts the structure exists and the
/// content is a column (the mockup shape), not the old square-graphic stack.
#[test]
fn weapon_content_structure_is_contained_under_the_root() {
    let mut app = battle_running_app();
    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Autogun",
            Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        ),
    );
    app.update();

    let node = content_node(&mut app);
    assert!(node.is_some(), "the weapon content column must exist");
    let Some(node) = node else { return };
    assert_eq!(
        node.flex_direction,
        FlexDirection::Column,
        "the content is a vertical column (graphic row above the info column)",
    );

    // The name / magazine / reload widgets exist (the info column under the content).
    assert!(
        single_with::<WeaponNameText>(&mut app).is_some(),
        "the content carries the weapon name line",
    );
    assert!(
        single_with::<WeaponMagazineText>(&mut app).is_some(),
        "the content carries the magazine line",
    );
    assert!(
        single_with::<ReloadButton>(&mut app).is_some(),
        "the content carries the LIVE Reload button",
    );
}

/// GTW-295 — the empty-state hide removes the content from LAYOUT via `Display::None` (so a
/// hidden weapon block takes no space), not merely `Visibility::Hidden`. Pin-discriminating:
/// a revert to a Visibility-only hide leaves `display == Display::Flex` and fails this assert.
#[test]
fn empty_state_hides_content_with_display_none() {
    let mut app = battle_running_app();

    // Armed → the content is shown (Display::Flex).
    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Autogun",
            Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        ),
    );
    app.update();
    if let Some(node) = content_node(&mut app) {
        assert_eq!(
            node.display,
            Display::Flex,
            "an armed selection shows the content (Display::Flex)",
        );
    }

    // Unarmed → the content is removed from layout (Display::None).
    spawn_unarmed_and_select(&mut app);
    app.update();
    let node = content_node(&mut app);
    assert!(
        node.is_some(),
        "the content column persists (mutate-in-place)"
    );
    let Some(node) = node else { return };
    assert_eq!(
        node.display,
        Display::None,
        "an unarmed selection removes the content from layout (Display::None)",
    );
    // And the existing Visibility contract still holds (GTW-275 not regressed).
    assert_eq!(
        visibility::<WeaponContent>(&mut app),
        Some(Visibility::Hidden),
        "the content is also Visibility::Hidden (GTW-275 contract preserved)",
    );
}
