//! The DEV-ONLY procgen-visualizer INPUT-PANEL listeners (GTW-498) — selection / commit
//! readers that MUTATE [`VizConfig`](super::resource::VizConfig), the size-validity readout +
//! Generate-enable sync (C2), and the Generate action that re-runs procgen (C5).
//!
//! Apply-on-change vs Generate (C5 — the LOGGED choice): selecting an input only mutates the
//! [`VizConfig`]; it does NOT regenerate. The rendered level changes ONLY when the Generate
//! button is pressed — so the user composes a full {theme, size, seed, gangs} choice, then
//! commits it in one deterministic rebuild (the ticket's stated default). The numeric fields'
//! ranges already clamp out-of-range edits; an invalid SIZE *combo* (e.g. a zero axis — which
//! the `1..=` range prevents, but the validity check stays honest) disables Generate via
//! [`sync_size_status`].
//!
//! Each selection / commit reader follows the gang-editor precedent: read the widget's typed
//! [`DropdownSelectionChanged`] / [`NumericFieldCommitted`] message, match the changed control
//! by its marker, and mutate the config. All guard on the config resource existing
//! (`bevy-traps.md` #1). The whole module is `#[cfg(debug_assertions)]`-gated by its parent.

use bevy::prelude::*;
use gdtf_battle_sim::{
    ganger::{GangName, GangRegistry},
    level::{GridHeight, GridLevels, GridWidth, PrefabRegistry, ThemeUuid},
    procgen::ProcgenTuning,
    rng::BattleSeed,
};
use gdtf_ui::{DisabledButton, DropdownSelectionChanged, NumericFieldCommitted};

use crate::states::running::procgen_viz::{
    config::{
        components::{
            EnemyGangDropdown, GenerateButton, HeightField, LevelsField, PlayerGangDropdown,
            SeedField, SizeStatusText, ThemeDropdown, WidthField,
        },
        panel::size_status_text,
        resource::VizConfig,
    },
    model::ProcgenViz,
};

/// Apply a THEME-dropdown selection to the config (C1).
///
/// Reads [`DropdownSelectionChanged`]`<`[`ThemeUuid`]`>` (only the theme dropdown emits this `T`)
/// and, for a change on the [`ThemeDropdown`] control, sets the config's theme. Guarded on the
/// config resource. Param-only (`bevy-traps.md` #7).
pub(in crate::states::running::procgen_viz) fn apply_theme_selection(
    mut changes: MessageReader<DropdownSelectionChanged<ThemeUuid>>,
    controls: Query<Entity, With<ThemeDropdown>>,
    config: Option<ResMut<VizConfig>>,
) {
    let Some(mut config) = config else {
        return;
    };
    for change in changes.read() {
        if controls.contains(change.control()) {
            config.set_theme(*change.id());
        }
    }
}

/// Apply a PLAYER / ENEMY gang-dropdown selection to the config (C4).
///
/// Reads [`DropdownSelectionChanged`]`<`[`GangName`]`>` (both gang dropdowns share this `T`) and
/// routes each change to the player or enemy gang by which control marker the changed entity
/// carries. A change on neither (e.g. a stray control) is ignored. Guarded on the config
/// resource. Param-only (`bevy-traps.md` #7).
pub(in crate::states::running::procgen_viz) fn apply_gang_selection(
    mut changes: MessageReader<DropdownSelectionChanged<GangName>>,
    player: Query<Entity, With<PlayerGangDropdown>>,
    enemy: Query<Entity, With<EnemyGangDropdown>>,
    config: Option<ResMut<VizConfig>>,
) {
    let Some(mut config) = config else {
        return;
    };
    for change in changes.read() {
        if player.contains(change.control()) {
            config.set_player_gang(change.id().clone());
        } else if enemy.contains(change.control()) {
            config.set_enemy_gang(change.id().clone());
        }
    }
}

/// Apply a SIZE numeric-field commit to the config (C2).
///
/// Reads [`NumericFieldCommitted`]`<`[`u8`]`>` (the three grid-axis fields share this `N`) and
/// routes the committed, already-clamped value to width / height / levels by the committing
/// field's marker. The numeric field clamps into its `1..=MAX` range before committing, so the
/// value stored is always in-range per axis; an invalid full-combo is surfaced by
/// [`sync_size_status`]. Guarded on the config resource. Param-only (`bevy-traps.md` #7).
pub(in crate::states::running::procgen_viz) fn apply_size_commit(
    mut commits: MessageReader<NumericFieldCommitted<u8>>,
    width: Query<Entity, With<WidthField>>,
    height: Query<Entity, With<HeightField>>,
    levels: Query<Entity, With<LevelsField>>,
    config: Option<ResMut<VizConfig>>,
) {
    let Some(mut config) = config else {
        return;
    };
    for commit in commits.read() {
        let value = commit.value().value();
        let field = commit.field();
        if width.contains(field) {
            config.set_width(GridWidth::new(value));
        } else if height.contains(field) {
            config.set_height(GridHeight::new(value));
        } else if levels.contains(field) {
            config.set_levels(GridLevels::new(value));
        }
    }
}

/// Apply a SEED numeric-field commit to the config (C3).
///
/// Reads [`NumericFieldCommitted`]`<`[`u64`]`>` (only the seed field uses this `N`) and, for a
/// commit on the [`SeedField`], sets the config's seed. A different seed yields a different
/// placement on the next Generate; the same seed reproduces it (C3 determinism). Guarded on the
/// config resource. Param-only (`bevy-traps.md` #7).
pub(in crate::states::running::procgen_viz) fn apply_seed_commit(
    mut commits: MessageReader<NumericFieldCommitted<u64>>,
    fields: Query<Entity, With<SeedField>>,
    config: Option<ResMut<VizConfig>>,
) {
    let Some(mut config) = config else {
        return;
    };
    for commit in commits.read() {
        if fields.contains(commit.field()) {
            config.set_seed(BattleSeed::new(commit.value().value()));
        }
    }
}

/// Keep the size-validity STATUS text + the Generate ENABLE state in sync with the config (C2).
///
/// Runs whenever the config changes (`Changed<VizConfig>` would not fire on a `Res` — the
/// resource is `ResMut`-mutated, so this reads it each frame and writes only on a difference,
/// the mutate-in-place rule). Writes the [`SizeStatusText`] to `OK WxHxL` for a valid combo or
/// the [`GridSizeError`](gdtf_battle_sim::level::GridSizeError) message otherwise, and toggles the
/// [`GenerateButton`]'s [`DisabledButton`] marker so an invalid size CANNOT regenerate (C2/C5).
/// Guarded on the config resource. Param-only (`bevy-traps.md` #7).
pub(in crate::states::running::procgen_viz) fn sync_size_status(
    config: Option<Res<VizConfig>>,
    mut commands: Commands,
    mut status: Query<&mut Text, With<SizeStatusText>>,
    generate: Query<(Entity, Has<DisabledButton>), With<GenerateButton>>,
) {
    let Some(config) = config else {
        return;
    };
    let valid = config.grid_size().is_ok();
    let want_text = size_status_text(&config);
    for mut text in &mut status {
        if text.0 != want_text {
            text.0.clone_from(&want_text);
        }
    }
    // Enable Generate only when the size combo is valid (C2): add / remove the DisabledButton
    // marker in place (the gang-editor disable precedent) so the press reader is gated.
    for (entity, is_disabled) in &generate {
        match (valid, is_disabled) {
            (true, true) => {
                commands.entity(entity).remove::<DisabledButton>();
            }
            (false, false) => {
                commands.entity(entity).insert(DisabledButton);
            }
            _ => {}
        }
    }
}

/// Read-only query of freshly-pressed ENABLED Generate buttons — a `Changed<Interaction>` filter
/// restricted to [`GenerateButton`]s that are NOT [`DisabledButton`] (so an invalid size never
/// regenerates, C2). Named to keep [`generate_on_press`]'s signature legible (clippy
/// `type_complexity`).
type EnabledGeneratePresses<'w, 's> = Query<
    'w,
    's,
    &'static Interaction,
    (
        Changed<Interaction>,
        With<GenerateButton>,
        Without<DisabledButton>,
    ),
>;

/// GENERATE (C5, the feature trigger) — on a fresh press of the ENABLED Generate button, re-run
/// the procgen pipeline from the current [`VizConfig`], REBUILD the [`ProcgenViz`] model (board +
/// ordered quads), apply the chosen-gang deployment-quad labels (C4), and RESET the reveal to 0.
///
/// Reads `Changed<Interaction>` filtered to enabled Generate buttons (`Without<DisabledButton>`
/// — so an invalid size never regenerates, C2). On a press it folds the config's three axes
/// through the validated [`VizConfig::grid_size`] (skipping the rebuild if invalid — defence in
/// depth on top of the disable) and replaces the model resource via
/// [`ProcgenViz::build_with_gangs`] with a fresh [`ProcgenRng`](gdtf_battle_sim::rng::ProcgenRng) from
/// the selected seed. The existing STEP / AUTO controls + the draw sync then step the NEW result
/// (the spawned quads are rebuilt by `respawn_quads_on_generate`, which runs after this).
/// Guarded on the config resource. Param-only (`bevy-traps.md` #7).
pub(in crate::states::running::procgen_viz) fn generate_on_press(
    buttons: EnabledGeneratePresses,
    config: Option<Res<VizConfig>>,
    prefabs: Option<Res<PrefabRegistry>>,
    gangs: Option<Res<GangRegistry>>,
    // GTW-533: the LIVE, hot-reloaded procgen fill tuning; a re-Generate after an edit to
    // `core_tuning/procgen.tuning.ron` re-tunes the visualizer's fill (absent ⇒ default).
    procgen_tuning: Option<Res<ProcgenTuning>>,
    mut model: ResMut<ProcgenViz>,
) {
    let Some(config) = config else {
        return;
    };
    let pressed = buttons
        .iter()
        .any(|interaction| matches!(interaction, Interaction::Pressed));
    if !pressed {
        return;
    }
    // Defence in depth (the button is already disabled on an invalid size, C2): only rebuild on
    // a valid combo, so an invalid size NEVER regenerates and never panics.
    let Ok(grid_size) = config.grid_size() else {
        return;
    };
    *model = ProcgenViz::build_with_gangs(
        prefabs.as_deref(),
        config.theme(),
        grid_size,
        config.seed(),
        gangs.as_deref(),
        config.player_gang(),
        config.enemy_gang(),
        procgen_tuning.as_deref(),
    );
}
