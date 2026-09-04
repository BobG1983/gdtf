use bevy::{
    color::Alpha,
    prelude::*,
    text::TextColor as UiTextColor,
    ui::{ComputedNode, Node, Val},
};

use crate::states::running::game::battlescape::combat_log::{
    components::{CombatLogLine, CombatLogRoot, LineSlide, LogLineFade, PanelHeightAnim},
    tuning::CombatLogTuning,
};

pub(in crate::states::running::game::battlescape) fn fade_combat_log_lines(
    mut commands: Commands,
    time: Res<Time>,
    mut lines: Query<(Entity, &mut UiTextColor, &mut LogLineFade)>,
) {
    let delta = time.delta();
    for (entity, mut color, mut fade) in &mut lines {
        if fade.advance(delta) {
            commands.entity(entity).despawn();
            continue;
        }
        color.0.set_alpha(fade.alpha());
    }
}

const MAX_LERP_RATE: f32 = 1_000.0;

fn lerp_factor(rate: f32, delta: f32) -> f32 {
    let rate = rate.clamp(0.0, MAX_LERP_RATE);
    (-rate * delta.max(0.0))
        .exp()
        .mul_add(-1.0, 1.0)
        .clamp(0.0, 1.0)
}

pub(in crate::states::running::game::battlescape) fn slide_combat_log_lines(
    time: Res<Time>,
    tuning: Option<Res<CombatLogTuning>>,
    mut lines: Query<(&mut Node, &mut LineSlide), With<CombatLogLine>>,
) {
    let tuning = tuning.map_or_else(CombatLogTuning::default, |t| *t);
    let factor = lerp_factor(*tuning.line_lerp_rate, time.delta_secs());
    for (mut node, mut slide) in &mut lines {
        slide.ease_toward_target(factor);
        node.top = Val::Px(slide.current());
    }
}

pub(in crate::states::running::game::battlescape) fn animate_combat_log_height(
    time: Res<Time>,
    tuning: Option<Res<CombatLogTuning>>,
    lines: Query<(&ComputedNode, &LineSlide), With<CombatLogLine>>,
    mut roots: Query<(&mut Node, &mut PanelHeightAnim), With<CombatLogRoot>>,
) {
    let Ok((mut node, mut anim)) = roots.single_mut() else {
        return;
    };
    let tuning = tuning.map_or_else(CombatLogTuning::default, |t| *t);
    let content: f32 = lines
        .iter()
        .map(|(computed, _)| computed.size().y * computed.inverse_scale_factor())
        .sum();
    let max_slide: f32 = lines
        .iter()
        .map(|(_, slide)| slide.current().max(0.0))
        .fold(0.0_f32, f32::max);
    let clearance = *tuning.line_font_pt * *tuning.bottom_clearance_lines;
    let target = height_target(content, max_slide, clearance);

    let factor = lerp_factor(*tuning.height_lerp_rate, time.delta_secs());
    anim.ease_toward(target, factor);
    node.height = Val::Px(anim.current());
}

fn height_target(content: f32, max_slide: f32, clearance: f32) -> f32 {
    if content <= f32::EPSILON {
        return 0.0;
    }
    content + max_slide + clearance
}

#[cfg(test)]
mod test {
    use super::height_target;

    fn approx(actual: f32, expected: f32) -> bool {
        (actual - expected).abs() < 1e-4
    }

    #[test]
    fn the_target_reserves_bottom_clearance_above_the_content_sum() {
        let content = 80.0;
        let clearance = 10.0;
        let target = height_target(content, 0.0, clearance);
        assert!(
            target > content,
            "with a positive clearance the panel must be taller than its content ({target} \
             must exceed {content}) so the bottommost line is not clipped",
        );
        assert!(
            approx(target, content + clearance),
            "the settled target is exactly content + clearance, got {target}",
        );
    }

    #[test]
    fn the_clearance_stacks_with_an_active_slide() {
        let target = height_target(80.0, 6.0, 10.0);
        assert!(
            approx(target, 96.0),
            "content + slide + clearance are all reserved (80 + 6 + 10), got {target}",
        );
    }

    #[test]
    fn an_empty_log_collapses_to_zero_with_no_clearance_sliver() {
        let target = height_target(0.0, 0.0, 10.0);
        assert!(
            approx(target, 0.0),
            "an empty log targets 0 height (no clearance sliver), got {target}",
        );
    }
}
