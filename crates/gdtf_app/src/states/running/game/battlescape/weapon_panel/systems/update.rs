//! Repaints the weapon panel from the selected player ganger (GTW-275).
//!
//! [`update_weapon_panel`] reads [`Res<SelectedShooter>`](gdtf_battle_input::SelectedShooter),
//! resolves it to the selected ganger [`Entity`], then resolves that ganger's WIELDED
//! WEAPON ENTITY through `ganger → `[`Wields`](gdtf_battle_sim::weapon::Wields)` → the weapon
//! entity` (GTW-323 slice 3, ADR-0004 — the weapon stats live on the related weapon
//! entity, not the ganger), and reads the WEAPON entity's
//! [`WeaponName`](gdtf_battle_sim::weapon::WeaponName) + [`Magazine`](gdtf_battle_sim::magazine::Magazine)
//! (both `Option` — a weapon may carry no magazine). It MUTATES the panel widgets in place
//! ([[ui-mutate-not-respawn]]): it sets the name text, the magazine `"cur/max"` text, and
//! reveals / hides the content column + the Reload button by the rules in AC5 / AC6 / AC9.
//! With no selection — or a selected ganger that wields no weapon — the content column is
//! [`Visibility::Hidden`] (the empty state). Never stale, never a panic.
//!
//! It runs in `Update` gated `run_if(resource_exists::<BattleInProgress>)` (the live-battle
//! witness, `bevy-traps.md` #1), `.after(InputSystems::Gather)` (so it observes the same
//! update's auto-select write to `SelectedShooter`, the status-panel ordering precedent).

use bevy::{prelude::*, ui::Display};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    magazine::Magazine,
    weapon::{MeleeWeapon, WeaponName, WieldedBy, Wields},
};

use crate::states::running::game::battlescape::weapon_panel::components::{
    ReloadButton, WeaponContent, WeaponMagazineText, WeaponNameText,
};

/// The read-only weapon state of a wielded WEAPON ENTITY the panel renders — the
/// [`WeaponName`] + [`Magazine`], both `Option` so a weapon with no magazine (or a
/// missing field) reads as "no magazine" rather than failing the query.
///
/// Queried off the WEAPON entity (`With<`[`WieldedBy`]`>`), NOT the ganger (GTW-323
/// slice 3): [`update_weapon_panel`] resolves the selected ganger's
/// [`Wields`] relationship to the weapon entity, then `.get`s this read-struct off it
/// with ONE lookup (the status-panel `StatBlockData` precedent). The `With<WieldedBy>`
/// filter scopes the query to weapon entities. Every field is a borrowed named sim
/// component (no bare primitive).
#[derive(bevy::ecs::query::QueryData)]
pub(in crate::states::running::game::battlescape::weapon_panel) struct WeaponData {
    /// The wielded weapon's display name (absent if the weapon carries no name).
    name:     Option<&'static WeaponName>,
    /// The wielded weapon's magazine grouping (absent if the weapon carries none).
    magazine: Option<&'static Magazine>,
}

/// Query filter selecting the content column's `&mut Visibility`, disjoint from the
/// magazine / reload `&mut Visibility` writers (clippy `type_complexity` — the action-bar
/// `PressedButton` alias precedent).
type ContentFilter = (
    With<WeaponContent>,
    Without<WeaponMagazineText>,
    Without<ReloadButton>,
);
/// Query filter selecting the name text line, disjoint from the magazine `&mut Text` writer.
type NameFilter = (With<WeaponNameText>, Without<WeaponMagazineText>);
/// Query filter selecting the magazine text + visibility, disjoint from every other
/// mutable query.
type MagazineFilter = (
    With<WeaponMagazineText>,
    Without<WeaponContent>,
    Without<WeaponNameText>,
    Without<ReloadButton>,
);
/// Query filter selecting the Reload button's `&mut Visibility`, disjoint from the content
/// / magazine `&mut Visibility` writers.
type ReloadFilter = (
    With<ReloadButton>,
    Without<WeaponContent>,
    Without<WeaponMagazineText>,
);

/// The panel widgets [`update_weapon_panel`] mutates, grouped into ONE
/// [`SystemParam`](bevy::ecs::system::SystemParam) so the system's parameter list stays
/// under clippy's argument-count gate (the sim's `BattleGridsParam` precedent).
///
/// Each field is a disjoint write query over one widget marker (a `Text` or a
/// `Visibility`); the `Without`-bearing filter aliases prove the disjointness so the three
/// `Visibility` writers + the two `Text` writers never trip Bevy's B0001 conflict check. A
/// transparent system-param bundle of framework queries — not itself a wrapped domain scalar.
#[derive(bevy::ecs::system::SystemParam)]
pub(in crate::states::running::game::battlescape::weapon_panel) struct WeaponWidgets<'w, 's> {
    /// The content column's visibility + layout `Node` — revealed when a weapon is selected,
    /// else HIDDEN via `Display::None` (removed from layout, GTW-295) AND `Visibility::Hidden`.
    content:  Query<'w, 's, (&'static mut Visibility, &'static mut Node), ContentFilter>,
    /// The weapon-name text line.
    name:     Query<'w, 's, &'static mut Text, NameFilter>,
    /// The magazine `"cur/max"` text line + its visibility (shown only with `size > 0`).
    magazine: Query<'w, 's, (&'static mut Text, &'static mut Visibility), MagazineFilter>,
    /// The LIVE Reload button's visibility — shown only when the weapon has a magazine.
    reload:   Query<'w, 's, &'static mut Visibility, ReloadFilter>,
}

/// Write `text` into a [`Text`] node, mutating in place (no despawn / respawn).
///
/// [`Text`] wraps a [`String`]; `clone_into` reuses the existing allocation
/// ([[ui-mutate-not-respawn]] — the stat-block text-mutate precedent).
fn set_text(text: &mut Text, value: &str) {
    value.clone_into(&mut text.0);
}

/// Set a [`Visibility`] to `visible` ? [`Visibility::Inherited`] : [`Visibility::Hidden`],
/// writing only on a real change so an idempotent rewrite does not trip change detection.
fn set_visible(visibility: &mut Visibility, visible: bool) {
    let want = if visible {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if *visibility != want {
        *visibility = want;
    }
}

/// Show / hide the content column by BOTH its layout [`Display`] and its [`Visibility`]
/// (GTW-295): when hidden the column is [`Display::None`] (removed from layout, so it takes
/// NO space — the AC2 mechanism) AND [`Visibility::Hidden`] (the existing GTW-275 contract);
/// when shown it is [`Display::Flex`] + [`Visibility::Inherited`]. Each write is gated on a
/// real change ([[ui-mutate-not-respawn]]).
fn set_content_shown(visibility: &mut Visibility, node: &mut Node, shown: bool) {
    set_visible(visibility, shown);
    let want = if shown { Display::Flex } else { Display::None };
    if node.display != want {
        node.display = want;
    }
}

/// Repaints the weapon panel from the current [`SelectedShooter`].
///
/// Resolves the selected ganger's WIELDED WEAPON ENTITY (`ganger → `[`Wields`]` → the
/// weapon entity`, GTW-323 slice 3) and reads its [`WeaponData`] (ONE `.get` over the
/// read-only weapon-entity query) and:
///
/// - **No selection / wields no weapon** → the [`WeaponContent`] column is hidden (AC9
///   empty state).
/// - **A weapon** → the content is shown; the [`WeaponNameText`] is set to the
///   [`WeaponName`] (AC5); and the magazine line + the LIVE [`ReloadButton`] are shown
///   with `"cur/max"` ONLY when the weapon has a magazine (`size > 0`), else hidden
///   (AC5 / AC6 — a weapon with no magazine has no reload).
///
/// Mutates in place ([[ui-mutate-not-respawn]]); never panics on a missing widget /
/// component (a `let else` / `Option` everywhere — AC9). Param-only (`bevy-traps.md` #7):
/// [`Res<SelectedShooter>`] + the read-only `Query<&Wields>` (the relationship) + the
/// read-only [`WeaponData`] weapon-entity query + a [`MeleeWeapon`] marker probe (GTW-505 C5
/// — so the panel shows the RANGED weapon, never the melee one the ganger also wields) + the
/// [`WeaponWidgets`] write bundle.
pub(in crate::states::running::game::battlescape) fn update_weapon_panel(
    selected: Res<SelectedShooter>,
    wields: Query<&Wields>,
    data: Query<WeaponData, With<WieldedBy>>,
    melee: Query<(), With<MeleeWeapon>>,
    mut widgets: WeaponWidgets,
) {
    // Resolve `ganger → Wields → the RANGED weapon entity`, then read the weapon's WeaponData
    // off that entity. GTW-505 C5: a ganger wields BOTH a ranged AND a melee weapon, so resolve
    // through `Wields::ranged_weapon` (excluding the `MeleeWeapon`-marked entity) — NOT
    // `Wields::weapon` (the spawn-order fragile first entity) — so the panel never shows the
    // melee weapon's name/magazine. No selection, no Wields, or no ranged weapon → no weapon.
    let selection = (**selected)
        .and_then(|ganger| wields.get(ganger).ok())
        .and_then(|w| w.ranged_weapon(|entity| melee.get(entity).is_ok()))
        .and_then(|weapon| data.get(weapon).ok());

    // The weapon name (None when unarmed / no selection) and whether the weapon has a
    // magazine (size > 0) with its cur/max string.
    let (has_weapon, name, mag_line, has_magazine) = match selection {
        Some(item) => {
            let has_weapon = item.name.is_some() || item.magazine.is_some();
            let name = item.name.map(|n| (**n).clone()).unwrap_or_default();
            let (mag_line, has_magazine) = item.magazine.map_or((String::new(), false), |m| {
                let size = *m.size();
                if size > 0 {
                    (format!("{}/{}", *m.rounds(), size), true)
                } else {
                    (String::new(), false)
                }
            });
            (has_weapon, name, mag_line, has_magazine)
        }
        None => (false, String::new(), String::new(), false),
    };

    // The content column: shown only when a weapon is selected (AC9). Hidden via
    // `Display::None` (no layout space) + `Visibility::Hidden` (GTW-295 / GTW-275).
    if let Ok((mut visibility, mut node)) = widgets.content.single_mut() {
        set_content_shown(&mut visibility, &mut node, has_weapon);
    }

    // The weapon name text (mutated in place; blank when unarmed).
    if let Ok(mut text) = widgets.name.single_mut() {
        set_text(&mut text, &name);
    }

    // The magazine cur/max text + its visibility (shown only with size > 0).
    if let Ok((mut text, mut visibility)) = widgets.magazine.single_mut() {
        set_text(&mut text, &mag_line);
        set_visible(&mut visibility, has_magazine);
    }

    // The LIVE Reload button: shown only when the weapon has a magazine (AC6).
    if let Ok(mut visibility) = widgets.reload.single_mut() {
        set_visible(&mut visibility, has_magazine);
    }
}
