//! The editor app, seeded registries, and shared row/control accessors.

use bevy::{
    app::App,
    ecs::{component::Component, entity::Entity},
    prelude::With,
    state::state::NextState,
};
use gdtf_app::test_support::{AddMemberButton, AppState, MemberRowIndex, RunningState};
use gdtf_battle_sim::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorName, ArmorPiece, ArmorProtection,
    ArmorRegistry, ArmorSpec, ArmorType, FatalBias, WeaponName, WeaponRegistry, WeaponSpec,
    test_support::test_weapon_spec,
};
use gdtf_test_utils::{GdtfTestAppBuilder, press_ui_button};
use gdtf_ui::theme::default_theme;

/// Two weapon keys (sorted: "Autogun" < "Lasgun"), so a dropdown selection has a DISTINCT target
/// to commit (a single-option dropdown could never prove a change). Their specs are arbitrary
/// (NOT shipped magnitudes — the brittle-test rule); only the KEYS matter to the dropdown.
pub(crate) const WEAPON_A: &str = "Autogun";
/// The second weapon key (see [`WEAPON_A`]).
pub(crate) const WEAPON_B: &str = "Lasgun";
/// Two armor keys (sorted: "Flak" < "Mesh"), mirroring the weapon keys.
pub(crate) const ARMOR_A: &str = "Flak";
/// The second armor key (see [`ARMOR_A`]).
pub(crate) const ARMOR_B: &str = "Mesh";

/// An arbitrary weapon spec (NOT shipped tuning) — the dropdown only reads the KEY, so the numbers
/// are immaterial. Mirrors `action_bar.rs`'s `armed_registry` shape.
pub(crate) fn arbitrary_weapon() -> WeaponSpec {
    WeaponSpec {
        // Not Fatal-skewed (the editor never resolves a wound) — the only divergence
        // from the canonical fixture.
        fatal_bias: FatalBias::new(0.0),
        ..test_weapon_spec()
    }
}

/// A [`WeaponRegistry`] of the two test weapon keys.
pub(crate) fn weapon_registry() -> WeaponRegistry {
    WeaponRegistry::new([
        (WeaponName::new(WEAPON_A.to_owned()), arbitrary_weapon()),
        (WeaponName::new(WEAPON_B.to_owned()), arbitrary_weapon()),
    ])
}

/// An arbitrary armor spec (NOT shipped tuning) — the dropdown only reads the KEY.
pub(crate) const fn arbitrary_armor() -> ArmorSpec {
    ArmorSpec::uniform(ArmorPiece::new(
        ArmorFloor::new(0),
        ArmorProtection::new(1),
        ArmorIntegrity::new(2),
        ArmorHardness::new(3),
        ArmorType::DEFAULT,
    ))
}

/// An [`ArmorRegistry`] of the two test armor keys.
pub(crate) fn armor_registry() -> ArmorRegistry {
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
pub(crate) fn editor_app() -> App {
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
pub(crate) fn count_with<M: Component>(app: &mut App) -> usize {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    q.iter(app.world()).count()
}

/// Every entity carrying marker `M`, in arbitrary order.
pub(crate) fn all_with<M: Component>(app: &mut App) -> Vec<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    q.iter(app.world()).collect()
}

/// Presses "Add member" through the real add-member system (set `Interaction::Pressed` + a single
/// `update()`, the headless idiom — `MinimalPlugins` has no `ui_focus_system` to clobber it).
pub(crate) fn press_add_member(app: &mut App) {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<AddMemberButton>>();
    let button = q.iter(app.world()).next().unwrap_or(Entity::PLACEHOLDER);
    press_ui_button(app, button);
    app.update();
    // Flush the deferred parent-into-area command + let the new row settle.
    app.update();
}

/// The first entity carrying BOTH marker `M` and a [`MemberRowIndex`] equal to `index`.
pub(crate) fn control_for_row<M: Component>(app: &mut App, index: usize) -> Option<Entity> {
    let mut q = app
        .world_mut()
        .query_filtered::<(Entity, &MemberRowIndex), With<M>>();
    q.iter(app.world())
        .find(|(_, row_index)| ***row_index == index)
        .map(|(entity, _)| entity)
}
