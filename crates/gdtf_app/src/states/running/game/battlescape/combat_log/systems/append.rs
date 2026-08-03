use bevy::{
    color::Alpha,
    prelude::*,
    scene::{CommandsSceneExt, bsn},
    text::{FontSize, FontWeight, LineHeight, TextFont},
    ui::{ComputedNode, Node, PositionType, Val},
};
use gdtf_battle_presenter::{CombatLogEvent, FctEmphasis, LogLine, classify_log_event};
use gdtf_ui::theme::GdtfTheme;

use crate::states::running::game::battlescape::combat_log::{
    components::{CombatLogLine, CombatLogRoot, LineSlide, LogLineFade},
    tuning::CombatLogTuning,
};

pub(in crate::states::running::game::battlescape) fn append_combat_log(
    mut commands: Commands,
    mut events: MessageReader<CombatLogEvent>,
    root: Query<Entity, With<CombatLogRoot>>,
    mut lines: Query<(Entity, &ComputedNode, &mut LineSlide), With<CombatLogLine>>,
    theme: Option<Res<GdtfTheme>>,
    tuning: Option<Res<CombatLogTuning>>,
) {
    let (Ok(root), Some(theme)) = (root.single(), theme) else {
        events.clear();
        return;
    };
    let tuning = tuning.map_or_else(CombatLogTuning::default, |t| *t);

    let mut appended: Vec<Entity> = Vec::new();
    for event in events.read() {
        for line in classify_log_event(event) {
            let entity = spawn_log_line(&mut commands, &theme, &tuning, &line);
            commands.entity(root).add_children(&[entity]);
            appended.push(entity);
        }
    }

    trim_to_cap(
        &mut commands,
        &mut lines,
        *tuning.max_visible_lines,
        &appended,
    );
}

fn trim_to_cap(
    commands: &mut Commands,
    lines: &mut Query<(Entity, &ComputedNode, &mut LineSlide), With<CombatLogLine>>,
    cap: usize,
    appended: &[Entity],
) {
    if cap == 0 {
        return;
    }
    let mut chronological: Vec<Entity> = lines.iter().map(|(e, ..)| e).collect();
    chronological.sort_unstable();
    chronological.extend_from_slice(appended);

    let overflow = chronological.len().saturating_sub(cap);
    if overflow == 0 {
        return;
    }
    let removed: Vec<Entity> = chronological.into_iter().take(overflow).collect();

    let mut freed_height = 0.0_f32;
    for &entity in &removed {
        if let Ok((_, node, _)) = lines.get(entity) {
            freed_height = node
                .size()
                .y
                .mul_add(node.inverse_scale_factor(), freed_height);
        }
    }

    if freed_height > f32::EPSILON {
        for (entity, _, mut slide) in lines.iter_mut() {
            if !removed.contains(&entity) {
                slide.displace(freed_height);
            }
        }
    }

    for entity in removed {
        commands.entity(entity).despawn();
    }
}

const APPEAR_OFFSET_LINE_HEIGHTS: f32 = 1.3;

const LINE_HEIGHT_SCALE: f32 = 1.4;

fn spawn_log_line(
    commands: &mut Commands,
    theme: &GdtfTheme,
    tuning: &CombatLogTuning,
    line: &LogLine,
) -> Entity {
    let text = (**line.text()).clone();
    let mut color = line.color();
    let base_alpha = color.alpha();
    let (weight, font_size) = emphasis_style(line.emphasis(), *tuning.line_font_pt);
    let font = theme.text.font.clone();
    let text_font = TextFont {
        font: font.into(),
        font_size: FontSize::Px(font_size),
        weight,
        ..default()
    };
    let line_box_px = font_size * LINE_HEIGHT_SCALE;
    let fade = LogLineFade::new(
        tuning.line_ttl_seconds,
        tuning.fade_in_seconds,
        tuning.fade_out_seconds,
        base_alpha,
    );
    let slide = LineSlide::new(font_size * APPEAR_OFFSET_LINE_HEIGHTS);
    color.set_alpha(0.0);
    commands
        .spawn_scene(bsn! {
            Text::new(text)
            TextColor(color)
            template(move |_| Ok(text_font.clone()))
        })
        .insert((
            CombatLogLine,
            fade,
            slide,
            LineHeight::Px(line_box_px),
            Node {
                position_type: PositionType::Relative,
                min_height: Val::Px(line_box_px),
                top: Val::Px(font_size * APPEAR_OFFSET_LINE_HEIGHTS),
                ..default()
            },
        ))
        .id()
}

fn emphasis_style(emphasis: FctEmphasis, base_pt: f32) -> (FontWeight, f32) {
    let font_size = match emphasis {
        FctEmphasis::Bold => base_pt * BOLD_FONT_SCALE,
        FctEmphasis::Normal => base_pt,
    };
    (emphasis.weight(), font_size)
}

const BOLD_FONT_SCALE: f32 = 1.25;
