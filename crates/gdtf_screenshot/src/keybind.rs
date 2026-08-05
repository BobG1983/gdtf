//! Keyboard-triggered screenshot capture (F10).

use bevy::prelude::*;

use crate::capture::{
    CaptureCompletions, CaptureOutcome, CapturePipelinePlugin, CaptureQueue, CaptureSystems,
    ShotStem,
};

const CAPTURE_KEY: KeyCode = KeyCode::F10;

/// Scene label baked into the capture's file name.
#[derive(Clone, Debug)]
pub struct CaptureTag(String);

impl CaptureTag {
    /// Build a tag from any string-like value.
    #[must_use]
    pub fn new(tag: impl Into<String>) -> Self {
        Self(tag.into())
    }

    /// Borrow the tag text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug)]
struct KeybindShot;

/// Plugin: press F10 to write a PNG into the shot directory, named after this app's tag.
pub struct KeyboardCapturePlugin {
    tag: CaptureTag,
}

impl KeyboardCapturePlugin {
    /// Create the plugin with a filename tag (e.g. `"editor"`).
    #[must_use]
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            tag: CaptureTag::new(tag),
        }
    }
}

impl Plugin for KeyboardCapturePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<CapturePipelinePlugin<KeybindShot>>() {
            app.add_plugins(CapturePipelinePlugin::<KeybindShot>::new());
        }
        app.insert_resource(KeybindCaptureTag(self.tag.clone()))
            .add_systems(
                Update,
                (
                    capture_on_keypress.before(CaptureSystems),
                    report_keybind_captures.after(CaptureSystems),
                ),
            );
    }
}

#[derive(Resource, Clone, Debug, Deref)]
struct KeybindCaptureTag(CaptureTag);

fn capture_on_keypress(
    keys: Res<ButtonInput<KeyCode>>,
    tag: Res<KeybindCaptureTag>,
    mut queue: ResMut<CaptureQueue<KeybindShot>>,
) {
    if !keys.just_pressed(CAPTURE_KEY) {
        return;
    }
    info!("gdtf_screenshot: keybind capture requested");
    queue.push(Some(ShotStem::new(tag.as_str())), KeybindShot);
}

fn report_keybind_captures(mut completions: ResMut<CaptureCompletions<KeybindShot>>) {
    for completion in completions.drain() {
        let (outcome, _) = completion.into_parts();
        match outcome {
            CaptureOutcome::Landed(path) => {
                info!(path = %path.display(), "gdtf_screenshot: keybind capture written");
            }
            CaptureOutcome::Refused(detail) => {
                warn!(detail = %detail.as_str(), "gdtf_screenshot: keybind capture refused");
            }
            CaptureOutcome::TimedOut(path) => {
                warn!(path = %path.display(), "gdtf_screenshot: keybind capture timed out");
            }
        }
    }
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
