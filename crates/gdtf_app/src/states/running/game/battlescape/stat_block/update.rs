use bevy::{
    ecs::{query::QueryData, system::SystemParam},
    prelude::*,
    ui::widget::ImageNode,
};
use gdtf_battle_presenter::DrawnVitals;
use gdtf_battle_sim::{
    ganger::{GangerName, Hp, HpMax, TuMax, Wounds, WoundsMax},
    inflicted_wound::InflictedWounds,
    injuries::InflictedInjuries,
    prelude::{Faction, Stance, Tu},
};
use gdtf_ui::{FillFraction, Pip, ProgressBarFill, set_progress_bar};

use crate::states::running::game::battlescape::stat_block::{
    components::StatBlockRefs,
    labels::{faction_label, name_label, stance_label},
    writers::{update_injury_list, update_pips, update_portrait, update_wound_list, write_text},
};

pub(in crate::states::running::game::battlescape) const MAX_WOUND_PIPS: usize = 6;

#[derive(QueryData)]
pub(in crate::states::running::game::battlescape) struct StatBlockData {
    pub name:       Option<&'static GangerName>,
    pub faction:    &'static Faction,
    pub stance:     &'static Stance,
    pub tu:         &'static Tu,
    pub tu_max:     &'static TuMax,
    pub hp:         &'static Hp,
    pub hp_max:     Option<&'static HpMax>,
    pub wounds:     &'static Wounds,
    pub wounds_max: Option<&'static WoundsMax>,
    pub inflicted:  Option<&'static InflictedWounds>,
    pub injuries:   Option<&'static InflictedInjuries>,
    pub drawn:      Option<&'static DrawnVitals>,
}

#[derive(SystemParam)]
pub(in crate::states::running::game::battlescape) struct StatBlockWidgets<'w, 's> {
    pub texts:      Query<'w, 's, &'static mut Text>,
    pub visibility: Query<'w, 's, &'static mut Visibility>,
    pub images:     Query<'w, 's, &'static mut ImageNode>,
    pub children:   Query<'w, 's, &'static Children>,
    pub fills:      Query<'w, 's, &'static mut Node, With<ProgressBarFill>>,
    pub pips:       Query<'w, 's, &'static mut BackgroundColor, With<Pip>>,
}

pub(in crate::states::running::game::battlescape) fn update_stat_block(
    refs: StatBlockRefs,
    data: &StatBlockDataItem,
    widgets: &mut StatBlockWidgets,
) {
    write_text(&mut widgets.texts, refs.name, &name_label(data.name));
    write_text(
        &mut widgets.texts,
        refs.faction,
        &faction_label(*data.faction),
    );
    write_text(&mut widgets.texts, refs.stance, &stance_label(*data.stance));

    let tu = data.drawn.map_or(*data.tu, DrawnVitals::tu);
    let hp = data.drawn.map_or(*data.hp, DrawnVitals::hp);
    let wounds = data.drawn.map_or(*data.wounds, DrawnVitals::wounds);
    let inflicted = data
        .drawn
        .map_or(data.inflicted, |drawn| Some(drawn.inflicted()));
    let injuries = data
        .drawn
        .map_or(data.injuries, |drawn| Some(drawn.injuries()));

    set_progress_bar(
        refs.tu_bar,
        FillFraction::from_ratio(f32::from(*tu), f32::from(**data.tu_max)),
        &widgets.children,
        &mut widgets.fills,
    );
    write_text(
        &mut widgets.texts,
        refs.tu_label,
        &format!("{}/{}", *tu, **data.tu_max),
    );
    let hp_max = data.hp_max.map_or(*hp, |m| **m);
    set_progress_bar(
        refs.hp_bar,
        FillFraction::from_ratio(f32::from(*hp), f32::from(hp_max)),
        &widgets.children,
        &mut widgets.fills,
    );
    write_text(
        &mut widgets.texts,
        refs.hp_label,
        &format!("{}/{}", *hp, hp_max),
    );

    let wounds_max = data
        .wounds_max
        .map_or(usize::from(*wounds), |m| usize::from(**m))
        .min(MAX_WOUND_PIPS);
    let filled = usize::from(*wounds).min(wounds_max);
    update_pips(refs.wounds, wounds_max, filled, widgets);

    update_wound_list(refs.wound_list, inflicted, widgets);
    update_injury_list(refs.injury_list, injuries, widgets);
    update_portrait(refs.portrait, data.name, &mut widgets.images);
}

pub(in crate::states::running::game::battlescape) const NO_TARGET: &str = "No ganger selected";

pub(in crate::states::running::game::battlescape) fn clear_stat_block(
    refs: StatBlockRefs,
    widgets: &mut StatBlockWidgets,
) {
    write_text(&mut widgets.texts, refs.name, NO_TARGET);
    write_text(&mut widgets.texts, refs.faction, "");
    write_text(&mut widgets.texts, refs.stance, "");
    set_progress_bar(
        refs.tu_bar,
        FillFraction::new(0.0),
        &widgets.children,
        &mut widgets.fills,
    );
    write_text(&mut widgets.texts, refs.tu_label, "");
    set_progress_bar(
        refs.hp_bar,
        FillFraction::new(0.0),
        &widgets.children,
        &mut widgets.fills,
    );
    write_text(&mut widgets.texts, refs.hp_label, "");
    update_pips(refs.wounds, 0, 0, widgets);
    update_wound_list(refs.wound_list, None, widgets);
    update_injury_list(refs.injury_list, None, widgets);
}
