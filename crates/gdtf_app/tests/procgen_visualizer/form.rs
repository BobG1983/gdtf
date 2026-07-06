//! GTW-498 widget-message form-driving vocabulary shared by the `config_form` and
//! `generate_drive` tests.

use bevy::ecs::{component::Component, entity::Entity};
use gdtf_app::test_support::{GenerateButton, ProcgenViz, SeedField, ThemeDropdown, VizConfig};
use gdtf_battle_sim::{ganger::GangName, level::ThemeUuid};
use gdtf_ui::{CommittedNumericValue, DropdownSelectionChanged, NumericFieldCommitted};

use super::harness::*;

// ---------------------------------------------------------------------------------------------
// GTW-498 — the configurable inputs + Generate (C1–C5), driven on the REAL widget-message path.
// ---------------------------------------------------------------------------------------------

/// Write a real [`NumericFieldCommitted`]`<`[`u8`]`>` for the field carrying marker `M` (the
/// grid-axis commit the keyboard observer raises on Enter / blur), then update once — driving the
/// real `apply_size_commit` listener.
pub(crate) fn commit_u8_field<M: Component>(app: &mut bevy::app::App, value: u8) {
    let field = single_with::<M>(app).unwrap_or(Entity::PLACEHOLDER);
    app.world_mut().write_message(NumericFieldCommitted::new(
        field,
        CommittedNumericValue::new(value),
    ));
    app.update();
}

/// Write a real [`NumericFieldCommitted`]`<`[`u64`]`>` for the seed field, then update once —
/// driving the real `apply_seed_commit` listener.
pub(crate) fn commit_seed(app: &mut bevy::app::App, value: u64) {
    let field = single_with::<SeedField>(app).unwrap_or(Entity::PLACEHOLDER);
    app.world_mut().write_message(NumericFieldCommitted::new(
        field,
        CommittedNumericValue::new(value),
    ));
    app.update();
}

/// Write a real [`DropdownSelectionChanged`]`<`[`ThemeUuid`]`>` for the theme dropdown, then
/// update once — driving the real `apply_theme_selection` listener.
pub(crate) fn select_theme(app: &mut bevy::app::App, theme: ThemeUuid) {
    let control = single_with::<ThemeDropdown>(app).unwrap_or(Entity::PLACEHOLDER);
    app.world_mut()
        .write_message(DropdownSelectionChanged::new(control, theme));
    app.update();
}

/// Write a real [`DropdownSelectionChanged`]`<`[`GangName`]`>` for the gang dropdown carrying
/// marker `M`, then update once — driving the real `apply_gang_selection` listener.
pub(crate) fn select_gang<M: Component>(app: &mut bevy::app::App, gang: GangName) {
    let control = single_with::<M>(app).unwrap_or(Entity::PLACEHOLDER);
    app.world_mut()
        .write_message(DropdownSelectionChanged::new(control, gang));
    app.update();
}

/// The current [`VizConfig`], if present (the `OnEnter` insert guarantees it; callers `assert!`
/// on the `Some` so a dropped config reddens loudly without an `expect`).
pub(crate) fn config(app: &bevy::app::App) -> Option<VizConfig> {
    app.world().get_resource::<VizConfig>().cloned()
}

/// The current config's selected theme, if the config is present.
pub(crate) fn config_theme(app: &bevy::app::App) -> Option<ThemeUuid> {
    config(app).map(|c| c.theme())
}

/// Whether the current config's size combo validates (`false` if the config is absent).
pub(crate) fn config_size_valid(app: &bevy::app::App) -> bool {
    config(app).is_some_and(|c| c.grid_size().is_ok())
}

/// Whether the (single) Generate button currently carries the
/// [`DisabledButton`](gdtf_ui::DisabledButton) marker — `false` if it is enabled or absent.
pub(crate) fn generate_disabled(app: &mut bevy::app::App) -> bool {
    single_with::<GenerateButton>(app)
        .is_some_and(|e| app.world().get::<gdtf_ui::DisabledButton>(e).is_some())
}

/// The board's `(width, height)` cell dimensions of the current model.
pub(crate) fn board_dims(app: &bevy::app::App) -> (u32, u32) {
    app.world()
        .get_resource::<ProcgenViz>()
        .map(ProcgenViz::board_dimensions)
        .unwrap_or_default()
}
