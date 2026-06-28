//! DEV-ONLY gang-editor self-screenshot QA hook (GTW-420, C6).
//!
//! This is **not shipping behavior**. It exists so QA (or a coding agent) can drive the app into
//! the gang editor and capture the rendered editor screen — proving C6 visually, which the
//! headless tests structurally cannot observe. It mirrors the GTW-419 loading-screen capture /
//! GTW-297 [`DevCapturePlugin`](crate::app::capture) gating discipline.
//!
//! ## Two gates, both must hold to activate
//!
//! 1. **Dev cfg.** Wired into [`EditorScenePlugin`](super::plugin::EditorScenePlugin) only under
//!    `cfg!(all(debug_assertions, feature = "dev_capture"))`; a release / default build never
//!    compiles it.
//! 2. **Opt-in env var.** Even when compiled in it is inert until `GDTF_EDITOR_SCREEN_SHOT=/abs/out.png`
//!    is set: with it unset the hook registers nothing.
//!
//! ## How it drives + captures
//!
//! When active it (a) drives the menu into [`RunningState::DebugEditor`] the moment the menu
//! rests (the only non-automatic transition from launch — the editor screen then spawns
//! `OnEnter`), (b) once in `DebugEditor` presses "Add member" (a populated row), then EXPANDS that
//! row's pip + commits a Grit attribute edit (so the GTW-428 stat table is open via the accordion
//! lerp and the readonly derived stats have visibly recomputed), (c) waits a settle so the
//! accordion has fully lerped open + the layout flushed, captures the editor screen, and (d) rides
//! the shared shutdown cascade by setting [`RunningState::Quit`] (NOT writing `AppExit` — the macOS
//! winit hang, Bevy #23313). The captured PNG shows the expanded per-member stat table: the eight
//! editable attribute fields + the readonly derived displays recomputed off the edited Grit.

use std::path::PathBuf;

use bevy::{
    prelude::*,
    render::view::window::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
    ui::Interaction,
};
use gdtf_ui::{AccordionAnim, CommittedNumericValue, NumericFieldCommitted};

use crate::states::{
    RunningState,
    running::editor::components::{
        AddMemberButton, AttributeField, BaseAttribute, EditorScreenRoot, ExpandPip, MemberRow,
        MemberRowIndex, MemberStatPanel, PipExpanded,
    },
};

/// The `GDTF_EDITOR_SCREEN_SHOT` env var: the absolute path of the output PNG. Setting it (in a
/// `dev_capture` debug build) opts into the editor capture hook.
const EDITOR_SHOT_ENV: &str = "GDTF_EDITOR_SCREEN_SHOT";

/// How many `DebugEditor` frames to wait before capturing, so the UI layout has flushed and the
/// editor screen is settled rather than mid-layout (the GTW-419 settle precedent). Bumped well over
/// the GTW-425 row-only capture so the full GTW-428 drive applies before the shot — the "Add
/// member" press, then the pip-expand + attribute-edit (so the accordion has FULLY LERPED open and
/// the live derived recompute has run), are all settled by the captured frame (GTW-428 C4). The
/// accordion full open is `~0.25 s` at the default frame rate, so a generous margin is used.
const SETTLE_FRAMES: u32 = 90;

/// The Grit value the capture drive commits into the expanded panel, so the shot's readonly derived
/// stats (HP / Wounds / Morale …) visibly move off their all-default values — proving the live
/// recompute on screen (GTW-428 C4). A framework-plumbing capture magnitude, not a domain value.
const CAPTURE_GRIT: f32 = 8.0;

/// The expanded-state glyph the capture drive writes onto the row-0 pip when it opens the panel
/// DIRECTLY — the exact `-` glyph
/// [`toggle_expand_pip`](super::systems::toggle_expand_pip) would write on a real press.
const EXPANDED_PIP_GLYPH: &str = "-";

/// Whether the editor capture hook is enabled, and where it writes.
///
/// `Some(path)` when [`EDITOR_SHOT_ENV`] is set to a non-empty (trimmed) value; `None` (the hook
/// stays inert) otherwise. The path is framework plumbing handed straight to
/// [`save_to_disk`](bevy::render::view::window::screenshot::save_to_disk) — not a domain value.
///
/// Pure (no `World`); delegates the gate to [`parse_editor_shot_path`] so the config test can
/// drive the SAME logic without mutating the process-global env var.
#[must_use]
pub(in crate::states::running::editor) fn editor_shot_path() -> Option<PathBuf> {
    parse_editor_shot_path(std::env::var(EDITOR_SHOT_ENV).ok().as_deref())
}

/// Apply the path gate to a raw env-var value: `Some(path)` when non-empty (trimmed), `None`
/// (hook inert) when absent / empty / all-whitespace. The pure core of [`editor_shot_path`].
#[must_use]
fn parse_editor_shot_path(value: Option<&str>) -> Option<PathBuf> {
    value
        .map(|raw| raw.trim().to_owned())
        .filter(|trimmed| !trimmed.is_empty())
        .map(PathBuf::from)
}

/// The resolved editor-shot configuration: where to write the captured PNG.
///
/// Inserted as a [`Resource`] when the hook is enabled, so the capture system can read its path.
/// Framework-plumbing config (a path), not a domain value.
#[derive(Resource, Debug, Clone)]
pub(in crate::states::running::editor) struct EditorShotConfig {
    /// Absolute path of the output PNG, handed to
    /// [`save_to_disk`](bevy::render::view::window::screenshot::save_to_disk).
    path: PathBuf,
}

impl EditorShotConfig {
    /// Build the config from the resolved output path.
    pub(in crate::states::running::editor) const fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

/// Drives the menu into [`RunningState::DebugEditor`] once the menu rests (the only non-automatic
/// launch transition), so the capture run reaches the editor unattended.
///
/// Runs in `Update`, gated `run_if(in_state(RunningState::Menu))` + the config resource existing.
/// Param-only (`bevy-traps.md` #7).
fn drive_into_editor(mut next: ResMut<NextState<RunningState>>) {
    next.set(RunningState::DebugEditor);
}

/// Ensures the captured editor frame shows at least one POPULATED member row + its dropdowns
/// (GTW-425 C6): once in `DebugEditor`, if the list has no rows yet it presses "Add member" ONCE
/// by setting the button's [`Interaction::Pressed`] directly — the real
/// [`add_member_on_press`](super::systems::add_member_on_press) system then appends a member +
/// spawns the row on the next frame.
///
/// Sets the interaction directly (not via a synthesized pointer) because the windowed
/// `ui_focus_system` would clear a synthesized `Pressed` before the driver reads it (the GTW-422
/// `drive_capture_selection` precedent — bevy-traps #6). A [`Local<bool>`] makes it fire ONCE, so
/// it does not spam members every frame. Runs in `Update`, gated on `DebugEditor` + the config
/// resource. Param-only (`bevy-traps.md` #7).
fn drive_capture_add_member(
    rows: Query<(), With<MemberRow>>,
    mut buttons: Query<&mut Interaction, With<AddMemberButton>>,
    mut pressed_once: Local<bool>,
) {
    if *pressed_once || rows.iter().next().is_some() {
        // Already pressed (or a loaded gang already has rows) — leave the list alone.
        return;
    }
    if let Some(mut interaction) = buttons.iter_mut().next() {
        *interaction = Interaction::Pressed;
        *pressed_once = true;
    }
}

/// Expands the first member's stat panel AND commits a Grit attribute edit, so the captured frame
/// shows the EXPANDED stat table + the LIVE derived recompute + an open accordion (GTW-428 C4).
///
/// Runs in `Update`, gated on `DebugEditor` + the config resource, ordered AFTER
/// [`drive_capture_add_member`]. The member row is spawned by a DEFERRED `commands.queue` in
/// [`add_member_on_press`](super::systems::add_member_on_press), so the row + its pip + its
/// [`MemberStatPanel`] do NOT exist the same frame the add-member press fires — this driver GATES on
/// the row-0 panel actually being present (across frames) and does nothing until it is.
///
/// It then opens the panel by DRIVING THE EXPAND STATE DIRECTLY rather than faking an
/// [`Interaction::Pressed`] on the pip: a synthesized `Pressed` is unreliable under the ambiguous
/// ordering between this driver and the [`toggle_expand_pip`](super::systems::toggle_expand_pip)
/// reader (`bevy-traps.md` #3 / #6 — the GTW-416 capture/demo precedent drives `AccordionAnim`
/// directly for exactly this reason). Concretely it applies the EXACT state a real pip click would
/// produce: (1) sets the row-0 [`ExpandPip`]'s [`PipExpanded`] to `true` and re-glyphs its [`Text`]
/// to `-`, and (2) sets the row-0 [`MemberStatPanel`]'s
/// [`AccordionAnim`](gdtf_ui::AccordionAnim) to [`Expanding`](gdtf_ui::AccordionAnim::Expanding) (=
/// `Collapsed.toggled()`) so the shared `drive_accordions` lerps the panel open over the settle
/// window. (3) It writes a real [`NumericFieldCommitted`]`<f32>` for the row-0 Grit
/// [`AttributeField`] so the real [`commit_member_attribute`](super::systems::commit_member_attribute)
/// re-derives + mutates the readonly displays. A [`Local<bool>`] makes it fire ONCE — set only once
/// the panel was found and driven. Param-only (`bevy-traps.md` #7).
fn drive_capture_expand_and_edit(
    mut pips: Query<(&MemberRowIndex, &mut PipExpanded, &mut Text), With<ExpandPip>>,
    mut panels: Query<(&MemberRowIndex, &mut AccordionAnim), With<MemberStatPanel>>,
    fields: Query<(Entity, &MemberRowIndex, &BaseAttribute), With<AttributeField>>,
    mut commits: MessageWriter<NumericFieldCommitted<f32>>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    // Gate on the row-0 panel existing: the member row + its pip + panel are spawned by a DEFERRED
    // command, so they are absent for the frame(s) after the add-member press. Drive nothing — and
    // do NOT mark done — until the panel is actually present.
    let Some(()) = panels
        .iter_mut()
        .find(|(index, _)| ***index == 0)
        .map(|(_, mut anim)| {
            // Set the panel to the exact state a real click produces (`Collapsed.toggled()` =
            // `Expanding`) so the shared `drive_accordions` lerps it open over the settle window.
            *anim = AccordionAnim::Expanding;
        })
    else {
        return;
    };
    // Apply the matching pip state a real toggle would write: PipExpanded(true) + the `-` glyph.
    if let Some((_, mut expanded, mut text)) = pips.iter_mut().find(|(index, ..)| ***index == 0) {
        *expanded = PipExpanded::new(true);
        if text.0 != EXPANDED_PIP_GLYPH {
            EXPANDED_PIP_GLYPH.clone_into(&mut text.0);
        }
    }
    // Commit a Grit edit on the row-0 attribute field so the readonly derived displays recompute.
    if let Some((field, ..)) = fields
        .iter()
        .find(|(_, index, attribute)| ***index == 0 && **attribute == BaseAttribute::Grit)
    {
        commits.write(NumericFieldCommitted::new(
            field,
            CommittedNumericValue::new(CAPTURE_GRIT),
        ));
    }
    *done = true;
}

/// Captures the rendered editor-screen frame to disk after a brief settle, then exits the app via
/// the shared shutdown cascade.
///
/// Runs in `Update`, gated `run_if(in_state(RunningState::DebugEditor))` + the config resource
/// existing. Its [`Local<u32>`] counter increments each `DebugEditor` frame; on the settle frame
/// ([`SETTLE_FRAMES`]) it spawns a [`Screenshot::primary_window`] entity with an observer that
/// saves the PNG synchronously and then sets [`RunningState::Quit`] (the shared cascade — NOT
/// `AppExit`, the macOS hang #23313). Guards on the editor root being present so it never captures
/// a blank screen if the theme was absent. Param-only (`bevy-traps.md` #7).
///
/// The actual screenshot needs a real render device, so it CANNOT be headless-tested — it is
/// verified by RUNNING the app (QA), then `Read`ing the PNG.
fn capture_editor_screen(
    mut commands: Commands,
    config: Res<EditorShotConfig>,
    screens: Query<(), With<EditorScreenRoot>>,
    mut frames: Local<u32>,
) {
    *frames += 1;
    if *frames != SETTLE_FRAMES {
        // Before the settle frame: wait. After it: already captured, do nothing.
        return;
    }
    if screens.iter().next().is_none() {
        // No editor screen yet (theme absent at spawn) — skip rather than capture a blank frame.
        return;
    }
    let path = config.path.clone();
    commands.spawn(Screenshot::primary_window()).observe(
        move |captured: On<ScreenshotCaptured>, mut next: ResMut<NextState<RunningState>>| {
            // Flush the PNG synchronously, then ride the shared shutdown cascade (Quit ->
            // AppState::Teardown), not a direct AppExit (Bevy #23313 macOS hang).
            save_to_disk(&path)(captured);
            next.set(RunningState::Quit);
        },
    );
}

/// Register the editor capture hook IF its env-var gate is set.
///
/// Called by [`EditorScenePlugin`](super::plugin::EditorScenePlugin) only under
/// `cfg!(all(debug_assertions, feature = "dev_capture"))`. When [`editor_shot_path`] returns
/// `None` it registers nothing (the hook is fully inert).
pub(in crate::states::running::editor) fn register_editor_capture(app: &mut App) {
    let Some(path) = editor_shot_path() else {
        return;
    };
    info!("gang-editor capture: ON (dev) -> {}", path.display());
    app.insert_resource(EditorShotConfig::new(path))
        .add_systems(
            Update,
            drive_into_editor
                .run_if(in_state(RunningState::Menu).and_then(resource_exists::<EditorShotConfig>)),
        )
        .add_systems(
            Update,
            // Drive the full GTW-428 demo before the settle-capture: press "Add member" (a
            // populated row), then expand its pip + commit a Grit edit (the open accordion stat
            // table + the live recompute), THEN capture. Chained so each step's effect is applied
            // before the next reads it and the capture frame is fully settled.
            (
                drive_capture_add_member,
                drive_capture_expand_and_edit,
                capture_editor_screen,
            )
                .chain()
                .run_if(
                    in_state(RunningState::DebugEditor)
                        .and_then(resource_exists::<EditorShotConfig>),
                ),
        );
}

#[cfg(test)]
mod tests {
    use super::parse_editor_shot_path;

    #[test]
    fn path_gate_accepts_non_empty_and_rejects_blank() {
        assert!(parse_editor_shot_path(Some("/abs/out.png")).is_some());
        assert!(parse_editor_shot_path(Some("  /trim/out.png  ")).is_some());
        assert!(parse_editor_shot_path(Some("")).is_none());
        assert!(parse_editor_shot_path(Some("   ")).is_none());
        assert!(parse_editor_shot_path(None).is_none());
    }
}
