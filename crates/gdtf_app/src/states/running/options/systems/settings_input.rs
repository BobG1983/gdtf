use bevy::{prelude::*, ui_widgets::ValueChange};

use crate::states::running::options::{
    components::{SoundToggle, SoundValueLabel},
    settings::{GameSettings, SoundEnabled, SoundSettingChanged, sound_value_text},
};

pub(in crate::states::running::options) fn sound_activated(
    change: On<ValueChange<bool>>,
    toggles: Query<(), With<SoundToggle>>,
    mut changed: MessageWriter<SoundSettingChanged>,
) {
    if toggles.contains(change.source) {
        changed.write(SoundSettingChanged::new(SoundEnabled::new(change.value)));
    }
}

pub(in crate::states::running::options) fn apply_sound_setting(
    mut changed: MessageReader<SoundSettingChanged>,
    mut settings: ResMut<GameSettings>,
) {
    for change in changed.read() {
        settings.sound = **change;
    }
}

pub(in crate::states::running::options) fn sync_sound_value_label(
    settings: Res<GameSettings>,
    mut labels: Query<&mut Text, With<SoundValueLabel>>,
) {
    if !settings.is_changed() {
        return;
    }
    let text = sound_value_text(settings.sound);
    for mut label in &mut labels {
        *label = Text::new(text);
    }
}
