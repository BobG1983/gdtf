//! The map editor's own minimal state machine.
//!
//! The editor is a SEPARATE binary from the game (GTW-417 housing constraint), so it
//! runs its OWN small lifecycle rather than the game's `AppState` (Init/Load/Intro/
//! Running/Teardown) and its full scene graph. Two states suffice for the shell:
//! a [`Load`](EditorState::Load) asset pass that mirrors the game's `resolve_*`
//! loaders (theme + theme-tile catalog + weapon/armor registries), then
//! [`Editing`](EditorState::Editing) where the four layout regions render and later
//! children (the right panel, the left palette, the canvas) populate them.

use bevy::prelude::*;

/// Top-level lifecycle state for the map editor binary.
///
/// A deliberately minimal two-state machine — the editor does NOT reuse the game's
/// `AppState` or its scene plugins. [`Load`](EditorState::Load) gates on the asset
/// pass (theme + registries) completing; [`Editing`](EditorState::Editing) is the
/// authoring scene where the empty themed regions live (later children fill them).
#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
pub enum EditorState {
    /// One-shot asset load: resolve the [`GdtfTheme`](gdtf_ui::theme::GdtfTheme) and
    /// the theme-tile-catalog / weapon / armor registries before the editor opens.
    /// The default so the editor boots straight into loading.
    #[default]
    Load,
    /// The authoring scene: the four themed layout regions render here.
    Editing,
}
