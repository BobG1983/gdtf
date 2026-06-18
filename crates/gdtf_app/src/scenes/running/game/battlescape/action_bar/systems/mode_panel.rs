//! The fire-mode 3-toggle sub-panel (GTW-265 / GTW-284) — the always-visible replacement
//! for the removed GTW-254 popup picker.
//!
//! The Mode sub-panel ([`ModePanelRoot`]) is a vertical [`spawn_panel`] column inside the
//! action bar whose CHILDREN are the THREE per-mode toggle buttons — [`ModeSingleButton`] /
//! [`ModeBurstButton`] / [`ModeFullButton`]. Clicking a toggle sets
//! [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode) DIRECTLY to that mode's
//! read-back [`FireModeSpec`](gdtf_battle_sim::FireModeSpec) (never fabricated), and the
//! current mode is shown via the `gdtf_ui` [`ActiveButton`] paint marker (the GTW-253
//! hook, made sticky by GTW-266). No modal, no scrim, no z-stacking, no world
//! click-through — by construction the whole popup bug class is gone.
//!
//! ## Mutate, never churn (GTW-284 / [[ui-mutate-not-respawn]])
//!
//! The THREE toggles are spawned ONCE — at panel spawn ([`spawn_mode_panel`]), each
//! tagged with its mode marker and its fixed label — and are NEVER despawned/respawned on a
//! selection change. The offered modes change with the selected weapon, so
//! [`rebuild_mode_buttons`] MUTATES each toggle's [`Visibility`] to show ONLY the modes the
//! SELECTED weapon offers ([`Visibility::Hidden`] for the modes it lacks), leaving the
//! entities (and their stable [`Entity`] ids) in place. A Single+Burst weapon shows the
//! Single + Burst toggles and hides Full.
//!
//! This is the GTW-284 fix: the old body despawned + respawned the `Themed` toggles on
//! every selection change, so a fresh spawn raised `Added<Themed>` → the (then-unfiltered)
//! `gdtf_ui::apply_theme` repainted ALL `Themed` widgets, clobbering every button's
//! hover / [`ActiveButton`] fill for a frame. With no entity churn there is no spurious
//! `Added<Themed>` to trigger that global repaint.
//!
//! It runs `.after(UiSystems::ApplyTheme)` (`bevy-traps.md` #3) so its visibility writes
//! settle deterministically relative to the theme pass. An UNARMED selection (no
//! [`FireMode`](gdtf_battle_sim::FireMode)) hides every toggle and the panel root.

use bevy::prelude::*;
use gdtf_battle_input::{SelectedFireMode, SelectedShooter};
use gdtf_battle_sim::{FireMode, FireModeSpec, ModeKind};
use gdtf_ui::{ActiveButton, ButtonLabel, spawn_button, theme::GdtfTheme};

use super::actions::{PressedButton, is_press};
use crate::scenes::running::game::battlescape::action_bar::components::{
    ModeBurstButton, ModeFullButton, ModePanelRoot, ModeSingleButton,
};

/// Query FILTER selecting ONLY the Single mode toggle's [`Visibility`], disjoint from the
/// panel-root + the other two toggles' `&mut Visibility` queries in
/// [`rebuild_mode_buttons`] (so Bevy proves the four mutable borrows non-conflicting).
///
/// Aliased so the four `&mut Visibility` query types stay legible (clippy
/// `type_complexity`). The `Without` clauses are the disjointness proof: each entity carries
/// at most one of these markers, so the four queries can mutably borrow `Visibility` in one
/// system.
type SingleToggle = (With<ModeSingleButton>, Without<ModePanelRoot>);

/// Query FILTER selecting ONLY the Burst mode toggle's [`Visibility`] (see [`SingleToggle`]).
type BurstToggle = (
    With<ModeBurstButton>,
    Without<ModePanelRoot>,
    Without<ModeSingleButton>,
);

/// Query FILTER selecting ONLY the Full mode toggle's [`Visibility`] (see [`SingleToggle`]).
type FullToggle = (
    With<ModeFullButton>,
    Without<ModePanelRoot>,
    Without<ModeSingleButton>,
    Without<ModeBurstButton>,
);

/// Horizontal gap between the Firemode panel's toggle buttons, in logical pixels.
///
/// A named newtype over the gap rather than a bare `f32` (no-bare-types rule): it is
/// layout spacing, not a theme color/size (the action-bar `BarGapPx` precedent).
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
struct ModeGapPx(f32);

impl ModeGapPx {
    /// The Firemode panel's inter-toggle gap: 4 px (a tight side-by-side toggle row).
    const PANEL: Self = Self(4.0);
}

/// Spawns the Mode sub-panel column ([`ModePanelRoot`]) with its THREE FIXED per-mode
/// toggles as children, and returns the panel [`Entity`] so `spawn_action_bar` can parent
/// it under the bar root (GTW-265 / GTW-284).
///
/// A themed [`spawn_panel`](gdtf_ui::spawn_panel) laid out as a vertical column whose
/// children are the Single / Burst / Full toggle buttons, spawned ONCE here (GTW-284: the
/// toggles are MUTATED in place by [`rebuild_mode_buttons`], never despawned/respawned).
/// Each toggle is tagged with its mode marker and its fixed
/// [`Display`](std::fmt::Display) label, and starts [`Visibility::Hidden`] — the panel root
/// is also [`Visibility::Hidden`] until [`rebuild_mode_buttons`] reveals exactly the modes
/// the first selected weapon offers (GTW-273). Returns the panel so the caller parents it
/// in the bar's left-to-right row. Takes `&mut Commands` + the live theme (the
/// `spawn_action_bar` precedent).
pub(in crate::scenes::running::game::battlescape) fn spawn_mode_panel(
    commands: &mut Commands,
    theme: &GdtfTheme,
) -> Entity {
    let panel = gdtf_ui::spawn_panel(commands, theme);
    commands.entity(panel).insert((
        ModePanelRoot,
        Node {
            // GTW-298: the Firemode panel FILLS its bottom-left grid cell; its 1-3 visible
            // toggles sit side by side in a ROW, each filling the panel height and sharing
            // the panel width (width varies by count). A `Row` with full width/height + the
            // toggles' `flex_grow` produces the contract's "width varies by count, height
            // fills the panel".
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(*ModeGapPx::PANEL),
            // Clip any toggle whose label is wider than its flex share so the firemode row
            // never overflows the panel cell into the neighbouring Aim panel (item 8 — no
            // panel overlaps another).
            overflow: bevy::ui::Overflow {
                x: bevy::ui::OverflowAxis::Hidden,
                y: bevy::ui::OverflowAxis::Hidden,
            },
            ..default()
        },
        // GTW-273: HIDDEN until an armed selection with modes reveals the offered toggles.
        // The three toggle children spawn `Hidden` too, so an empty/over-full Mode box never
        // shows before / when nothing armed is selected. `rebuild_mode_buttons` sets the
        // root + per-toggle visibility on every selection-change branch; this initial
        // `Hidden` covers the pre-first-change gap (and the never-selected empty battle,
        // where the rebuild's no-op / unarmed branch keeps it hidden).
        Visibility::Hidden,
    ));

    // GTW-284: spawn the THREE FIXED toggles ONCE, each `Hidden` — `rebuild_mode_buttons`
    // reveals exactly the offered ones on selection change (no despawn/respawn churn). Each
    // toggle carries its fixed label (the kind's `Display`), so its caption never changes.
    let single = spawn_hidden_toggle(commands, theme, ModeKind::Single, ModeSingleButton);
    let burst = spawn_hidden_toggle(commands, theme, ModeKind::Burst, ModeBurstButton);
    let full = spawn_hidden_toggle(commands, theme, ModeKind::Full, ModeFullButton);
    commands.entity(panel).add_children(&[single, burst, full]);

    panel
}

/// Spawns one Mode toggle [`spawn_button`] for `kind`, tagged with its `marker`, with the
/// kind's [`Display`](std::fmt::Display) as its fixed label, starting [`Visibility::Hidden`]
/// (GTW-284).
///
/// The shared toggle constructor for [`spawn_mode_panel`]'s fixed Single / Burst / Full
/// set: each toggle exists for the whole bar lifetime and is only ever MUTATED (its
/// [`Visibility`] and its [`ActiveButton`] marker), never churned. Returns the toggle so
/// the caller parents it under the panel root.
fn spawn_hidden_toggle<M: Component>(
    commands: &mut Commands,
    theme: &GdtfTheme,
    kind: ModeKind,
    marker: M,
) -> Entity {
    let toggle = spawn_button(
        commands,
        theme,
        ButtonLabel::new(toggle_label(kind)),
        marker,
    );
    // GTW-298: each visible toggle FILLS the panel height + shares the panel width with its
    // siblings (`flex_grow` + zero `flex_basis` → equal shares; width varies by visible count).
    // Overwriting the auto-sized `box_node` is safe — `apply_theme` re-applies the theme-owned
    // border / radius / padding every run, preserving these layout fields.
    commands.entity(toggle).insert((
        Node {
            height: Val::Percent(100.0),
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_basis: Val::Percent(0.0),
            // Allow the toggle to shrink below its label's intrinsic width + clip the caption,
            // so a wide label (e.g. "full-auto") shares the row evenly instead of overflowing
            // the panel into the Aim cell.
            min_width: Val::Px(0.0),
            // GTW-298: the Firemode panel is the bottom 1/4-height cell — a SHORT strip. Without a
            // zero `min_height` the button's intrinsic content (18pt label + theme padding) is its
            // flex min-height, so the toggles refuse to compress and overflow the cell (the label
            // wrapping + the toggles overlapping the row below). A zero `min_height` lets flexbox
            // compress them to the cell height; `overflow: Hidden` then clips the label cleanly.
            min_height: Val::Px(0.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            overflow: bevy::ui::Overflow {
                x: bevy::ui::OverflowAxis::Hidden,
                y: bevy::ui::OverflowAxis::Hidden,
            },
            ..default()
        },
        Visibility::Hidden,
    ));
    toggle
}

/// The DISPLAYED firemode-toggle caption for `kind` — `"single"` / `"burst"` for those modes
/// (the sim's canonical [`ModeKind`] [`Display`](std::fmt::Display) label), and the SHORTER
/// `"auto"` for [`ModeKind::Full`] (screenshot review 2026-06-18: at the comfortably-legible
/// `CONTROL_LABEL_PT` font the full sim label `"full-auto"` clipped in the narrow firemode cell,
/// so the firemode panel displays it as `"auto"`).
///
/// A presentation-only override local to this firemode panel: it does NOT change the sim's
/// [`ModeKind`] [`Display`] (which other readers may rely on) — only what this UI toggle shows.
/// Single / Burst pass through unchanged.
fn toggle_label(kind: ModeKind) -> String {
    match kind {
        ModeKind::Full => "auto".to_owned(),
        other => other.to_string(),
    }
}

/// MUTATES the Mode sub-panel's three FIXED toggles' [`Visibility`] to show exactly the
/// modes the SELECTED weapon offers, whenever
/// [`SelectedShooter`](gdtf_battle_input::SelectedShooter) changes (GTW-265 / GTW-284: a
/// Single+Burst weapon shows the Single + Burst toggles and HIDES Full).
///
/// GTW-284 ([[ui-mutate-not-respawn]]) — it NEVER despawns/respawns toggles. The three
/// toggles are spawned once by [`spawn_mode_panel`]; on a selection change this system sets
/// each toggle's [`Visibility`] to [`Visibility::Visible`] if the selected weapon's
/// [`FireMode`](gdtf_battle_sim::FireMode) selector offers that
/// [`ModeKind`](gdtf_battle_sim::ModeKind), else [`Visibility::Hidden`] — so the toggle
/// [`Entity`] ids stay STABLE across the change (no `Added<Themed>` churn that would trigger
/// `gdtf_ui::apply_theme`'s repaint). An UNARMED selection (no `FireMode`) or a cleared
/// selection hides all three. Runs `.after(UiSystems::ApplyTheme)` (see the module docs).
///
/// It runs its body on a real selection change OR when the [`ModePanelRoot`] is freshly
/// spawned ([`Added<ModePanelRoot>`](Added)) — the battle-start auto-select fills
/// [`SelectedShooter`](gdtf_battle_input::SelectedShooter) several frames BEFORE the action
/// bar spawns its panel, so the selection-change has already passed by the time the panel
/// exists; the `Added` trigger re-reads the CURRENT selection on the spawn frame (the
/// GTW-255 auto-select ordering trap). Otherwise it early-returns (change-detection
/// hygiene).
///
/// GTW-273 — it also drives the [`ModePanelRoot`]'s own [`Visibility`] on EVERY branch: an
/// armed selection that HAS modes flips it [`Visibility::Visible`]; an unarmed / cleared /
/// no-mode selection sets it [`Visibility::Hidden`], so there is never an empty Mode box
/// when nothing armed is selected (the root's `Hidden` hides every toggle child too). The
/// root is never despawned — only its and its toggles' visibility change.
///
/// Param-only (`bevy-traps.md` #7): the `Res<SelectedShooter>` read, a read-only
/// `Query<&FireMode>`, an [`Added<ModePanelRoot>`](Added) spawn detector, the panel-root
/// `&mut`[`Visibility`] query, and one `&mut`[`Visibility`] query per mode toggle (disjoint
/// by marker) — no `Commands`, no `&mut World`.
pub(in crate::scenes::running::game::battlescape) fn rebuild_mode_buttons(
    selected: Res<SelectedShooter>,
    weapons: Query<&FireMode>,
    added_panels: Query<(), Added<ModePanelRoot>>,
    mut panels: Query<&mut Visibility, With<ModePanelRoot>>,
    mut single: Query<&mut Visibility, SingleToggle>,
    mut burst: Query<&mut Visibility, BurstToggle>,
    mut full: Query<&mut Visibility, FullToggle>,
) {
    // Re-read on a real selection change OR when the Mode panel was JUST spawned (the bar's
    // `OnEnter(BattleRunning)` `spawn_mode_panel`). The battle-start auto-select
    // (`auto_select_first_player_ganger`) fills `SelectedShooter` several frames BEFORE the
    // action bar spawns its panel, so the selection-change has already passed by the time
    // the panel exists — without the `Added` trigger the toggles would never get their
    // visibility from the CURRENT selection (the GTW-255 auto-select ordering trap).
    let panel_just_spawned = added_panels.iter().next().is_some();
    if !selected.is_changed() && !panel_just_spawned {
        return;
    }

    // Which modes the SELECTED weapon offers (the closed `ModeKind` set). A cleared / no-
    // selection / unarmed (no `FireMode`) selection offers nothing → every toggle hidden.
    let offered = (**selected).and_then(|shooter| weapons.get(shooter).ok());
    let offers =
        |kind: ModeKind| offered.is_some_and(|weapon| weapon.iter().any(|m| m.kind == kind));

    // MUTATE each fixed toggle's visibility to its offered state — no despawn/respawn.
    set_visibility(&mut single, offers(ModeKind::Single));
    set_visibility(&mut burst, offers(ModeKind::Burst));
    set_visibility(&mut full, offers(ModeKind::Full));

    // GTW-273 — the panel root is VISIBLE only when at least one mode is offered (an armed
    // selection with modes), HIDDEN otherwise, so there is never an empty Mode box.
    let any_offered = offers(ModeKind::Single) || offers(ModeKind::Burst) || offers(ModeKind::Full);
    let root_want = if any_offered {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut panels {
        if *visibility != root_want {
            *visibility = root_want;
        }
    }
}

/// Sets every [`Visibility`] matched by `query` to [`Visibility::Visible`] when `visible`,
/// else [`Visibility::Hidden`] — writing only on a real change (change-detection hygiene).
///
/// Shared by [`rebuild_mode_buttons`] across the three fixed mode toggles (GTW-284): the
/// per-toggle mutate that REPLACED the old despawn/respawn.
fn set_visibility<F: bevy::ecs::query::QueryFilter>(
    query: &mut Query<&mut Visibility, F>,
    visible: bool,
) {
    let want = if visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut visibility in query {
        if *visibility != want {
            *visibility = want;
        }
    }
}

/// On a Mode toggle press, set [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode)
/// DIRECTLY to that mode's read-back spec off the selected weapon (GTW-265).
///
/// For each pressed toggle ([`ModeSingleButton`] / [`ModeBurstButton`] /
/// [`ModeFullButton`]), it looks up the matching [`FireModeSpec`](gdtf_battle_sim::FireModeSpec)
/// in the selected weapon's [`FireMode`](gdtf_battle_sim::FireMode) selector and sets
/// [`SelectedFireMode`] to it — the read-back value, NEVER a fabricated spec. A toggle was
/// spawned only for a mode the weapon offers, so the lookup is total in practice; a
/// missing mode (defensive) is a no-op. Writes only on a real change (change-detection
/// hygiene). The three press queries are disjoint per marker, so they never conflict.
///
/// Param-only (`bevy-traps.md` #7): the `ResMut<SelectedFireMode>` write, the
/// `Res<SelectedShooter>` read, a read-only `Query<&FireMode>`, and three read-only
/// per-marker press queries — no `&mut World`.
pub(in crate::scenes::running::game::battlescape) fn mode_button_pressed(
    mut fire_mode: ResMut<SelectedFireMode>,
    selected: Res<SelectedShooter>,
    weapons: Query<&FireMode>,
    single: Query<&Interaction, PressedButton<ModeSingleButton>>,
    burst: Query<&Interaction, PressedButton<ModeBurstButton>>,
    full: Query<&Interaction, PressedButton<ModeFullButton>>,
) {
    // Which mode (if any) was pressed this frame. The toggles are disjoint, so at most one
    // matches per frame in practice; the first match wins.
    let pressed: Option<ModeKind> = if single.iter().copied().any(is_press) {
        Some(ModeKind::Single)
    } else if burst.iter().copied().any(is_press) {
        Some(ModeKind::Burst)
    } else if full.iter().copied().any(is_press) {
        Some(ModeKind::Full)
    } else {
        None
    };
    let Some(kind) = pressed else {
        return;
    };
    // Read the selected weapon's spec for that kind — never a fabricated value.
    let Some(spec) = mode_spec_for(*selected, &weapons, kind) else {
        return;
    };
    let next = SelectedFireMode::new(spec);
    if *fire_mode != next {
        *fire_mode = next;
    }
}

/// Drives the three Mode toggles' active (toggled-on) look from the live
/// [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode) (GTW-265).
///
/// Marks exactly the toggle whose [`ModeKind`](gdtf_battle_sim::ModeKind) equals the
/// selected mode's kind [`ActiveButton`](gdtf_ui::ActiveButton) and removes it from the
/// other two, so the current mode reads as toggled-on (the GTW-266 sticky paint). Insert /
/// remove are idempotent, so it runs every frame. Mirrors
/// [`sync_stance_buttons_active`](super::stance_active::sync_stance_buttons_active).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the insert/remove, the
/// `Res<SelectedFireMode>` read, and one `Query<Entity, With<…>>` per mode toggle — no
/// `&mut World`.
pub(in crate::scenes::running::game::battlescape) fn sync_mode_buttons_active(
    mut commands: Commands,
    fire_mode: Res<SelectedFireMode>,
    single_buttons: Query<Entity, With<ModeSingleButton>>,
    burst_buttons: Query<Entity, With<ModeBurstButton>>,
    full_buttons: Query<Entity, With<ModeFullButton>>,
) {
    let active = fire_mode.kind;
    set_active(&mut commands, &single_buttons, active == ModeKind::Single);
    set_active(&mut commands, &burst_buttons, active == ModeKind::Burst);
    set_active(&mut commands, &full_buttons, active == ModeKind::Full);
}

/// The selected weapon's [`FireModeSpec`](gdtf_battle_sim::FireModeSpec) for `kind`, read
/// back off its [`FireMode`](gdtf_battle_sim::FireMode) selector — or [`None`] when there
/// is no selection, the selection is unarmed, or the weapon does not offer `kind`.
///
/// The single read-back point so [`mode_button_pressed`] never fabricates a spec (the
/// GTW-265 "read-back, never fabricated" rule). Read-only over the selection + weapon
/// query.
fn mode_spec_for(
    selected: SelectedShooter,
    weapons: &Query<&FireMode>,
    kind: ModeKind,
) -> Option<FireModeSpec> {
    let shooter = (*selected)?;
    let weapon = weapons.get(shooter).ok()?;
    weapon.iter().find(|spec| spec.kind == kind).copied()
}

/// Inserts or removes [`ActiveButton`](gdtf_ui::ActiveButton) on every button matched by
/// `buttons`, by whether that toggle is the `active` one this frame.
///
/// Shared by [`sync_mode_buttons_active`] across the three mode toggles (the stance-panel
/// `set_active` shape).
fn set_active<M: Component>(
    commands: &mut Commands,
    buttons: &Query<Entity, With<M>>,
    active: bool,
) {
    for button in buttons {
        let mut entity = commands.entity(button);
        if active {
            entity.insert(ActiveButton);
        } else {
            entity.remove::<ActiveButton>();
        }
    }
}
