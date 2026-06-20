//! Marker components + the spawned-entity handle bundle for the shared ganger
//! **stat block** (GTW-278 / GTW-274).
//!
//! The stat block is the one render of a ganger's portrait / name / faction / stance /
//! TU+HP bars / Wounds pips / wound-name list, built once by
//! [`spawn_stat_block`](super::spawn_stat_block) and reused by BOTH the top-left status
//! panel (reads `SelectedShooter`) and the top-right inspect panel (reads `InspectTarget`).
//! Each panel spawns its OWN stat block, so the two coexist; the widgets are found for
//! the per-update mutate via the [`StatBlockRefs`] handle the builder stamps on the
//! block root (stable entity ids — [[ui-mutate-not-respawn]]), not by a shared marker.
//!
//! The per-widget markers below carry the panel-discriminating identity the
//! integration tests query by (`status_panel` / `inspect_panel` attach their own variant
//! alongside the shared widget so a test can name "the status panel's HP bar" vs "the
//! inspect panel's HP bar"). They are **unit structs** — presence alone is the signal, no
//! domain value, so the no-bare-types rule does not apply (the status-panel per-line
//! marker precedent).

use bevy::prelude::*;

/// The handle bundle the [`spawn_stat_block`](super::spawn_stat_block) builder stamps on
/// the stat-block root, holding the [`Entity`] of every widget the per-update mutate
/// touches.
///
/// Per [[ui-mutate-not-respawn]] the widgets are spawned ONCE and only ever mutated; the
/// updater ([`update_stat_block`](super::update_stat_block)) reads these stored ids to
/// find each widget rather than re-querying by marker, so the same entity ids survive
/// every update. A `Component` (a framework type, exempt from no-bare-types) whose fields
/// are all real [`Entity`] handles — the cross-system identity Bevy uses for "the same
/// widget", never a numeric id.
#[derive(Component, Clone, Copy, Debug)]
pub(in crate::states::running::game::battlescape) struct StatBlockRefs {
    /// The portrait [`ImageNode`](bevy::ui::widget::ImageNode) whose atlas index the
    /// update mutates to the ganger's deterministic face.
    pub portrait:   Entity,
    /// The name-title `Text`.
    pub name:       Entity,
    /// The faction `Text`.
    pub faction:    Entity,
    /// The stance `Text`.
    pub stance:     Entity,
    /// The TU `ProgressBar` track (its fill width is mutated).
    pub tu_bar:     Entity,
    /// The TU `cur/max` numeric `Text` sitting above the TU bar (its content is mutated).
    pub tu_label:   Entity,
    /// The HP `ProgressBar` track (its fill width is mutated).
    pub hp_bar:     Entity,
    /// The HP `cur/max` numeric `Text` sitting above the HP bar (its content is mutated).
    pub hp_label:   Entity,
    /// The Wounds `Pips` row (its pip colors are mutated).
    pub wounds:     Entity,
    /// The wound-name list container (its line children + its visibility are mutated).
    pub wound_list: Entity,
}

crate::support_item! {
    /// Marks the **portrait** [`ImageNode`](bevy::ui::widget::ImageNode) of a stat block —
    /// the face whose atlas index the update mutates to the ganger's deterministic portrait.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatPortrait;
}

crate::support_item! {
    /// Marks the **name-title** `Text` of a stat block — the selected / hovered ganger's
    /// [`GangerName`](gdtf_battle_sim::GangerName), `"???"` when nameless.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatName;
}

crate::support_item! {
    /// Marks the **faction** `Text` of a stat block — the ganger's faction (gang) index.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatFaction;
}

crate::support_item! {
    /// Marks the **stance** `Text` of a stat block — the ganger's posture word.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatStance;
}

crate::support_item! {
    /// Marks the **TU** `ProgressBar` track of a stat block — fill = `Tu`/`TuMax`.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatTuBar;
}

crate::support_item! {
    /// Marks the **TU** `cur/max` numeric `Text` of a stat block — the literal `Tu`/`TuMax`
    /// value (e.g. `"7/10"`) shown above the TU bar so the player reads the exact numbers,
    /// not just the bar fill (GTW-310).
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatTuLabel;
}

crate::support_item! {
    /// Marks the **HP** `ProgressBar` track of a stat block — fill = `Hp`/`HpMax`.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatHpBar;
}

crate::support_item! {
    /// Marks the **HP** `cur/max` numeric `Text` of a stat block — the literal `Hp`/`HpMax`
    /// value (e.g. `"8/16"`) shown above the HP bar so the player reads the exact numbers,
    /// not just the bar fill (GTW-310).
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatHpLabel;
}

crate::support_item! {
    /// Marks the **Wounds** `Pips` row of a stat block — `WoundsMax` pips, `Wounds`
    /// filled.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatWoundsPips;
}

crate::support_item! {
    /// Marks the **wound-name list** container of a stat block — the vertical list of
    /// `"{tier} — {location}"` lines, hidden when the ganger has no inflicted wounds.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatWoundList;
}

crate::support_item! {
    /// Marks one **wound-name line** `Text` inside the wound-name list — a pooled line
    /// whose content + visibility the update sets, so the list mutates in place rather
    /// than respawning per wound ([[ui-mutate-not-respawn]]).
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StatWoundLine;
}
