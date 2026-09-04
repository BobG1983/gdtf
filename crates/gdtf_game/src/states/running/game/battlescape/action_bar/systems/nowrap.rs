//! re-corrected the next frame. NOTE the Aim control is now a knob-only `Switch` (no text label),
use bevy::{
    prelude::*,
    text::{FontSize, LineBreak, TextFont},
    ui::{Node, Val},
};

use crate::states::running::game::battlescape::action_bar::components::{
    ModeBurstButton, ModeFullButton, ModePanelRoot, ModeSingleButton, StanceKneelingButton,
    StanceProneButton, StanceStandingButton,
};

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
struct ControlLabelPt(f32);

const CONTROL_LABEL_PT: ControlLabelPt = ControlLabelPt(14.0);

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
struct ControlPadVw(f32);

const MODE_PANEL_PAD_VW: ControlPadVw = ControlPadVw(0.15625);

type RelocatedControlSegment = (
    Or<(
        With<ModeSingleButton>,
        With<ModeBurstButton>,
        With<ModeFullButton>,
        With<StanceStandingButton>,
        With<StanceKneelingButton>,
        With<StanceProneButton>,
    )>,
    Without<ModePanelRoot>,
);

pub(in crate::states::running::game::battlescape) fn nowrap_control_labels(
    segments: Query<&Children, RelocatedControlSegment>,
    mut mode_panels: Query<&mut Node, With<ModePanelRoot>>,
    mut labels: Query<(&mut TextLayout, &mut TextFont), With<Text>>,
) {
    for mut node in &mut mode_panels {
        if node.padding.left != Val::Vw(*MODE_PANEL_PAD_VW) {
            node.padding.left = Val::Vw(*MODE_PANEL_PAD_VW);
        }
        if node.padding.right != Val::Vw(*MODE_PANEL_PAD_VW) {
            node.padding.right = Val::Vw(*MODE_PANEL_PAD_VW);
        }
    }
    for children in &segments {
        for &child in children {
            let Ok((mut layout, mut font)) = labels.get_mut(child) else {
                continue;
            };
            if layout.linebreak != LineBreak::NoWrap {
                layout.linebreak = LineBreak::NoWrap;
            }
            if font.font_size != FontSize::Px(*CONTROL_LABEL_PT) {
                font.font_size = FontSize::Px(*CONTROL_LABEL_PT);
            }
        }
    }
}
