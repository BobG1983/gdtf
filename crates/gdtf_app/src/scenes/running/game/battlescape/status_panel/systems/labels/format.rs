//! Pure format helpers that render a selected ganger's typed vital components into
//! their display strings (GTW-252, AC5).
//!
//! Each helper takes the real typed sim component(s) and returns the line's
//! [`String`] — no [`App`](bevy::prelude::App), no `World`, no ECS. They are the
//! presentation policy of the status panel (how a vital reads on screen), isolated
//! so they are unit-testable directly and the update system stays a thin wiring
//! layer. The vocabulary (Standing / Crouching / Prone, Alive / Downed / Dead)
//! matches the sim's named domain enums.

use gdtf_battle_sim::{
    Faction, GangerName, Hp, LifeState, Stance, StanceKind, Tu, TuMax, WeaponName, Wounds,
};

/// The line shown when there is no selection — a clear empty state (AC4).
///
/// A `const` so the empty-state copy lives in one place and the update system can
/// write it without re-allocating a literal per line per frame.
pub(in crate::scenes::running::game::battlescape::status_panel) const NO_SELECTION: &str =
    "No ganger selected";

/// The weapon-name fallback shown when the SELECTED ganger carries no weapon (no
/// [`WeaponName`] component) — an em-dash so an unarmed selection reads as having no
/// weapon, distinct from the no-selection empty state (GTW-254).
///
/// A `const` so the unarmed copy lives in one place (the `NO_SELECTION` precedent).
pub(in crate::scenes::running::game::battlescape::status_panel) const UNARMED: &str = "—";

/// The name fallback shown when the SELECTED ganger carries no name (no
/// [`GangerName`] component) — a placeholder so a nameless selection reads as
/// unnamed without a panic, distinct from the no-selection empty state (GTW-285).
///
/// A `const` so the no-name copy lives in one place (the [`UNARMED`] precedent).
pub(in crate::scenes::running::game::battlescape::status_panel) const NAMELESS: &str = "???";

/// The **identity** line: the ganger's [`GangerName`] and its faction (gang) index.
///
/// The selected ganger's NAME stands as its identity (GTW-285 — replacing the
/// placeholder cell location, per the user's "remove the cell location, show
/// names"). Taken as an `Option<&GangerName>` (defensive — gangers are named today,
/// but a selection without the component must not panic): [`Some`] renders the name,
/// [`None`] renders the [`NAMELESS`] fallback. [`GangerName`]
/// [`Deref`](std::ops::Deref)s to its inner `&str`; by reference because it owns a
/// `String` (not `Copy`). [`Faction`] derefs to its small gang index, taken by value
/// (clippy `trivially_copy_pass_by_ref` — a 1-byte `Copy` newtype). The faction text
/// is kept as-is for now (the mockup's faction sigil/coloring is GTW-278 layout).
#[must_use]
pub(in crate::scenes::running::game::battlescape::status_panel) fn identity_label(
    name: Option<&GangerName>,
    faction: Faction,
) -> String {
    let name = name.map_or(NAMELESS, |n| n);
    format!("{name}  Gang {}", *faction)
}

/// The **stance** line: the ganger's posture by its [`StanceKind`] name.
///
/// [`Stance`] derefs to the [`StanceKind`] posture; the rendered word matches the
/// sim's named vocabulary (resolution.md's stand / kneel / prone postures). Taken by
/// value — `Stance` is a 1-byte `Copy` newtype (clippy `trivially_copy_pass_by_ref`).
#[must_use]
pub(in crate::scenes::running::game::battlescape::status_panel) fn stance_label(
    stance: Stance,
) -> String {
    let word = match *stance {
        StanceKind::Standing => "Standing",
        StanceKind::Crouching => "Crouching",
        StanceKind::Prone => "Prone",
    };
    format!("Stance: {word}")
}

/// The **time-units** line: the current [`Tu`] pool over the [`TuMax`] ceiling, as
/// `cur/max` (e.g. "TU 7/10").
///
/// Both newtypes deref to their inner `u8` budget — current is the live pool, max the
/// round-start ceiling (the GTW-38 reaction-ratio denominator). Taken by value — each
/// is a 1-byte `Copy` newtype (clippy `trivially_copy_pass_by_ref`).
#[must_use]
pub(in crate::scenes::running::game::battlescape::status_panel) fn tu_label(
    tu: Tu,
    tu_max: TuMax,
) -> String {
    format!("TU {}/{}", *tu, *tu_max)
}

/// The **hit-points** line: the current [`Hp`] pool plus the [`Wounds`] count.
///
/// There is NO `HpMax` component today, so HP is shown as a bare current value (no
/// fabricated max — an `HpMax` source is flagged future work). [`Hp`] derefs to its
/// `u16` knock-down pool, [`Wounds`] to its `u8` life pool. Taken by value — both are
/// small `Copy` newtypes (clippy `trivially_copy_pass_by_ref`).
#[must_use]
pub(in crate::scenes::running::game::battlescape::status_panel) fn hp_label(
    hp: Hp,
    wounds: Wounds,
) -> String {
    format!("HP {}  Wounds {}", *hp, *wounds)
}

/// The **life-state** line: the ganger's terminal [`LifeState`] by name.
///
/// The two-pool outcome (Alive / Downed / Dead), matching the sim's named state
/// machine. Taken by value — [`LifeState`] is a 1-byte `Copy` enum (clippy
/// `trivially_copy_pass_by_ref`).
#[must_use]
pub(in crate::scenes::running::game::battlescape::status_panel) fn life_label(
    life: LifeState,
) -> String {
    let word = match life {
        LifeState::Alive => "Alive",
        LifeState::Downed => "Downed",
        LifeState::Dead => "Dead",
    };
    format!("State: {word}")
}

/// The **weapon-name** line (GTW-254): the ganger's carried weapon by its
/// [`WeaponName`] (e.g. "Weapon: autogun"), or the [`UNARMED`] em-dash fallback when
/// the selection carries no weapon.
///
/// [`WeaponName`] [`Deref`](std::ops::Deref)s to its inner `&str`. Taken as an
/// `Option<&WeaponName>` (defensive — gangers are armed today, but a selection without
/// the component must not panic): [`Some`] renders the name, [`None`] renders the
/// unarmed fallback. By reference because [`WeaponName`] owns a `String` (not `Copy`).
#[must_use]
pub(in crate::scenes::running::game::battlescape::status_panel) fn weapon_name_label(
    weapon: Option<&WeaponName>,
) -> String {
    let name = weapon.map_or(UNARMED, |w| w);
    format!("Weapon: {name}")
}
