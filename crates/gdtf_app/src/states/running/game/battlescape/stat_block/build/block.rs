use bevy::{
    ecs::template::template,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    text::{FontSize, TextColor as UiTextColor, TextFont},
    ui::{Node, Val},
};
use gdtf_battle_presenter::TopDownAtlases;
use gdtf_ui::{
    FillFraction, FilledPips, spawn_pips, spawn_progress_bar,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use super::{
    bars::{spawn_bar_group, spawn_bar_label},
    lists::{spawn_injury_list, spawn_wound_list},
};
use crate::states::running::game::battlescape::stat_block::{
    colors::{HP_LOST, HP_REMAINING, TU_LOST, TU_REMAINING, WOUNDS_LOST, WOUNDS_REMAINING},
    components::{
        StatBlockRefs, StatFaction, StatHpBar, StatHpLabel, StatName, StatStance, StatTuBar,
        StatTuLabel, StatWoundsPips,
    },
    portrait::spawn_portrait,
    update::MAX_WOUND_PIPS,
};

pub(super) const ROW_GAP_VH: f32 = 0.55556;

pub(in crate::states::running::game::battlescape) fn spawn_stat_block(
    commands: &mut Commands,
    theme: &GdtfTheme,
    atlases: Option<&TopDownAtlases>,
) -> Entity {
    let portrait = spawn_portrait(commands, atlases);
    let name = spawn_text(commands, theme, StatName, "");
    let faction = spawn_text(commands, theme, StatFaction, "");
    let stance = spawn_text(commands, theme, StatStance, "");
    let tu_bar = spawn_progress_bar(
        commands,
        FillFraction::new(0.0),
        TU_REMAINING,
        TU_LOST,
        StatTuBar,
    );
    let tu_label = spawn_bar_label(commands, theme, StatTuLabel);
    let tu_group = spawn_bar_group(commands, tu_label, tu_bar);
    let hp_bar = spawn_progress_bar(
        commands,
        FillFraction::new(0.0),
        HP_REMAINING,
        HP_LOST,
        StatHpBar,
    );
    let hp_label = spawn_bar_label(commands, theme, StatHpLabel);
    let hp_group = spawn_bar_group(commands, hp_label, hp_bar);
    let wounds = spawn_pips(
        commands,
        MAX_WOUND_PIPS,
        FilledPips::new(0),
        WOUNDS_REMAINING,
        WOUNDS_LOST,
        StatWoundsPips,
    );
    let wound_list = spawn_wound_list(commands, theme);
    let injury_list = spawn_injury_list(commands, theme);

    let root_node = Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Vh(ROW_GAP_VH),
        ..default()
    };
    let root = commands
        .spawn_scene(template_value(root_node))
        .insert(StatBlockRefs {
            portrait,
            name,
            faction,
            stance,
            tu_bar,
            tu_label,
            hp_bar,
            hp_label,
            wounds,
            wound_list,
            injury_list,
        })
        .id();
    commands.entity(root).add_children(&[
        portrait,
        name,
        faction,
        stance,
        tu_group,
        hp_group,
        wounds,
        wound_list,
        injury_list,
    ]);
    root
}

fn spawn_text(
    commands: &mut Commands,
    theme: &GdtfTheme,
    marker: impl Bundle,
    initial: &str,
) -> Entity {
    let text_color = *theme.text.text_color;
    let caption = initial.to_owned();
    let text_font = TextFont {
        font: theme.text.font.clone().into(),
        font_size: FontSize::Px(*theme.text.font_size_pt),
        ..default()
    };
    commands
        .spawn_scene(bsn! {
            Themed::new(ThemeRole::Text)
            Text::new(caption)
            UiTextColor(text_color)
            template(move |_| Ok(text_font.clone()))
        })
        .insert(marker)
        .id()
}
