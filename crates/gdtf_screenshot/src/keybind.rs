use bevy::{
    prelude::*,
    render::view::window::screenshot::{Screenshot, save_to_disk},
};

use crate::path::timestamped_path;

const CAPTURE_KEY: KeyCode = KeyCode::F10;

#[derive(Clone, Debug)]
pub struct CaptureTag(String);

impl CaptureTag {
        #[must_use]
    pub fn new(tag: impl Into<String>) -> Self {
        Self(tag.into())
    }

        #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub struct KeyboardCapturePlugin {
        tag: CaptureTag,
}

impl KeyboardCapturePlugin {
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

#[derive(Resource, Clone, Debug, Deref)]
struct KeybindCaptureTag(CaptureTag);

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
