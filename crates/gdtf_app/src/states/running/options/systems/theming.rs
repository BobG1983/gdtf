use bevy::{
    prelude::*,
    ui::{BackgroundColor, BorderColor as UiBorderColor},
};
use gdtf_ui::theme::GdtfTheme;

#[cfg(feature = "dev_tools")]
use crate::states::running::options::settings::ProcgenStepperEnabled;
use crate::states::running::options::{
    components::{SoundToggle, SoundToggleKnob},
    settings::{GameSettings, SoundEnabled},
};

pub(in crate::states::running::options) trait ToggleValue:
    Copy
{
        fn is_on(self) -> bool;
}

impl ToggleValue for SoundEnabled {
    fn is_on(self) -> bool {
        Self::is_on(self)
    }
}

#[cfg(feature = "dev_tools")]
impl ToggleValue for ProcgenStepperEnabled {
    fn is_on(self) -> bool {
        Self::is_on(self)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::states::running::options) struct ToggleColors {
        off:    Color,
        on:     Color,
        knob:   Color,
            border: Color,
}

impl ToggleColors {
        pub(in crate::states::running::options) fn track<V: ToggleValue>(&self, value: V) -> Color {
        if value.is_on() { self.on } else { self.off }
    }

        pub(in crate::states::running::options) const fn knob(&self) -> Color {
        self.knob
    }

        pub(in crate::states::running::options) const fn border(&self) -> Color {
        self.border
    }
}

pub(in crate::states::running::options) fn toggle_colors(theme: &GdtfTheme) -> ToggleColors {
    ToggleColors {
        off:    *theme.button.disabled,
        on:     *theme.button.active,
        knob:   *theme.button.text_color,
        border: *theme.button.border_color,
    }
}

pub(in crate::states::running::options) fn knob_justify<V: ToggleValue>(
    value: V,
) -> JustifyContent {
    if value.is_on() {
        JustifyContent::FlexEnd
    } else {
        JustifyContent::FlexStart
    }
}

pub(in crate::states::running::options) type ToggleVisuals = (
    &'static mut BackgroundColor,
    &'static mut UiBorderColor,
    &'static mut Node,
    &'static Children,
);

pub(in crate::states::running::options) fn repaint_toggle<
    V: ToggleValue,
    T: Component,
    K: Component,
>(
    colors: ToggleColors,
    value: V,
    toggles: &mut Query<ToggleVisuals, With<T>>,
    knobs: &mut Query<&mut BackgroundColor, (With<K>, Without<T>)>,
) {
    for (mut track, mut border, mut node, children) in toggles {
        track.0 = colors.track(value);
        *border = UiBorderColor::all(colors.border());
        node.justify_content = knob_justify(value);
        for child in children {
            if let Ok(mut knob) = knobs.get_mut(*child) {
                knob.0 = colors.knob();
            }
        }
    }
}

pub(in crate::states::running::options) fn paint_sound_toggle(
    theme: Option<Res<GdtfTheme>>,
    settings: Res<GameSettings>,
    mut toggles: Query<ToggleVisuals, With<SoundToggle>>,
    mut knobs: Query<&mut BackgroundColor, (With<SoundToggleKnob>, Without<SoundToggle>)>,
) {
    let Some(theme) = theme else {
        return;
    };
    let colors = toggle_colors(&theme);
    repaint_toggle(colors, settings.sound, &mut toggles, &mut knobs);
}
