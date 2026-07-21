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

/// The size of the pooled Wounds pip row a stat block pre-spawns.
///
/// The pip row is built ONCE at this size; each update shows the first `WoundsMax` pips
/// (filled per `Wounds`) and hides the rest, so the DISPLAYED pip count tracks the
/// ganger's `WoundsMax` (the contract's "`WoundsMax` total pips") without respawning the
/// row ([[ui-mutate-not-respawn]]). A ganger whose `WoundsMax` exceeds this cap shows the
/// first `MAX_WOUND_PIPS` pips (a generous display ceiling; gangers ship with a Wounds
/// pool of 3). A `const`, not a domain newtype — a pool size fed to a loop
/// (`.claude/rules/no-bare-types.md` clause 4).
pub(in crate::states::running::game::battlescape) const MAX_WOUND_PIPS: usize = 6;

/// A target ganger's stat-block components, read in ONE query tuple.
///
/// A [`QueryData`] struct (the wide-read precedent) so the read does not trip clippy
/// `type_complexity`. The display ceilings ([`HpMax`] / [`WoundsMax`]) and the
/// [`InflictedWounds`] list are read defensively as [`Option`] so a target missing one
/// still resolves (the panel renders what it can rather than failing the whole `.get`).
/// `pub(in …battlescape)` so it is at least as visible as the systems that name it.
#[derive(QueryData)]
pub(in crate::states::running::game::battlescape) struct StatBlockData {
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
    /// The ganger's inflicted-injury ledger (GTW-439), if present — the DURABLE source of
    /// the persistent injury-name list (read defensively as [`Option`] so a target missing
    /// it still renders the rest of the block).
    pub injuries:   Option<&'static InflictedInjuries>,
    /// The CURSOR-TIME vitals the presenter is currently SHOWING (GTW-727 C26), if the
    /// playback mirrors are present.
    ///
    /// Every numeric field below prefers this over the live one. That is the whole of
    /// clause (b) for the stat block: without it the panel reads live `Hp` / `Wounds` /
    /// `InflictedInjuries`, so a shot's damage and its injury entry appear the frame the
    /// sim resolves them — while the bolt that caused them is still in flight. `Option`
    /// because the mirrors exist only where the top-down presenter does; with no
    /// presenter (or once caught up) the read is identical to what it always was.
    pub drawn:      Option<&'static DrawnVitals>,
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
pub(in crate::states::running::game::battlescape) struct StatBlockWidgets<'w, 's> {
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
/// N ≥ 1), AND the injury-name list (GTW-439 — the first N pooled lines set to each inflicted
/// injury's authored `inspect_text`, driven by the durable `InflictedInjuries` ledger). Every
/// write is a mutate of the stored widget entity ([[ui-mutate-not-respawn]]).
/// Param-only (`bevy-traps.md` #7): the [`StatBlockWidgets`] query bundle, no `&mut World`.
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

    // GTW-727 C26: every pool below is read at CURSOR TIME when the playback mirror is
    // present, falling back to the live component when it is not. `unwrap_or` per field
    // rather than a whole-block branch, so a partially-mirrored entity still renders.
    let tu = data.drawn.map_or(*data.tu, DrawnVitals::tu);
    let hp = data.drawn.map_or(*data.hp, DrawnVitals::hp);
    let wounds = data.drawn.map_or(*data.wounds, DrawnVitals::wounds);
    let inflicted = data
        .drawn
        .map_or(data.inflicted, |drawn| Some(drawn.inflicted()));
    let injuries = data
        .drawn
        .map_or(data.injuries, |drawn| Some(drawn.injuries()));

    // TU / HP bars — current over the display ceiling (HP ceiling defaults to the current
    // HP when no HpMax is present, so a missing ceiling renders a full bar rather than a
    // div-by-zero empty one).
    set_progress_bar(
        refs.tu_bar,
        FillFraction::from_ratio(f32::from(*tu), f32::from(**data.tu_max)),
        &widgets.children,
        &mut widgets.fills,
    );
    // The TU cur/max numeric label (the magazine "30/30" formatting model — GTW-310).
    write_text(
        &mut widgets.texts,
        refs.tu_label,
        &format!("{}/{}", *tu, **data.tu_max),
    );
    // HP ceiling: HpMax when present, else the current HP (so a missing ceiling renders a
    // full bar rather than a div-by-zero empty one).
    let hp_max = data.hp_max.map_or(*hp, |m| **m);
    set_progress_bar(
        refs.hp_bar,
        FillFraction::from_ratio(f32::from(*hp), f32::from(hp_max)),
        &widgets.children,
        &mut widgets.fills,
    );
    // The HP cur/max numeric label — current over the SAME display ceiling the bar uses
    // (HpMax when present, else current HP), so the number equals the rendered fill ratio.
    write_text(
        &mut widgets.texts,
        refs.hp_label,
        &format!("{}/{}", *hp, hp_max),
    );

    // Wounds pips — show the first `WoundsMax` pips (filled per the remaining `Wounds`
    // pool, the rest empty), hide the surplus pool pips. WoundsMax defaults to the current
    // Wounds count when no ceiling is present, capped at the pooled row size.
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

/// The empty-state title shown when a panel has no target (no selection / nothing
/// hovered) — a clear empty state, written into the stat block's name line by
/// [`clear_stat_block`].
///
/// A `const` so the empty-state copy lives in one place (the status-panel `NO_SELECTION`
/// precedent — preserved verbatim so the existing empty-state assertion still reads).
pub(in crate::states::running::game::battlescape) const NO_TARGET: &str = "No ganger selected";

/// Resets the stat block referenced by `refs` to its EMPTY state — the contract's
/// no-target appearance (the status panel's "No ganger selected", the hover panel's
/// nothing-hovered): the name line shows [`NO_TARGET`], the faction/stance lines clear,
/// the TU/HP bars empty, all Wounds pips hide, and the wound-name list hides. Never stale
/// data ([[ui-mutate-not-respawn]] — every widget mutates, none respawn).
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
    // Hide every pip, the wound list, and the injury list.
    update_pips(refs.wounds, 0, 0, widgets);
    update_wound_list(refs.wound_list, None, widgets);
    update_injury_list(refs.injury_list, None, widgets);
}
