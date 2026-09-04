use bevy::prelude::*;
use gdtf_battle_sim::{
    ganger::GangerName, inflicted_wound::InflictedWounds, injuries::InflictedInjuries,
};

use super::{
    colors::{WOUNDS_LOST, WOUNDS_REMAINING},
    labels::wound_label,
    portrait::PortraitIndex,
    update::StatBlockWidgets,
};

pub(super) fn update_pips(
    row: Entity,
    total: usize,
    filled: usize,
    widgets: &mut StatBlockWidgets,
) {
    let Ok(pip_children) = widgets.children.get(row) else {
        return;
    };
    let pip_ids: Vec<Entity> = pip_children.iter().collect();
    for (index, &pip) in pip_ids.iter().enumerate() {
        if index < total {
            set_visible(&mut widgets.visibility, pip, Visibility::Inherited);
            let want = if index < filled {
                WOUNDS_REMAINING
            } else {
                WOUNDS_LOST
            };
            if let Ok(mut background) = widgets.pips.get_mut(pip)
                && background.0 != want
            {
                background.0 = want;
            }
        } else {
            set_visible(&mut widgets.visibility, pip, Visibility::Hidden);
        }
    }
}

pub(super) fn write_text(texts: &mut Query<&mut Text>, entity: Entity, value: &str) {
    if let Ok(mut text) = texts.get_mut(entity)
        && text.as_str() != value
    {
        value.clone_into(&mut text.0);
    }
}

pub(super) fn update_wound_list(
    container: Entity,
    inflicted: Option<&InflictedWounds>,
    widgets: &mut StatBlockWidgets,
) {
    let empty: &[gdtf_battle_sim::inflicted_wound::InflictedWound] = &[];
    let wounds = inflicted.map_or(empty, |w| w);

    let want = if wounds.is_empty() {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    if let Ok(mut vis) = widgets.visibility.get_mut(container)
        && *vis != want
    {
        *vis = want;
    }

    let Ok(lines) = widgets.children.get(container) else {
        return;
    };
    let line_ids: Vec<Entity> = lines.iter().collect();
    for (index, &line) in line_ids.iter().enumerate() {
        match wounds.get(index) {
            Some(&wound) => {
                let label = wound_label(wound);
                write_text(&mut widgets.texts, line, &label);
                set_visible(&mut widgets.visibility, line, Visibility::Inherited);
            }
            None => {
                set_visible(&mut widgets.visibility, line, Visibility::Hidden);
            }
        }
    }
}

pub(super) fn update_injury_list(
    container: Entity,
    injuries: Option<&InflictedInjuries>,
    widgets: &mut StatBlockWidgets,
) {
    let empty: &[gdtf_battle_sim::injuries::GainedInjury] = &[];
    let gained = injuries.map_or(empty, InflictedInjuries::gained);

    let want = if gained.is_empty() {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    if let Ok(mut vis) = widgets.visibility.get_mut(container)
        && *vis != want
    {
        *vis = want;
    }

    let Ok(lines) = widgets.children.get(container) else {
        return;
    };
    let line_ids: Vec<Entity> = lines.iter().collect();
    for (index, &line) in line_ids.iter().enumerate() {
        match gained.get(index) {
            Some(injury) => {
                write_text(&mut widgets.texts, line, &injury.inspect_text);
                set_visible(&mut widgets.visibility, line, Visibility::Inherited);
            }
            None => {
                set_visible(&mut widgets.visibility, line, Visibility::Hidden);
            }
        }
    }
}

pub(super) fn set_visible(
    visibility: &mut Query<&mut Visibility>,
    entity: Entity,
    want: Visibility,
) {
    if let Ok(mut vis) = visibility.get_mut(entity)
        && *vis != want
    {
        *vis = want;
    }
}

pub(super) fn update_portrait(
    portrait: Entity,
    name: Option<&GangerName>,
    images: &mut Query<&mut ImageNode>,
) {
    let index = PortraitIndex::for_name(name);
    if let Ok(mut image) = images.get_mut(portrait)
        && let Some(atlas) = image.texture_atlas.as_mut()
        && atlas.index != *index
    {
        atlas.index = *index;
    }
}
