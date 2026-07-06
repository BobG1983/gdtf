//! The editor app, seeded registries, and shared row/tuning accessors.

use bevy::{
    app::App,
    ecs::{component::Component, entity::Entity},
    prelude::With,
    state::state::NextState,
};
use gdtf_app::test_support::{AddMemberButton, AppState, MemberRowIndex, RunningState};
use gdtf_battle_sim::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorName, ArmorPiece, ArmorProtection,
    ArmorRegistry, ArmorSpec, ArmorType, FatalBias, GangerStatTuning, WeaponName, WeaponSpec,
    test_support::test_weapon_spec,
};
use gdtf_test_utils::{GdtfTestAppBuilder, press_ui_button};
use gdtf_ui::theme::default_theme;

/// A weapon key (the dropdowns need at least one option to spawn cleanly).
pub(crate) const WEAPON_A: &str = "Autogun";
/// An armor key (mirrors [`WEAPON_A`]).
pub(crate) const ARMOR_A: &str = "Flak";

/// An arbitrary weapon spec (NOT shipped tuning) — the editor only reads the KEY.
pub(crate) fn arbitrary_weapon() -> WeaponSpec {
    WeaponSpec {
        // Not Fatal-skewed (the editor never resolves a wound) — the only divergence
        // from the canonical fixture.
        fatal_bias: FatalBias::new(0.0),
        ..test_weapon_spec()
    }
}

/// A [`WeaponRegistry`](gdtf_battle_sim::WeaponRegistry) of one test weapon key.
pub(crate) fn weapon_registry() -> gdtf_battle_sim::WeaponRegistry {
    gdtf_battle_sim::WeaponRegistry::new([(
        WeaponName::new(WEAPON_A.to_owned()),
        arbitrary_weapon(),
    )])
}

/// An arbitrary armor spec (NOT shipped tuning).
pub(crate) const fn arbitrary_armor() -> ArmorSpec {
    ArmorSpec::uniform(ArmorPiece::new(
        ArmorFloor::new(0),
        ArmorProtection::new(1),
        ArmorIntegrity::new(2),
        ArmorHardness::new(3),
        ArmorType::DEFAULT,
    ))
}

/// An [`ArmorRegistry`] of one test armor key.
pub(crate) fn armor_registry() -> ArmorRegistry {
    ArmorRegistry::new([(ArmorName::new(ARMOR_A.to_owned()), arbitrary_armor())])
}

/// Builds a headless app driven into [`RunningState::DebugEditor`] with the editor screen spawned,
/// the weapon / armor registries seeded (so the per-member dropdowns have options), and the
/// [`GangerStatTuning`] seeded (so the production recompute and the test compute through the SAME
/// derivation weights — C3). Starts in `AppState::Running` (default sub-state `Menu`), seeds the
/// resources before the first update so the `OnEnter` spawn sees them, then sets the `DebugEditor`
/// transition and pumps a few updates so the screen + any deferred parenting flush.
pub(crate) fn editor_app() -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(weapon_registry());
    app.world_mut().insert_resource(armor_registry());
    app.world_mut().insert_resource(test_tuning());
    app.update();
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::DebugEditor);
    app.update();
    app.update();
    app
}

/// The derivation tuning the test and the production code share — the const default
/// [`GangerStatTuning`] (the stats.md flat weights). Both the editor's recompute and the test's
/// expected derivation read THIS, so the comparison is exact.
pub(crate) fn test_tuning() -> GangerStatTuning {
    GangerStatTuning::default()
}

/// Presses "Add member" through the real add-member system (set `Interaction::Pressed` + a couple
/// `update()`s, the headless idiom — the deferred parent-into-area command needs a second flush).
pub(crate) fn press_add_member(app: &mut App) {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<AddMemberButton>>();
    let button = q.iter(app.world()).next().unwrap_or(Entity::PLACEHOLDER);
    press_ui_button(app, button);
    app.update();
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
