//! GTW-425: headless behavioral tests for the in-app gang-editor's COLLAPSED member rows + inline
//! editing.
//!
//! These run on the `MinimalPlugins` [`GdtfTestAppBuilder`] (the real state stack, `UiPlugin`, and
//! the real `EditorScenePlugin` wired through `ScenesPlugin`), seeded with a theme + a
//! [`WeaponRegistry`] / [`ArmorRegistry`] so the editor screen spawns with populated weapon /
//! armor dropdowns. They assert on the WORLD and the model resource (the system-effects) — never
//! on rendering or real device input.
//!
//! Per CRITICAL GOTCHA 2: editing is driven through the REAL input MESSAGES the widgets raise —
//! [`TextFieldCommitted`] / [`DropdownSelectionChanged`] — or the real action system via a set
//! [`Interaction::Pressed`] + a single `Update` run (NOT a full `app.update()`, which the windowed
//! `ui_focus_system` would clobber — but this harness is `MinimalPlugins`, so there is no
//! `ui_focus_system`; setting `Pressed` then `update()` is the accepted headless idiom, the
//! `gang_editor_scaffold.rs` precedent). Each assertion is pin-discriminating: it would FAIL if its
//! clause were reverted.
//!
//! Coverage (C6):
//!
//! - [`add_member_appears_inside_scroll_area`] — add a member → a row appears, parented INSIDE the
//!   member-list `ScrollListArea` (C1 + GOTCHA 1).
//! - [`member_row_has_exactly_one_control_per_field`] — a row carries EXACTLY ONE control per
//!   field (name / weapon / armor): the redundant doubled echo labels are gone (GTW-499 C1).
//! - [`commit_member_name_updates_model`] — a real `TextFieldCommitted` on a member's name
//!   field edits the model name (C3).
//! - [`commit_member_weapon_updates_model`] / [`commit_member_armor_updates_model`] — a real
//!   `DropdownSelectionChanged` edits the model loadout (C2).
//! - [`delete_member_removes_from_model_and_list`] — a delete press removes the member from the
//!   model AND despawns its row (C4).
//! - [`editing_one_member_leaves_other_rows_untouched`] — after editing ONE member, the OTHER
//!   rows' entity ids are unchanged (proves the edit MUTATES, never rebuilds the list — C5).

use bevy::{
    app::App,
    ecs::{
        component::Component,
        entity::Entity,
        hierarchy::{ChildOf, Children},
    },
    prelude::With,
    state::state::NextState,
    ui::Interaction,
};
use gdtf_app::test_support::{
    AddMemberButton, AppState, DeleteMemberButton, EditableGang, MemberArmorDropdown,
    MemberNameField, MemberRow, MemberRowIndex, MemberWeaponDropdown, RunningState,
};
use gdtf_battle_sim::{
    Accuracy, ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorName, ArmorPiece, ArmorProtection,
    ArmorRegistry, ArmorSpec, ArmorType, BaseSpread, DamageType, FatalBias, FireMode, FireModeSpec,
    Kickback, Magazine, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, ReloadTu,
    Shove, Stable, WeaponDamage, WeaponName, WeaponPunch, WeaponRegistry, WeaponShred, WeaponSpec,
    weapon::Handedness,
};
use gdtf_test_utils::GdtfTestAppBuilder;
use gdtf_ui::{
    CommittedTextValue, DropdownSelectionChanged, ScrollListArea, TextFieldCommitted,
    theme::default_theme,
};

/// Two weapon keys (sorted: "Autogun" < "Lasgun"), so a dropdown selection has a DISTINCT target
/// to commit (a single-option dropdown could never prove a change). Their specs are arbitrary
/// (NOT shipped magnitudes — the brittle-test rule); only the KEYS matter to the dropdown.
const WEAPON_A: &str = "Autogun";
/// The second weapon key (see [`WEAPON_A`]).
const WEAPON_B: &str = "Lasgun";
/// Two armor keys (sorted: "Flak" < "Mesh"), mirroring the weapon keys.
const ARMOR_A: &str = "Flak";
/// The second armor key (see [`ARMOR_A`]).
const ARMOR_B: &str = "Mesh";

/// An arbitrary weapon spec (NOT shipped tuning) — the dropdown only reads the KEY, so the numbers
/// are immaterial. Mirrors `action_bar.rs`'s `armed_registry` shape.
fn arbitrary_weapon() -> WeaponSpec {
    WeaponSpec {
        base_spread:      BaseSpread::new(0.25),
        accuracy:         Accuracy::new(1.0),
        kickback:         Kickback::new(0.4),
        fatal_bias:       FatalBias::new(0.0),
        damage:           WeaponDamage::new(12),
        punch:            WeaponPunch::new(5),
        shred:            WeaponShred::new(3),
        damage_type:      DamageType::Kinetic,
        magazine:         Magazine::loaded(MagazineSize::new(30), ReloadTu::new(12)),
        fire_mode:        FireMode::new(vec![FireModeSpec::new(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.5),
            ModeShots::new(1),
        )]),
        stable:           Stable::new(false),
        shove:            Shove::new(false),
        handedness:       Handedness::OneHanded,
        attachment_slots: Vec::new(),
        dot:              None,
        on_death:         None,
    }
}

/// A [`WeaponRegistry`] of the two test weapon keys.
fn weapon_registry() -> WeaponRegistry {
    WeaponRegistry::new([
        (WeaponName::new(WEAPON_A.to_owned()), arbitrary_weapon()),
        (WeaponName::new(WEAPON_B.to_owned()), arbitrary_weapon()),
    ])
}

/// An arbitrary armor spec (NOT shipped tuning) — the dropdown only reads the KEY.
const fn arbitrary_armor() -> ArmorSpec {
    ArmorSpec::uniform(ArmorPiece::new(
        ArmorFloor::new(0),
        ArmorProtection::new(1),
        ArmorIntegrity::new(2),
        ArmorHardness::new(3),
        ArmorType::DEFAULT,
    ))
}

/// An [`ArmorRegistry`] of the two test armor keys.
fn armor_registry() -> ArmorRegistry {
    ArmorRegistry::new([
        (ArmorName::new(ARMOR_A.to_owned()), arbitrary_armor()),
        (ArmorName::new(ARMOR_B.to_owned()), arbitrary_armor()),
    ])
}

/// Builds a headless app driven into [`RunningState::DebugEditor`] with the editor screen spawned
/// and the weapon / armor registries seeded (so the per-member dropdowns have options). Starts in
/// `AppState::Running` (whose default sub-state is `Menu`), seeds the resources before the first
/// update so the `OnEnter` spawn sees them, then sets the `DebugEditor` transition and pumps a few
/// updates so the screen + any deferred parenting flush.
fn editor_app() -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(weapon_registry());
    app.world_mut().insert_resource(armor_registry());
    app.update();
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::DebugEditor);
    app.update();
    app.update();
    app
}

/// Counts the entities carrying marker `M` in the world.
fn count_with<M: Component>(app: &mut App) -> usize {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    q.iter(app.world()).count()
}

/// Every entity carrying marker `M`, in arbitrary order.
fn all_with<M: Component>(app: &mut App) -> Vec<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    q.iter(app.world()).collect()
}

/// Presses "Add member" through the real add-member system (set `Interaction::Pressed` + a single
/// `update()`, the headless idiom — `MinimalPlugins` has no `ui_focus_system` to clobber it).
fn press_add_member(app: &mut App) {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<AddMemberButton>>();
    let button = q.iter(app.world()).next().unwrap_or(Entity::PLACEHOLDER);
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(button) {
        *interaction = Interaction::Pressed;
    }
    app.update();
    // Flush the deferred parent-into-area command + let the new row settle.
    app.update();
}

/// The first entity carrying BOTH marker `M` and a [`MemberRowIndex`] equal to `index`.
fn control_for_row<M: Component>(app: &mut App, index: usize) -> Option<Entity> {
    let mut q = app
        .world_mut()
        .query_filtered::<(Entity, &MemberRowIndex), With<M>>();
    q.iter(app.world())
        .find(|(_, row_index)| ***row_index == index)
        .map(|(entity, _)| entity)
}

/// How many entities carry BOTH marker `M` and a [`MemberRowIndex`] equal to `index` — the count
/// of THAT field's controls in row `index` (GTW-499 C1: must be exactly one per field).
fn count_controls_for_row<M: Component>(app: &mut App, index: usize) -> usize {
    let mut q = app.world_mut().query_filtered::<&MemberRowIndex, With<M>>();
    q.iter(app.world())
        .filter(|row_index| ***row_index == index)
        .count()
}

/// The number of DIRECT children of row `index`'s collapsed HEADER node — the `+` pip, the portrait
/// placeholder, the inline name field, the weapon dropdown, the armor dropdown, and the delete
/// button: SIX controls after the GTW-499 C1 fix.
///
/// The header carries no marker of its own, so it is located as the common parent of the row's
/// [`MemberNameField`] (a direct child the row builder `add_children`s onto the header). This is the
/// structural pin for C1: the OLD doubled layout `add_children`d THREE extra static echo `Text`
/// nodes (one beside each editable control), so its header held NINE children — re-introducing any
/// echo pushes this count above six and fails the assert.
fn header_child_count(app: &mut App, index: usize) -> usize {
    let Some(field) = control_for_row::<MemberNameField>(app, index) else {
        return 0;
    };
    let Some(header) = app.world().get::<ChildOf>(field).map(ChildOf::parent) else {
        return 0;
    };
    app.world()
        .get::<Children>(header)
        .map(|children| children.len())
        .unwrap_or_default()
}

/// Whether `descendant` has `ancestor` somewhere up its `ChildOf` chain.
fn is_ancestor(app: &App, ancestor: Entity, mut descendant: Entity) -> bool {
    while let Some(child_of) = app.world().get::<ChildOf>(descendant) {
        let parent = child_of.parent();
        if parent == ancestor {
            return true;
        }
        descendant = parent;
    }
    false
}

/// C1 + GOTCHA 1: adding a member appends a row, parented INSIDE the member-list `ScrollListArea`.
///
/// Pin: a no-op add (no new row) fails the count assert; a row parented onto the scroll-list ROOT
/// FRAME instead of the AREA (the bottom-cramp bug) fails the ancestor assert.
#[test]
fn add_member_appears_inside_scroll_area() {
    let mut app = editor_app();
    let before = count_with::<MemberRow>(&mut app);

    press_add_member(&mut app);

    let after = count_with::<MemberRow>(&mut app);
    assert_eq!(
        after,
        before + 1,
        "Add member must spawn exactly one new collapsed member row (C1)",
    );

    // The new row must descend from a ScrollListArea (the GTW-421/422 parenting rule — GOTCHA 1).
    let areas = all_with::<ScrollListArea>(&mut app);
    let rows = all_with::<MemberRow>(&mut app);
    let row = rows.first().copied().unwrap_or(Entity::PLACEHOLDER);
    assert!(
        areas.iter().any(|&area| is_ancestor(&app, area, row)),
        "a member row must be parented INSIDE a ScrollListArea (not the grid root frame) so it \
         top-anchors and scrolls (C1 / GOTCHA 1)",
    );
}

/// GTW-499 C1: a member row carries EXACTLY ONE control per field — the redundant doubled echo
/// labels are gone. For row 0 the editor must render exactly one name control ([`MemberNameField`]),
/// one weapon control ([`MemberWeaponDropdown`]), and one armor control ([`MemberArmorDropdown`]),
/// AND no extra static echo node beside any of them.
///
/// Pin-discriminating against the actual defect — the structural [`header_child_count`] assert: the
/// OLD doubled layout `add_children`d a static echo `Text` node BESIDE each editable control (a
/// SEPARATE entity carrying its OWN echo marker, not the editable marker), so its header held NINE
/// children; the fixed layout holds exactly SIX (pip, portrait, name field, weapon dropdown, armor
/// dropdown, delete). Re-introducing any of the three echo nodes (reverting the C1 fix) pushes the
/// header child count back above six and fails this assert — whereas the per-field control counts
/// alone stayed at 1 in BOTH layouts (the echo carried a distinct marker), so they cannot catch the
/// regression on their own.
#[test]
fn member_row_has_exactly_one_control_per_field() {
    let mut app = editor_app();
    press_add_member(&mut app);

    // One editable control per field (documents the per-field intent — but NOT the discriminating
    // assert: the old echo carried a distinct marker, so these counts were 1 in the doubled layout
    // too).
    assert_eq!(
        count_controls_for_row::<MemberNameField>(&mut app, 0),
        1,
        "row 0 must have exactly ONE name control (no redundant echo label — GTW-499 C1)",
    );
    assert_eq!(
        count_controls_for_row::<MemberWeaponDropdown>(&mut app, 0),
        1,
        "row 0 must have exactly ONE weapon control (no redundant echo label — GTW-499 C1)",
    );
    assert_eq!(
        count_controls_for_row::<MemberArmorDropdown>(&mut app, 0),
        1,
        "row 0 must have exactly ONE armor control (no redundant echo label — GTW-499 C1)",
    );

    // The discriminating pin: the header holds exactly the six controls — NOT the nine of the old
    // doubled layout (six + three static echo `Text` nodes). Re-adding any echo fails this.
    assert_eq!(
        header_child_count(&mut app, 0),
        6,
        "row 0's header must hold EXACTLY the six controls (pip, portrait, name field, weapon \
         dropdown, armor dropdown, delete) — the three redundant echo labels are gone; the old \
         doubled layout held nine, so re-adding any echo fails this (GTW-499 C1)",
    );
}

/// C3: a real `TextFieldCommitted` on a member's name field edits the model name.
///
/// Pin: if the commit listener drops the row-index mapping or never sets the name, the model name
/// stays at the default and the assert fails. The name field itself shows the committed value
/// (there is no separate echo node anymore — GTW-499 C1).
#[test]
fn commit_member_name_updates_model() {
    let mut app = editor_app();
    press_add_member(&mut app);
    let field = control_for_row::<MemberNameField>(&mut app, 0).unwrap_or(Entity::PLACEHOLDER);

    let committed = "Razor";
    app.world_mut().write_message(TextFieldCommitted::new(
        field,
        CommittedTextValue::new(committed),
    ));
    app.update();

    let model_name = app
        .world()
        .get_resource::<EditableGang>()
        .and_then(|model| model.member_at(0).map(|m| m.name().as_str().to_owned()));
    assert_eq!(
        model_name.as_deref(),
        Some(committed),
        "a commit on a member's name field must set THAT member's name in the model (C3)",
    );
}

/// C2: a real weapon `DropdownSelectionChanged` edits the model weapon.
///
/// Pin: a dropped selection leaves the model weapon unchanged (default empty) — fails the assert.
/// The dropdown's own label shows the chosen key (there is no separate echo node — GTW-499 C1).
#[test]
fn commit_member_weapon_updates_model() {
    let mut app = editor_app();
    press_add_member(&mut app);
    let control =
        control_for_row::<MemberWeaponDropdown>(&mut app, 0).unwrap_or(Entity::PLACEHOLDER);

    let chosen = WeaponName::new(WEAPON_B.to_owned());
    app.world_mut()
        .write_message(DropdownSelectionChanged::new(control, chosen));
    app.update();

    let model_weapon = app
        .world()
        .get_resource::<EditableGang>()
        .and_then(|model| model.member_at(0).map(|m| (**m.weapon()).clone()));
    assert_eq!(
        model_weapon.as_deref(),
        Some(WEAPON_B),
        "a weapon dropdown selection must set THAT member's weapon key in the model (C2)",
    );
}

/// C2: a real armor `DropdownSelectionChanged` edits the model armor (the armor mirror of
/// [`commit_member_weapon_updates_model`]).
#[test]
fn commit_member_armor_updates_model() {
    let mut app = editor_app();
    press_add_member(&mut app);
    let control =
        control_for_row::<MemberArmorDropdown>(&mut app, 0).unwrap_or(Entity::PLACEHOLDER);

    let chosen = ArmorName::new(ARMOR_B.to_owned());
    app.world_mut()
        .write_message(DropdownSelectionChanged::new(control, chosen));
    app.update();

    let model_armor = app
        .world()
        .get_resource::<EditableGang>()
        .and_then(|model| model.member_at(0).map(|m| (**m.armor()).clone()));
    assert_eq!(
        model_armor.as_deref(),
        Some(ARMOR_B),
        "an armor dropdown selection must set THAT member's armor key in the model (C2)",
    );
}

/// C4: a delete press removes the member from the model AND despawns its row.
///
/// Pin: a no-op delete leaves the model count + row count unchanged — either fails.
#[test]
fn delete_member_removes_from_model_and_list() {
    let mut app = editor_app();
    press_add_member(&mut app);
    press_add_member(&mut app);

    let model_before = app
        .world()
        .get_resource::<EditableGang>()
        .map(|model| model.members().len())
        .unwrap_or_default();
    let rows_before = count_with::<MemberRow>(&mut app);
    assert!(
        model_before >= 2,
        "precondition: at least two members to delete one"
    );

    // Delete the member at row index 0 through its real delete button + the real delete system.
    let delete = control_for_row::<DeleteMemberButton>(&mut app, 0).unwrap_or(Entity::PLACEHOLDER);
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(delete) {
        *interaction = Interaction::Pressed;
    }
    app.update();
    app.update();

    let model_after = app
        .world()
        .get_resource::<EditableGang>()
        .map(|model| model.members().len())
        .unwrap_or_default();
    let rows_after = count_with::<MemberRow>(&mut app);
    assert_eq!(
        model_after,
        model_before - 1,
        "delete must remove exactly one member from the model (C4)",
    );
    assert_eq!(
        rows_after,
        rows_before - 1,
        "delete must despawn exactly one member row (C4)",
    );
}

/// C5: editing ONE member leaves the OTHER rows' entity ids unchanged (proves a name/weapon/armor
/// edit MUTATES in place and never rebuilds the whole list).
///
/// Pin: if an edit rebuilt the list (despawn + respawn), the surviving member rows would get NEW
/// entity ids and the set-equality assert would fail.
#[test]
fn editing_one_member_leaves_other_rows_untouched() {
    let mut app = editor_app();
    press_add_member(&mut app);
    press_add_member(&mut app);

    // The row entity at index 1 (the member we will NOT edit).
    let other_row_before = control_for_row::<MemberRow>(&mut app, 1).unwrap_or(Entity::PLACEHOLDER);
    let all_rows_before: std::collections::BTreeSet<Entity> =
        all_with::<MemberRow>(&mut app).into_iter().collect();

    // Edit member 0's name via the real commit path.
    let field = control_for_row::<MemberNameField>(&mut app, 0).unwrap_or(Entity::PLACEHOLDER);
    app.world_mut().write_message(TextFieldCommitted::new(
        field,
        CommittedTextValue::new("Mutated"),
    ));
    app.update();

    let other_row_after = control_for_row::<MemberRow>(&mut app, 1).unwrap_or(Entity::PLACEHOLDER);
    let all_rows_after: std::collections::BTreeSet<Entity> =
        all_with::<MemberRow>(&mut app).into_iter().collect();

    assert_eq!(
        other_row_after, other_row_before,
        "editing member 0 must NOT change the OTHER row's entity id (no whole-list rebuild — C5)",
    );
    assert_eq!(
        all_rows_after, all_rows_before,
        "editing one member must leave EVERY row's entity id unchanged (mutate-not-rebuild — C5)",
    );
}
