//! The shared stat-block updater (GTW-278 / GTW-274): mutates one stat block's widgets
//! from a ganger's CURRENT components, in place ([[ui-mutate-not-respawn]]).
//!
//! Both panels' update systems resolve their target ganger (the status panel from
//! `SelectedShooter`, the inspect panel from `InspectTarget`), read its components, and call
//! [`update_stat_block`] with the panel's [`StatBlockRefs`] handle + the shared
//! [`StatBlockWidgets`] query bundle. The updater writes the name/faction/stance text,
//! the TU/HP bar fills, the Wounds pips, the wound-name list (content + per-line + container
//! visibility), and the portrait atlas index — every write a mutate of the stored widget
//! entity, never a respawn.

use bevy::{
    ecs::{query::QueryData, system::SystemParam},
    prelude::*,
    ui::widget::ImageNode,
};
use gdtf_battle_sim::{
    Faction, GangerName, Hp, HpMax, InflictedWounds, Stance, Tu, TuMax, Wounds, WoundsMax,
};
use gdtf_ui::{FillFraction, Pip, ProgressBarFill, set_progress_bar};

use crate::scenes::running::game::battlescape::stat_block::{
    colors::{WOUNDS_LOST, WOUNDS_REMAINING},
    components::StatBlockRefs,
    labels::{faction_label, name_label, stance_label, wound_label},
    portrait::PortraitIndex,
};

/// The size of the pooled Wounds pip row a stat block pre-spawns.
///
/// The pip row is built ONCE at this size; each update shows the first `WoundsMax` pips
/// (filled per `Wounds`) and hides the rest, so the DISPLAYED pip count tracks the
/// ganger's `WoundsMax` (the contract's "`WoundsMax` total pips") without respawning the
/// row ([[ui-mutate-not-respawn]]). A ganger whose `WoundsMax` exceeds this cap shows the
/// first `MAX_WOUND_PIPS` pips (a generous display ceiling; gangers ship with a Wounds
/// pool of 3). A `const`, not a domain newtype — a pool size fed to a loop
/// (`.claude/rules/no-bare-types.md` clause 4).
pub(in crate::scenes::running::game::battlescape) const MAX_WOUND_PIPS: usize = 6;

/// A target ganger's stat-block components, read in ONE query tuple.
///
/// A [`QueryData`] struct (the wide-read precedent) so the read does not trip clippy
/// `type_complexity`. The display ceilings ([`HpMax`] / [`WoundsMax`]) and the
/// [`InflictedWounds`] list are read defensively as [`Option`] so a target missing one
/// still resolves (the panel renders what it can rather than failing the whole `.get`).
/// `pub(in …battlescape)` so it is at least as visible as the systems that name it.
#[derive(QueryData)]
pub(in crate::scenes::running::game::battlescape) struct StatBlockData {
    /// The ganger's name, if named.
    pub name:       Option<&'static GangerName>,
    /// The ganger's faction (gang) identity.
    pub faction:    &'static Faction,
    /// The ganger's stance posture.
    pub stance:     &'static Stance,
    /// The ganger's current TU pool.
    pub tu:         &'static Tu,
    /// The ganger's round-start TU ceiling.
    pub tu_max:     &'static TuMax,
    /// The ganger's current HP pool.
    pub hp:         &'static Hp,
    /// The ganger's HP display ceiling (GTW-291), if present.
    pub hp_max:     Option<&'static HpMax>,
    /// The ganger's current Wounds (life) pool.
    pub wounds:     &'static Wounds,
    /// The ganger's Wounds display ceiling (GTW-291), if present.
    pub wounds_max: Option<&'static WoundsMax>,
    /// The ganger's inflicted-wound list (GTW-279), if present.
    pub inflicted:  Option<&'static InflictedWounds>,
}

/// The shared widget-write queries a stat-block update touches, bundled as a
/// [`SystemParam`] so each panel's update system declares it as one param (the
/// `LineWriters` bundle precedent) and the wide query list stays out of the system
/// signature.
///
/// Every query reads/writes a DISTINCT component type, so they are mutually disjoint —
/// no [`ParamSet`] is needed (the name/faction/stance text AND the wound lines all flow
/// through the SAME `Query<&mut Text>`, addressed by stored [`Entity`], so there is one
/// `Text` writer, not several conflicting ones). `pub(in …battlescape)` so it is at least
/// as visible as the systems that name it (clippy `private_interfaces`).
#[derive(SystemParam)]
pub(in crate::scenes::running::game::battlescape) struct StatBlockWidgets<'w, 's> {
    /// Text writer for every text widget (name/faction/stance/wound lines), by id.
    pub texts:      Query<'w, 's, &'static mut Text>,
    /// Visibility writer for the wound-list container + its line pool.
    pub visibility: Query<'w, 's, &'static mut Visibility>,
    /// The portrait `ImageNode` writer (its atlas index is mutated).
    pub images:     Query<'w, 's, &'static mut ImageNode>,
    /// The wound-list container's children (its pooled line entities, in order).
    pub children:   Query<'w, 's, &'static Children>,
    /// The `ProgressBar` fill writer the `gdtf_ui` `set_progress_bar` helper mutates.
    pub fills:      Query<'w, 's, &'static mut Node, With<ProgressBarFill>>,
    /// The `Pips` color writer the `gdtf_ui` `set_pips` helper mutates.
    pub pips:       Query<'w, 's, &'static mut BackgroundColor, With<Pip>>,
}

/// Mutates the stat block referenced by `refs` from `data` — the ganger's current state.
///
/// Writes the name/faction/stance text, the TU/HP bar fills
/// ([`set_progress_bar`](gdtf_ui::set_progress_bar)), the Wounds pips (`WoundsMax` shown /
/// `Wounds` filled, [`update_pips`]), the portrait's atlas index
/// ([`PortraitIndex::for_name`]), and the wound-name list (the first N pooled lines set to
/// `"{tier} — {location}"` and shown, the rest hidden, the container shown only when
/// N ≥ 1). Every write is a mutate of the stored widget entity ([[ui-mutate-not-respawn]]).
/// Param-only (`bevy-traps.md` #7): the [`StatBlockWidgets`] query bundle, no `&mut World`.
pub(in crate::scenes::running::game::battlescape) fn update_stat_block(
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

    // TU / HP bars — current over the display ceiling (HP ceiling defaults to the current
    // HP when no HpMax is present, so a missing ceiling renders a full bar rather than a
    // div-by-zero empty one).
    set_progress_bar(
        refs.tu_bar,
        FillFraction::from_ratio(f32::from(**data.tu), f32::from(**data.tu_max)),
        &widgets.children,
        &mut widgets.fills,
    );
    // HP ceiling: HpMax when present, else the current HP (so a missing ceiling renders a
    // full bar rather than a div-by-zero empty one).
    let hp_max = data.hp_max.map_or(**data.hp, |m| **m);
    set_progress_bar(
        refs.hp_bar,
        FillFraction::from_ratio(f32::from(**data.hp), f32::from(hp_max)),
        &widgets.children,
        &mut widgets.fills,
    );

    // Wounds pips — show the first `WoundsMax` pips (filled per the remaining `Wounds`
    // pool, the rest empty), hide the surplus pool pips. WoundsMax defaults to the current
    // Wounds count when no ceiling is present, capped at the pooled row size.
    let wounds_max = data
        .wounds_max
        .map_or(usize::from(**data.wounds), |m| usize::from(**m))
        .min(MAX_WOUND_PIPS);
    let filled = usize::from(**data.wounds).min(wounds_max);
    update_pips(refs.wounds, wounds_max, filled, widgets);

    update_wound_list(refs.wound_list, data.inflicted, widgets);
    update_portrait(refs.portrait, data.name, &mut widgets.images);
}

/// Mutates the Wounds pip row in place: the first `total` pips are shown (the first
/// `filled` in the remaining color, the rest in the lost color) and the surplus pooled
/// pips are hidden — so the displayed pip count is `WoundsMax` and `Wounds` are filled,
/// without respawning the row ([[ui-mutate-not-respawn]]).
fn update_pips(row: Entity, total: usize, filled: usize, widgets: &mut StatBlockWidgets) {
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
fn write_text(texts: &mut Query<&mut Text>, entity: Entity, value: &str) {
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
fn update_wound_list(
    container: Entity,
    inflicted: Option<&InflictedWounds>,
    widgets: &mut StatBlockWidgets,
) {
    let empty: &[gdtf_battle_sim::InflictedWound] = &[];
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

/// Sets `entity`'s [`Visibility`] to `want`, only when it differs.
fn set_visible(visibility: &mut Query<&mut Visibility>, entity: Entity, want: Visibility) {
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
fn update_portrait(
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

/// The empty-state title shown when a panel has no target (no selection / nothing
/// hovered) — a clear empty state, written into the stat block's name line by
/// [`clear_stat_block`].
///
/// A `const` so the empty-state copy lives in one place (the status-panel `NO_SELECTION`
/// precedent — preserved verbatim so the existing empty-state assertion still reads).
pub(in crate::scenes::running::game::battlescape) const NO_TARGET: &str = "No ganger selected";

/// Resets the stat block referenced by `refs` to its EMPTY state — the contract's
/// no-target appearance (the status panel's "No ganger selected", the hover panel's
/// nothing-hovered): the name line shows [`NO_TARGET`], the faction/stance lines clear,
/// the TU/HP bars empty, all Wounds pips hide, and the wound-name list hides. Never stale
/// data ([[ui-mutate-not-respawn]] — every widget mutates, none respawn).
pub(in crate::scenes::running::game::battlescape) fn clear_stat_block(
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
    set_progress_bar(
        refs.hp_bar,
        FillFraction::new(0.0),
        &widgets.children,
        &mut widgets.fills,
    );
    // Hide every pip and the wound list.
    update_pips(refs.wounds, 0, 0, widgets);
    update_wound_list(refs.wound_list, None, widgets);
}
