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
//! - **GTW-298 rework** — the cluster matches the AUTHORITATIVE structure: the Overall Weapon
//!   Panel 2×2 grid (Combined / Firemode / Item / Aim) + the separate Stance Panel, the
//!   relocated firemode / aim / stance controls present in their new panels, and all sizing
//!   RESPONSIVE (`Val::Vw`/`Val::Vh`/`Val::Percent`, NOT a fixed `Val::Px`); the empty-state
//!   hide uses `Display::None` (removed from layout) so a hidden weapon block takes no space.

use bevy::{
    ecs::entity::Entity,
    prelude::*,
    state::state::State,
    ui::{Display, widget::Button},
};
use gdtf_app::test_support::{
    AimLabel, AimPanel, AimToggleButton, AppState, BattleScapeState, BottomBarRoot,
    CombinedWeaponPanel, ModePanelRoot, ReloadButton, RunningState, StancePanelRoot,
    StanceProneButton, StanceStandingButton, WeaponContent, WeaponImage, WeaponItemButton,
    WeaponItemPanel, WeaponMagazineText, WeaponNameText, WeaponPanelRoot,
};
use gdtf_battle_input::{SelectedShooter, dispatch_act_intents};
use gdtf_battle_sim::{
    Aiming, Cell, CellLevel, Direction, Facing, Faction, FireMode, FireModeSpec, LifeState,
    Magazine, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, Position, ReloadTu,
    Stance, StanceKind, Tu, TuMax, WeaponBundle,
    acts::ReloadRequested,
    injuries::InjuryRegistry,
    level::ThemeCatalogRegistry,
    terrain::piece::TerrainRegistry,
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, Handedness, HandlingProfile,
        Kickback, Stable, WeaponDamage, WeaponName, WeaponPunch, WeaponShred,
    },
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::{
    theme::default_theme,
    themed::{ThemeRole, Themed},
};

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
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::WeaponRegistry::default());
    // GTW-269: setup_battle_on_request now reads an ArmorRegistry to armor each ganger,
    // failing closed (no BattleReady) without one. This panel harness builds a
    // ganger-free default battle, so an empty registry suffices — it just must be
    // present for the setup to run and reach BattleRunning.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    // GTW-394: the Load gate also requires a TerrainRegistry; empty clears it.
    app.world_mut().insert_resource(TerrainRegistry::default());
    app.world_mut()
        .insert_resource(ThemeCatalogRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    // GTW-415: the Load→Intro gate also requires a GangRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    // GTW-418: the Load gate also requires a PrefabRegistry; empty clears it.
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
        ))
        .id();
    app.world_mut()
        .spawn((gdtf_battle_sim::WieldedBy::new(ganger), weapon));
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
// GTW-298 rework — the cluster matches the AUTHORITATIVE structure (Overall 2×2 grid +
// separate Stance Panel), the relocated controls are present in their new panels, the content
// is CONTAINED + RESPONSIVE, and the empty-state hide uses Display::None (removed from layout) —
// not just Visibility::Hidden.
// ---------------------------------------------------------------------------------

/// The content column's [`Node`], if present.
fn content_node(app: &mut App) -> Option<Node> {
    let entity = single_with::<WeaponContent>(app)?;
    app.world().get::<Node>(entity).cloned()
}

/// The [`Node`] of the single entity carrying marker `M`, if present.
fn node_of<M: Component>(app: &mut App) -> Option<Node> {
    let entity = single_with::<M>(app)?;
    app.world().get::<Node>(entity).cloned()
}

/// `true` when `descendant` has `ancestor` somewhere on its parent chain (walking
/// [`ChildOf`](bevy::prelude::ChildOf) upward). Used to assert the Stance Panel is laid out
/// INSIDE the bottom bar (D4) rather than being a free-floating sibling overlay.
fn is_descendant_of(app: &App, descendant: Entity, ancestor: Entity) -> bool {
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

/// GTW-298 rework — the cluster matches the AUTHORITATIVE structure: the Overall Weapon Panel
/// root holds the four 2×2 grid cells (Combined / Firemode / Item / Aim), the Combined panel
/// carries the image + the name/mag text column + the LIVE Reload button, the Item panel carries
/// TWO disabled item buttons, the Firemode panel (the relocated mode panel) and the Aim panel
/// (the relocated aim toggle) are present, and the SEPARATE Stance Panel carries the three
/// relocated stance toggles. Pin-discriminating: it asserts the structural + relocated markers —
/// a revert to the OLD layout (TOP/BOTTOM bands + empty controls frames, controls still in the
/// action bar) fails the marker asserts.
#[test]
fn weapon_panel_matches_authoritative_structure() {
    let mut app = battle_running_app();
    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Autogun",
            Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        ),
    );
    app.update();
    // Settle the firemode rebuild (.after ApplyTheme) so the relocated mode toggles resolve.
    app.update();

    // ONE Overall Weapon Panel root + the four grid cells.
    assert!(
        single_with::<WeaponPanelRoot>(&mut app).is_some(),
        "exactly one Overall Weapon Panel root",
    );
    assert!(
        single_with::<CombinedWeaponPanel>(&mut app).is_some(),
        "the Combined Weapon Panel (top-left) exists",
    );
    assert!(
        single_with::<WeaponImage>(&mut app).is_some(),
        "the Combined panel carries the full-width Weapon Image placeholder",
    );
    assert!(
        single_with::<WeaponItemPanel>(&mut app).is_some(),
        "the Item Panel (top-right) exists",
    );
    assert!(
        single_with::<AimPanel>(&mut app).is_some(),
        "the Aim Panel (bottom-right) exists",
    );

    // The weapon-text column inside the Combined panel is a vertical column (name over mag).
    let content = content_node(&mut app);
    assert!(content.is_some(), "the weapon-text column must exist");
    let Some(content) = content else { return };
    assert_eq!(
        content.flex_direction,
        FlexDirection::Column,
        "the weapon-text column is a vertical column (name over magazine)",
    );

    // The Combined panel carries the name + magazine lines + the LIVE Reload button.
    assert!(
        single_with::<WeaponNameText>(&mut app).is_some(),
        "the Combined panel carries the weapon name line",
    );
    assert!(
        single_with::<WeaponMagazineText>(&mut app).is_some(),
        "the Combined panel carries the magazine line",
    );
    assert!(
        single_with::<ReloadButton>(&mut app).is_some(),
        "the Combined panel carries the LIVE Reload button",
    );

    // The Item Panel carries exactly TWO disabled item buttons.
    assert_eq!(
        all_with::<WeaponItemButton>(&mut app).len(),
        2,
        "the Item Panel carries two stacked disabled item buttons",
    );

    // The relocated Firemode panel (the mode-panel root) is present in the cluster.
    assert!(
        single_with::<ModePanelRoot>(&mut app).is_some(),
        "the relocated Firemode panel (ModePanelRoot) is present",
    );
    // The relocated Aim toggle is present in the Aim panel.
    assert!(
        single_with::<AimToggleButton>(&mut app).is_some(),
        "the relocated Aim toggle is present",
    );

    // The SEPARATE Stance Panel carries the three relocated stance toggles.
    assert!(
        single_with::<StancePanelRoot>(&mut app).is_some(),
        "the separate Stance Panel is present",
    );
    assert!(
        single_with::<StanceStandingButton>(&mut app).is_some()
            && single_with::<StanceProneButton>(&mut app).is_some(),
        "the Stance Panel carries the relocated stance toggles",
    );
}

/// The Aim control reads "Aim [switch]" — the GTW-277 widget migration dropped the caption,
/// leaving the bare `Switch`; this restores an [`AimLabel`] "Aim" Text caption laid out in the
/// SAME cell as the switch (a ROW, label on the LEFT). Pin-discriminating: with the bare switch
/// (no label restored) the `AimLabel` marker is absent and the cell is not a Row, so both the
/// label assert and the row-layout assert fail.
#[test]
fn aim_panel_has_aim_caption_beside_the_switch() {
    let mut app = battle_running_app();
    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Autogun",
            Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        ),
    );
    app.update();
    // Settle the post-ApplyTheme fit pass (the label font is held at the control size there).
    app.update();

    // The "Aim" caption exists and reads "Aim".
    assert!(
        single_with::<AimLabel>(&mut app).is_some(),
        "the Aim Panel carries the restored \"Aim\" caption (GTW-277 had dropped it)",
    );
    assert_eq!(
        line_text::<AimLabel>(&mut app).as_deref(),
        Some("Aim"),
        "the Aim caption reads \"Aim\"",
    );

    // The caption + the switch live in the SAME cell — both descend from the one Aim Panel.
    let cell_entity = single_with::<AimPanel>(&mut app);
    let label = single_with::<AimLabel>(&mut app);
    let switch = single_with::<AimToggleButton>(&mut app);
    assert!(
        cell_entity.is_some() && label.is_some() && switch.is_some(),
        "the Aim Panel cell, the caption, and the switch all exist",
    );
    let (Some(aim_panel), Some(label), Some(switch)) = (cell_entity, label, switch) else {
        return;
    };
    assert!(
        is_descendant_of(&app, label, aim_panel),
        "the \"Aim\" caption is laid out inside the Aim Panel cell",
    );
    assert!(
        is_descendant_of(&app, switch, aim_panel),
        "the Aim switch is laid out inside the Aim Panel cell (same cell as the caption)",
    );

    // The cell lays the caption + switch out as a ROW ("Aim [switch]", the mockup reading).
    let cell = node_of::<AimPanel>(&mut app);
    assert!(cell.is_some(), "the Aim Panel cell has a Node");
    let Some(cell) = cell else { return };
    assert_eq!(
        cell.flex_direction,
        FlexDirection::Row,
        "the Aim Panel cell is a ROW so the caption sits to the LEFT of the switch",
    );
}

/// GTW-298 — the cluster cells size RESPONSIVELY: the Overall panel root carries a
/// window-relative `Val::Vw`/`Val::Vh` size, and the grid cells (Combined, Item) split it by
/// `Val::Percent` — NOT fixed `Val::Px`. The separate Stance Panel is a `Val::Vw`-positioned
/// fixed-fraction column. Pin-discriminating: a revert to a px-pinned layout fails the unit-kind
/// asserts.
#[test]
fn weapon_panel_bands_size_responsively_not_px() {
    let mut app = battle_running_app();
    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Autogun",
            Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        ),
    );
    app.update();

    // The root is sized in window-relative units (Vw width + Vh height), not fixed px.
    let root = node_of::<WeaponPanelRoot>(&mut app);
    assert!(root.is_some(), "the Overall Weapon Panel root must exist");
    let Some(root) = root else { return };
    assert!(
        matches!(root.width, Val::Vw(_) | Val::Vh(_) | Val::Percent(_)),
        "the panel root width is responsive (Vw/Vh/Percent), not Px — got {:?}",
        root.width,
    );
    assert!(
        matches!(root.height, Val::Vh(_) | Val::Vw(_) | Val::Percent(_)),
        "the panel root height is responsive (Vh/Vw/Percent), not Px — got {:?}",
        root.height,
    );

    // The Combined + Item grid cells split their columns by Percent height (the 3/4 split), not
    // fixed px.
    let combined = node_of::<CombinedWeaponPanel>(&mut app);
    let items = node_of::<WeaponItemPanel>(&mut app);
    let Some(combined) = combined else { return };
    let Some(items) = items else { return };
    assert!(
        matches!(combined.height, Val::Percent(_)),
        "the Combined cell height is a Percent of its column, not Px — got {:?}",
        combined.height,
    );
    assert!(
        matches!(items.height, Val::Percent(_)),
        "the Item cell height is a Percent of its column, not Px — got {:?}",
        items.height,
    );

    // The separate Stance Panel is a fixed-fraction-of-window (Vw) width column.
    let stance = node_of::<StancePanelRoot>(&mut app);
    let Some(stance) = stance else { return };
    assert!(
        matches!(stance.width, Val::Vw(_)),
        "the separate Stance Panel width is a fixed window fraction (Vw), not Px — got {:?}",
        stance.width,
    );
}

/// D4 (2026-06-18 screenshot review) — the Stance Panel is laid out INSIDE the bottom bar: its
/// [`StancePanelRoot`] is a DESCENDANT of the [`BottomBarRoot`] container, not a free-floating
/// sibling overlay sitting on top of the bottom panel's edge. Pin-discriminating: a revert to
/// parenting the stance under the weapon root (or anywhere outside the bar) fails the ancestry
/// assert.
#[test]
fn stance_panel_is_child_of_the_bottom_bar() {
    let mut app = battle_running_app();
    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Autogun",
            Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        ),
    );
    app.update();

    let stance = single_with::<StancePanelRoot>(&mut app);
    let bar = single_with::<BottomBarRoot>(&mut app);
    assert!(
        stance.is_some() && bar.is_some(),
        "both the Stance Panel and the bottom bar must exist in BattleRunning",
    );
    let (Some(stance), Some(bar)) = (stance, bar) else {
        return;
    };
    assert!(
        is_descendant_of(&app, stance, bar),
        "the Stance Panel must be laid out INSIDE the bottom bar (a descendant of BottomBarRoot), \
         not a free-floating overlay",
    );
}

/// D-B (2026-06-18 screenshot review) — the Stance Panel is its OWN bordered sub-panel: its
/// [`StancePanelRoot`] carries `Themed(ThemeRole::Panel)` (the same themed-box role the weapon
/// cluster's panels use), NOT a plain transparent `Node`. Pin-discriminating: the prior D4
/// plain-`Node` Stance column carried NO `Themed`, so this assert fails on a revert.
#[test]
fn stance_panel_is_a_themed_panel_box() {
    let mut app = battle_running_app();
    spawn_armed_and_select(
        &mut app,
        weapon_kit(
            "Autogun",
            Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        ),
    );
    app.update();

    let Some(stance) = single_with::<StancePanelRoot>(&mut app) else {
        return;
    };
    assert_eq!(
        app.world().get::<Themed>(stance).map(|t| **t),
        Some(ThemeRole::Panel),
        "the Stance Panel must be its OWN bordered Themed(Panel) box (D-B), like the weapon cluster",
    );
}

/// D-B — the Stance Panel is sized to the SAME HEIGHT as the Overall Weapon Panel (NOT the full
/// bottom-bar height) and a fixed relative width: both height fields are the SAME responsive
/// `Val::Vh` value (so the two bordered panels are the same height), and the width is a fixed
/// window fraction (`Val::Vw`), never a fixed `Val::Px`. Pin-discriminating: sizing the stance to
/// the full bar height (a different `Vh`) or to a px width fails the equal-height / unit-kind
/// asserts.
#[test]
fn stance_panel_matches_overall_weapon_panel_height_relative_units() {
    let mut app = battle_running_app();
    app.update();

    let stance = node_of::<StancePanelRoot>(&mut app);
    let overall = node_of::<WeaponPanelRoot>(&mut app);
    let (Some(stance), Some(overall)) = (stance, overall) else {
        return;
    };

    // Width is a fixed window fraction (Vw), never Px.
    assert!(
        matches!(stance.width, Val::Vw(_)),
        "the Stance Panel width is a fixed window fraction (Vw), not Px — got {:?}",
        stance.width,
    );
    // Height is responsive (Vh) and EXACTLY the Overall Weapon Panel's height — the same height,
    // not the full bottom-bar height.
    assert!(
        matches!(stance.height, Val::Vh(_)),
        "the Stance Panel height is responsive (Vh), not Px — got {:?}",
        stance.height,
    );
    assert_eq!(
        stance.height, overall.height,
        "the Stance Panel height EQUALS the Overall Weapon Panel height (same height, not the full \
         bottom-bar height) — D-B",
    );
    // B FAIL (2026-06-18 screenshot review) — the Stance Panel is anchored the SAME relative
    // `bottom` as the Overall Weapon Panel: a non-zero `Val::Vh` inset above the window bottom (NOT
    // flush at `bottom: 0`, which ran its framed box to the window's bottom edge so it read as
    // loose buttons on the bar fill). A revert to `bottom: Px(0.0)` (or a mismatch with the Overall
    // panel) fails this.
    assert_eq!(
        stance.bottom, overall.bottom,
        "the Stance Panel is anchored the SAME relative bottom as the Overall Weapon Panel (so the \
         two bordered boxes are flush-bottomed and both inset off the window edge) — got {:?} vs {:?}",
        stance.bottom, overall.bottom,
    );
    let stance_bottom = match stance.bottom {
        Val::Vh(v) => v,
        _ => 0.0,
    };
    assert!(
        matches!(stance.bottom, Val::Vh(_)) && stance_bottom > 0.0,
        "the Stance Panel is anchored a bottom-padding ABOVE the window bottom — a non-zero \
         relative Vh inset, not flush at bottom: 0 — got {:?}",
        stance.bottom,
    );
}

// ---------------------------------------------------------------------------------
// GTW-275 layout overhaul (items 1 / 4 / 6 / 7) — the BOTTOM BAR is the one opaque strip that
// reduces the map, full-width + responsive height; the weapon panel sits IN it.
// ---------------------------------------------------------------------------------

/// GTW-275 layout overhaul (items 1 / 4) — exactly ONE bottom bar exists in `BattleRunning`,
/// sized FULL window WIDTH (`Val::Vw(100)`) with a RESPONSIVE height (a window-relative
/// `Val::Vh`/`Val::Percent`, NOT a fixed `Val::Px`). It is the only UI that reduces the map.
/// Pin-discriminating: a missing bar, a non-full width, or a px-pinned height fails the asserts.
#[test]
fn bottom_bar_exists_full_width_responsive_height() {
    let mut app = battle_running_app();
    app.update();

    assert!(
        single_with::<BottomBarRoot>(&mut app).is_some(),
        "exactly one bottom bar is spawned in BattleRunning",
    );
    let bar = node_of::<BottomBarRoot>(&mut app);
    assert!(bar.is_some(), "the bottom bar must have a Node");
    let Some(bar) = bar else { return };
    assert_eq!(
        bar.width,
        Val::Vw(100.0),
        "the bottom bar is full window width (Vw 100) — items 1 / 4",
    );
    assert!(
        matches!(bar.height, Val::Vh(_) | Val::Percent(_)),
        "the bottom bar height is responsive (Vh/Percent), not Px — got {:?}",
        bar.height,
    );
    // It is anchored to the window bottom (absolute, bottom: 0) so the map ends at its top edge.
    assert_eq!(
        bar.position_type,
        PositionType::Absolute,
        "the bottom bar is an absolute strip",
    );
    assert_eq!(
        bar.bottom,
        Val::Px(0.0),
        "the bottom bar is anchored to the window bottom",
    );
}

/// Screenshot review 2026-06-18 — the bottom panel insets its content (weapon cluster + stance
/// column) on ALL FOUR sides via `Node::padding`, in RELATIVE units (`Val::Vw` left/right,
/// `Val::Vh` top/bottom — NOT a fixed `Val::Px`), so the bottom row (Prone / firemode buttons /
/// Aim) is not flush against the window bottom and the cluster has breathing room on every edge,
/// matching the mockup. It survives the theme pass (`repad_bottom_bar` re-applies it after
/// `apply_theme`'s `box_node` clobbers `Node::padding`). Pin-discriminating: a zero / missing
/// padding, OR any edge reverted to a fixed `Val::Px`, fails the unit-kind + non-zero asserts.
#[test]
fn bottom_bar_content_has_four_sided_relative_padding() {
    let mut app = battle_running_app();
    app.update();

    let bar = node_of::<BottomBarRoot>(&mut app);
    assert!(bar.is_some(), "the bottom bar must have a Node");
    let Some(bar) = bar else { return };
    let pad = bar.padding;

    // Every edge is a non-zero, window-relative inset (Vw on the horizontal axis, Vh on the
    // vertical) — never a fixed Px (px does not survive a resize) and never zero (zero = flush).
    // A non-relative kind maps to 0.0, so the non-zero assert below catches both a Px revert and
    // a zero inset on any edge.
    for (edge, val) in [
        ("left", pad.left),
        ("right", pad.right),
        ("top", pad.top),
        ("bottom", pad.bottom),
    ] {
        let frac = match val {
            Val::Vw(v) | Val::Vh(v) | Val::Percent(v) => v,
            _ => 0.0,
        };
        assert!(
            frac > 0.0,
            "bottom-bar {edge} padding must be a non-zero RELATIVE inset (Vw/Vh/Percent), got {val:?} — so the content is not flush against the {edge} edge",
        );
    }
    // The horizontal edges track WIDTH (Vw) and the vertical edges track HEIGHT (Vh), so the
    // gutter is uniform-feeling on resize — guards against a swap to the wrong axis.
    assert!(
        matches!(pad.left, Val::Vw(_)) && matches!(pad.right, Val::Vw(_)),
        "the left/right padding tracks window WIDTH (Vw) — got {:?} / {:?}",
        pad.left,
        pad.right,
    );
    assert!(
        matches!(pad.top, Val::Vh(_)) && matches!(pad.bottom, Val::Vh(_)),
        "the top/bottom padding tracks window HEIGHT (Vh) — got {:?} / {:?}",
        pad.top,
        pad.bottom,
    );
}

/// GTW-275 layout overhaul (item 6) / D5 / A FAIL (2026-06-18 screenshot review) — the weapon
/// panel sits IN the bottom bar: it is anchored a bottom-padding's worth ABOVE the window bottom
/// (`bottom: Vh(_)`, a NON-ZERO RELATIVE inset — NOT flush at `bottom: 0`, which jammed the
/// Firemode / Aim bottom row against the window edge) with a FIXED `Val::Vw` width (item 7 — the
/// map area does not shift when the contents change) and a responsive `Val::Vh` height that is
/// INSET inside the bar's CONTENT box — strictly LESS than the full bar height, so it neither
/// bleeds out the top (item 6) nor jams the bottom row against the window edge (the D5 inset off
/// both vertical edges). The `bottom` offset + both heights are `Val::Vh` (relative units), never
/// `Val::Px`. Pin-discriminating: a non-Vw width, a non-Vh height, a `bottom: 0` flush anchor, or a
/// panel as TALL as (or taller than) the full bar (bleed / no inset) fails.
#[test]
fn weapon_panel_sits_inside_the_bottom_bar() {
    let mut app = battle_running_app();
    app.update();

    let bar = node_of::<BottomBarRoot>(&mut app);
    let panel = node_of::<WeaponPanelRoot>(&mut app);
    assert!(bar.is_some(), "the bottom bar must exist");
    assert!(panel.is_some(), "the weapon panel must exist");
    let Some(bar) = bar else { return };
    let Some(panel) = panel else { return };

    // The panel is a fixed-fraction-of-window width (item 7).
    assert!(
        matches!(panel.width, Val::Vw(_)),
        "the weapon panel width is a fixed window fraction (Vw), not Px/auto — got {:?}",
        panel.width,
    );
    // The panel is anchored a bottom-padding ABOVE the window bottom (A FAIL fix): a NON-ZERO
    // RELATIVE `Vh` inset, NOT flush at `bottom: 0` (which jammed the Firemode / Aim row against
    // the window's bottom edge). A revert to `bottom: Px(0.0)` (or any non-Vh / zero offset) fails.
    let bottom_inset = match panel.bottom {
        Val::Vh(v) => v,
        _ => 0.0,
    };
    assert!(
        matches!(panel.bottom, Val::Vh(_)) && bottom_inset > 0.0,
        "the weapon panel is anchored a bottom-padding ABOVE the window bottom — a non-zero \
         relative Vh inset, not flush at bottom: 0 — got {:?}",
        panel.bottom,
    );
    // Both heights are responsive (Vh), never Px (a non-Vh kind maps to 0.0 below, which the
    // non-zero + strict-less asserts then catch).
    assert!(
        matches!(panel.height, Val::Vh(_)),
        "the weapon panel height is responsive (Vh), not Px — got {:?}",
        panel.height,
    );
    assert!(
        matches!(bar.height, Val::Vh(_)),
        "the bottom-bar height is responsive (Vh), not Px — got {:?}",
        bar.height,
    );
    let panel_h = match panel.height {
        Val::Vh(v) => v,
        _ => 0.0,
    };
    let bar_h = match bar.height {
        Val::Vh(v) => v,
        _ => 0.0,
    };
    // The panel height is INSET inside the bar's content box — strictly less than the full bar
    // height (D5: it insets off both vertical edges), so it neither bleeds out the top (item 6) nor
    // jams the bottom row against the window bottom.
    assert!(
        panel_h < bar_h,
        "the weapon panel height ({panel_h}vh) must be INSET inside the bar's content box — strictly \
         less than the full bar height ({bar_h}vh) — so it neither bleeds out the top nor jams the \
         bottom (D5 inset)",
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
