//! The debug-keybind capture trigger: press a key to capture the primary window WITHOUT exiting.
//!
//! Unlike the env-var capture-then-exit path (for unattended QA runs), this is an INTERACTIVE dev
//! affordance — a human running the app presses a key and a PNG lands under `target/screenshots/`
//! with a per-press-unique name ([`timestamped_path`](crate::path::timestamped_path)), the app
//! keeps running. The consuming binary adds it ONLY under a debug/dev gate, so it is compiled OUT
//! of release.

use bevy::{
    prelude::*,
    render::view::window::screenshot::{Screenshot, save_to_disk},
};

use crate::path::timestamped_path;

/// The key that triggers an interactive capture. [`KeyCode::F10`] — a function key unlikely to clash
/// with gameplay/editor bindings. Framework plumbing (a key), not a domain value.
const CAPTURE_KEY: KeyCode = KeyCode::F10;

/// A scene tag prefixed onto the keybind-capture filename (`target/screenshots/<tag>-<secs>.png`),
/// so a capture reads as coming from a known app (`"editor"` / `"game"`).
///
/// A named newtype over `String` (no-bare-types): the tag is a domain label, not a bare string. The
/// inner is PRIVATE — construct via [`CaptureTag::new`], read via [`CaptureTag::as_str`].
#[derive(Clone, Debug)]
pub struct CaptureTag(String);

impl CaptureTag {
    /// Build a capture tag from a scene label (e.g. `"editor"`, `"game"`).
    #[must_use]
    pub fn new(tag: impl Into<String>) -> Self {
        Self(tag.into())
    }

    /// The tag as a string slice (for `timestamped_path`).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The debug-keybind capture plugin: press `CAPTURE_KEY` (F10) to capture the primary window to a
/// timestamped PNG under `target/screenshots/`, without exiting.
///
/// The consuming binary adds this ONLY under a debug/dev cfg gate (`cfg!(debug_assertions)` or the
/// `dev_capture` double-gate), so it is compiled OUT of release. It holds a [`CaptureTag`] for the
/// filename prefix.
pub struct KeyboardCapturePlugin {
    /// The scene tag prefixed onto each captured filename.
    tag: CaptureTag,
}

impl KeyboardCapturePlugin {
    /// Build the keybind-capture plugin for a scene, tagging its captures with `tag`
    /// (e.g. `"editor"`, `"game"`).
    #[must_use]
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            tag: CaptureTag::new(tag),
        }
    }
}

impl Plugin for KeyboardCapturePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(KeybindCaptureTag(self.tag.clone()))
            .add_systems(Update, capture_on_keypress);
    }
}

/// The keybind plugin's tag, held as a [`Resource`] so [`capture_on_keypress`] can read it.
/// A thin wrapper so the resource type is named (no-bare-types); the inner [`CaptureTag`] is itself
/// a newtype.
#[derive(Resource, Clone, Debug, Deref)]
struct KeybindCaptureTag(CaptureTag);

/// `Update`: on a fresh `CAPTURE_KEY` (F10) press, spawn ONE primary-window
/// [`Screenshot`](bevy::render::view::window::screenshot::Screenshot) with a
/// [`save_to_disk`](bevy::render::view::window::screenshot::save_to_disk) observer writing to a
/// timestamped `target/screenshots/<tag>-<secs>.png`. Does NOT exit — the app keeps running so the
/// dev can capture again. Param-only (`bevy-traps.md` #7).
fn capture_on_keypress(
    keys: Res<ButtonInput<KeyCode>>,
    tag: Res<KeybindCaptureTag>,
    mut commands: Commands,
) {
    if !keys.just_pressed(CAPTURE_KEY) {
        return;
    }
    let path = timestamped_path(tag.as_str());
    info!("gdtf_screenshot: keybind capture -> {}", path.display());
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk((*path).clone()));
}

#[cfg(test)]
mod tests {
    use super::CaptureTag;

    #[test]
    fn capture_tag_roundtrips() {
        assert_eq!(CaptureTag::new("editor").as_str(), "editor");
        assert_eq!(CaptureTag::new("game".to_owned()).as_str(), "game");
    }
}
