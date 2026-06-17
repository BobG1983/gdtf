//! Repaints the status panel's vitals lines from the selected player ganger
//! (GTW-252, AC2 / AC3 / AC4).
//!
//! [`update_status_panel`] reads [`Res<SelectedShooter>`](gdtf_battle_input::SelectedShooter),
//! resolves `**selected` to the selected [`Entity`], `.get`s its vital components in
//! ONE query tuple, and writes each panel line's [`Text`](bevy::prelude::Text) from
//! the pure [`labels`](super::labels) helpers. With no selection (or a selected
//! entity that lacks the vital components) every line shows the
//! [`NO_SELECTION`](super::labels::NO_SELECTION) empty state — never stale data,
//! never a panic.
//!
//! It runs in `Update` gated `run_if(resource_exists::<BattleInProgress>)` (the live-
//! battle witness the action-bar / input / presenter gate on, `bevy-traps.md` #1), so
//! it is inert outside a live battle. It repaints every battle frame — a first-cut
//! presentation choice (change-detection gating is a later optimization, AC's perf
//! note).
//!
//! ## Why the line writes go through a [`ParamSet`]
//!
//! Each line is written via a `Query<&mut Text, With<…line marker…>>`. Five such
//! queries all mutably access [`Text`](bevy::prelude::Text); disjoint `With<>` filters
//! do NOT make same-component `&mut` access disjoint, so registering them as five
//! separate params would be a query conflict. They are nested in a [`ParamSet`] (the
//! GTW-242 precedent) so only one is borrowed at a time — never `&mut World`
//! (`bevy-traps.md` #7).

use bevy::{
    ecs::{query::QueryData, system::SystemParam},
    prelude::*,
};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{Faction, Hp, LifeState, Position, Stance, Tu, TuMax, WeaponName, Wounds};

use crate::scenes::running::game::battlescape::status_panel::{
    components::{HpText, IdentityText, LifeText, StanceText, TuText, WeaponNameText},
    systems::labels::{
        NO_SELECTION, hp_label, identity_label, life_label, stance_label, tu_label,
        weapon_name_label,
    },
};

/// The selected ganger's vital components, read in ONE query tuple.
///
/// A [`QueryData`] struct so the wide read does not trip clippy `type_complexity` (the
/// GTW-198 sub-tuple shape precedent for a wide read), while keeping the contract's
/// single-tuple `.get`. `pub(in …status_panel)` so it is at least as visible as the
/// system that names it in its signature (clippy `private_interfaces`).
#[derive(QueryData)]
pub(in crate::scenes::running::game::battlescape::status_panel) struct Vitals {
    /// The ganger's stance posture.
    stance:   &'static Stance,
    /// The ganger's current TU pool.
    tu:       &'static Tu,
    /// The ganger's round-start TU ceiling.
    tu_max:   &'static TuMax,
    /// The ganger's current HP pool.
    hp:       &'static Hp,
    /// The ganger's Wounds (life) pool.
    wounds:   &'static Wounds,
    /// The ganger's `(cell, level)` position.
    position: &'static Position,
    /// The ganger's faction (gang) identity.
    faction:  &'static Faction,
    /// The ganger's terminal life state.
    life:     &'static LifeState,
    /// The ganger's carried weapon name, if armed (GTW-254 — read defensively as an
    /// [`Option`] so an unarmed selection still resolves, rather than the whole `.get`
    /// failing closed and blanking every OTHER vital line).
    weapon:   Option<&'static WeaponName>,
}

/// One per-line `&mut Text` writer query, parameterised by the line marker `M`.
///
/// A type alias (the GTW-242 `TurnQuery` precedent) so each per-line writer is a single
/// named type. All five writers mutate [`Text`](bevy::prelude::Text).
type LineWriter<'world, 'state, M> = Query<'world, 'state, &'static mut Text, With<M>>;

/// The five per-line writer queries' [`ParamSet`], factored out so the [`LineWriters`]
/// field type is a single named alias rather than a "very complex type"
/// (clippy `type_complexity`). Disjoint `With<>` markers do NOT make same-component
/// `&mut` access disjoint, so the `ParamSet` time-multiplexes the five writers — one
/// borrowed at a time, never `&mut World` (`bevy-traps.md` #7).
type LineParamSet<'w, 's> = ParamSet<
    'w,
    's,
    (
        LineWriter<'w, 's, IdentityText>,
        LineWriter<'w, 's, StanceText>,
        LineWriter<'w, 's, TuText>,
        LineWriter<'w, 's, HpText>,
        LineWriter<'w, 's, LifeText>,
        LineWriter<'w, 's, WeaponNameText>,
    ),
>;

/// The five per-line `&mut Text` writer queries, bundled in a [`ParamSet`].
///
/// A [`SystemParam`] bundle (the GTW-204 `BattleGridsParam` precedent) so the wide
/// `ParamSet` lives in the struct definition rather than the system signature — which
/// keeps the system's parameter list out of clippy `type_complexity` and avoids the
/// `ParamSet`-as-a-bare-type-alias-system-param breakage. `pub(in …status_panel)` so it
/// is at least as visible as the system that names it (clippy `private_interfaces`).
#[derive(SystemParam)]
pub(in crate::scenes::running::game::battlescape::status_panel) struct LineWriters<'w, 's> {
    /// The five per-line writer queries, only one borrowed at a time.
    lines: LineParamSet<'w, 's>,
}

/// Repaints every status-panel line from the current [`SelectedShooter`].
///
/// Resolves the selection to its vital components (ONE `.get` over the wide read-only
/// query) and writes each line; an empty selection — or a selected entity missing the
/// vital components — paints every line the [`NO_SELECTION`] empty state (AC4: no
/// panic, no stale data). The values always reflect the ganger's CURRENT state because
/// the components are read fresh every run (AC3: after the ganger acts, the lines
/// update).
///
/// The five per-line writers all mutate [`Text`](bevy::prelude::Text); they are bundled
/// in [`LineWriters`]' [`ParamSet`] (the GTW-242 precedent) — one borrowed at a time,
/// never `&mut World`.
///
/// Param-only (`bevy-traps.md` #7): [`Res<SelectedShooter>`] + the read-only [`Vitals`]
/// query + the [`LineWriters`] `ParamSet` bundle.
pub(in crate::scenes::running::game::battlescape) fn update_status_panel(
    selected: Res<SelectedShooter>,
    vitals: Query<Vitals>,
    mut writers: LineWriters,
) {
    // Resolve the selection to its rendered lines, or the empty state. `.get` fails
    // closed (None) for both "nothing selected" and "selected entity lacks the vital
    // components" — either way every line shows the empty state, never stale data. The
    // small `Copy` newtypes are passed by value to the helpers (clippy
    // `trivially_copy_pass_by_ref`); the 12-byte `Position` by reference.
    let rendered = (**selected)
        .and_then(|entity| vitals.get(entity).ok())
        .map(|v| RenderedLines {
            identity: identity_label(v.position, *v.faction),
            stance:   stance_label(*v.stance),
            tu:       tu_label(*v.tu, *v.tu_max),
            hp:       hp_label(*v.hp, *v.wounds),
            life:     life_label(*v.life),
            weapon:   weapon_name_label(v.weapon),
        });

    // Borrow each line query in turn (ParamSet: one at a time) and write its text — the
    // rendered value when a ganger is selected, else the shared empty state.
    write_line(
        writers.lines.p0(),
        rendered.as_ref().map(|r| r.identity.as_str()),
    );
    write_line(
        writers.lines.p1(),
        rendered.as_ref().map(|r| r.stance.as_str()),
    );
    write_line(writers.lines.p2(), rendered.as_ref().map(|r| r.tu.as_str()));
    write_line(writers.lines.p3(), rendered.as_ref().map(|r| r.hp.as_str()));
    write_line(
        writers.lines.p4(),
        rendered.as_ref().map(|r| r.life.as_str()),
    );
    write_line(
        writers.lines.p5(),
        rendered.as_ref().map(|r| r.weapon.as_str()),
    );
}

/// The five rendered vitals strings for the selected ganger.
///
/// An internal bundle so the one read of the selection produces all five lines
/// together, then the [`ParamSet`] writes them one borrow at a time.
struct RenderedLines {
    /// The identity (cell + faction) line.
    identity: String,
    /// The stance posture line.
    stance:   String,
    /// The TU `cur/max` line.
    tu:       String,
    /// The HP + Wounds line.
    hp:       String,
    /// The life-state line.
    life:     String,
    /// The weapon-name line (GTW-254).
    weapon:   String,
}

/// Writes `value` (or the [`NO_SELECTION`] empty state when `value` is [`None`]) into
/// the single line `Text` of `query`, only when the content differs.
///
/// Guards the write on a content change so a same-value frame does not spuriously
/// mark the `Text` `Changed` (change-detection hygiene), while still repainting every
/// frame the value actually moves.
fn write_line<F: bevy::ecs::query::QueryFilter>(
    mut query: Query<&mut Text, F>,
    value: Option<&str>,
) {
    let next = value.unwrap_or(NO_SELECTION);
    for mut text in &mut query {
        if text.as_str() != next {
            next.clone_into(&mut text.0);
        }
    }
}
