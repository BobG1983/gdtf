//! The map editor's own minimal state machine.
//!
//! The editor is a SEPARATE binary from the game (GTW-417 housing constraint), so it
//! runs its OWN small lifecycle rather than the game's `AppState` (Init/Load/Intro/
//! Running/Teardown) and its full scene graph. Two states suffice for the shell:
//! a [`Load`](EditorState::Load) asset pass that registers the SAME generic asset
//! loaders the game does (GTW-579 — tile roles + the weapon/armor/terrain/theme
//! registries; the game's UI theme is NOT among them — GTW-625), then
//! [`Editing`](EditorState::Editing) where the authoring scene lives.

use bevy::prelude::*;

/// Top-level lifecycle state for the map editor binary.
///
/// A deliberately minimal two-state machine — the editor does NOT reuse the game's
/// `AppState` or its scene plugins. [`Load`](EditorState::Load) gates on the asset
/// pass (registries + tile roles) completing; [`Editing`](EditorState::Editing) is the
/// authoring scene where the empty themed regions live (later children fill them).
#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
pub enum EditorState {
    /// The asset pass: the generic loaders resolve the tile-role table and the
    /// weapon / armor / terrain-def / theme-def registries before the editor opens.
    /// The default so the editor boots straight into loading.
    #[default]
    Load,
    /// The authoring scene: the four themed layout regions render here.
    Editing,
}
