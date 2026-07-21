//! The per-widget WRITE helpers the stat-block update mutates through — the pip row, the
//! text lines, the two name lists, the visibility flip, and the portrait.
//!
//! Split out of `update` (GTW-727) when the cursor-time read pushed that file past the
//! line band: `update` now holds WHAT the block shows (the query shape and the
//! drawn-over-live preference), and this file holds HOW each widget is mutated. The two
//! change for different reasons — a new displayed VALUE touches the first, a new widget
//! KIND touches the second.

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

/// Mutates the Wounds pip row in place: the first `total` pips are shown (the first
/// `filled` in the remaining color, the rest in the lost color) and the surplus pooled
/// pips are hidden — so the displayed pip count is `WoundsMax` and `Wounds` are filled,
/// without respawning the row ([[ui-mutate-not-respawn]]).
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

/// Writes `value` into the `Text` of `entity`, only when the content differs (so a
/// same-value frame does not spuriously trip `Changed<Text>`).
pub(super) fn write_text(texts: &mut Query<&mut Text>, entity: Entity, value: &str) {
    if let Ok(mut text) = texts.get_mut(entity)
        && text.as_str() != value
    {
        value.clone_into(&mut text.0);
    }
}

/// Mutates the wound-name list: shows the first N pooled lines (one per inflicted wound)
/// with their `"{tier} — {location}"` content, hides the rest, and shows the container
/// only when N ≥ 1 — all in place ([[ui-mutate-not-respawn]]).
///
/// A `None`/absent [`InflictedWounds`] is treated as an empty list (container hidden).
/// More wounds than the pooled line count render the first pool's worth (the
/// `WOUND_LINE_POOL` display cap).
pub(super) fn update_wound_list(
    container: Entity,
    inflicted: Option<&InflictedWounds>,
    widgets: &mut StatBlockWidgets,
) {
    let empty: &[gdtf_battle_sim::inflicted_wound::InflictedWound] = &[];
    let wounds = inflicted.map_or(empty, |w| w);

    // Show/hide the container.
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

    // Collect the pooled line entities in order, then set the first N + hide the rest.
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

/// Mutates the injury-name list (GTW-439): shows the first N pooled lines (one per inflicted
/// injury) with their authored
/// [`inspect_text`](gdtf_battle_sim::injuries::GainedInjury::inspect_text) content, hides the rest, and
/// shows the container only when N ≥ 1 — all in place ([[ui-mutate-not-respawn]]).
///
/// Driven by the DURABLE [`InflictedInjuries`](gdtf_battle_sim::injuries::InflictedInjuries) ledger
/// (read through [`gained`](gdtf_battle_sim::injuries::InflictedInjuries::gained)), so the list PERSISTS
/// while the ganger is inspected/selected — distinct from the transient FCT flash the
/// `InjuryInflicted` message drives (the message routes the one-shot pop; the ledger routes
/// this list). A `None`/absent ledger is treated as an empty list (container hidden). More
/// injuries than the pooled line count render the first pool's worth (the `INJURY_LINE_POOL`
/// display cap), the wound-list precedent.
pub(super) fn update_injury_list(
    container: Entity,
    injuries: Option<&InflictedInjuries>,
    widgets: &mut StatBlockWidgets,
) {
    let empty: &[gdtf_battle_sim::injuries::GainedInjury] = &[];
    let gained = injuries.map_or(empty, InflictedInjuries::gained);

    // Show/hide the container.
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

    // Collect the pooled line entities in order, then set the first N + hide the rest.
    let Ok(lines) = widgets.children.get(container) else {
        return;
    };
    let line_ids: Vec<Entity> = lines.iter().collect();
    for (index, &line) in line_ids.iter().enumerate() {
        match gained.get(index) {
            Some(injury) => {
                // The authored, durable inspect text (e.g. "Lost Eye -- -2 Aim, -1 Cool") —
                // rendered verbatim (no presenter re-formatting; the sim authored it).
                write_text(&mut widgets.texts, line, &injury.inspect_text);
                set_visible(&mut widgets.visibility, line, Visibility::Inherited);
            }
            None => {
                set_visible(&mut widgets.visibility, line, Visibility::Hidden);
            }
        }
    }
}

/// Sets `entity`'s [`Visibility`] to `want`, only when it differs.
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

/// Mutates the portrait node's atlas index to the deterministic face for `name`.
///
/// Reads the [`PortraitIndex::for_name`] derivation and writes the
/// [`TextureAtlas::index`](bevy::image::TextureAtlas) of the portrait's
/// [`ImageNode`](bevy::ui::widget::ImageNode)
/// in place — only when the index differs, so a steady selection does not churn the node.
/// A node spawned WITHOUT an atlas (the sheet was not loaded at spawn) is left untouched.
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
