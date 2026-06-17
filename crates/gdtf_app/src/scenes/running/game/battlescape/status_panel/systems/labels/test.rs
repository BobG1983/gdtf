//! Unit tests for the pure status-panel format helpers (relocated from the inline
//! `labels` test module, GTW-201).

use gdtf_battle_sim::{
    Cell, CellLevel, Faction, Hp, Level, LifeState, Position, Stance, StanceKind, Tu, TuMax,
    WeaponName, Wounds,
};

use super::format::{
    UNARMED, hp_label, identity_label, life_label, stance_label, tu_label, weapon_name_label,
};

/// The identity line names the ganger's cell coordinates, storey, and faction —
/// each component's value appears, so swapping x↔y or faction would be visible.
#[test]
fn identity_label_names_cell_and_faction() {
    let position = Position::new(CellLevel::new(Cell::new(5, 6), Level::new(2)));
    let faction = Faction::new(1);
    let label = identity_label(&position, faction);
    assert!(
        label.contains('5'),
        "identity must show the cell x: {label}"
    );
    assert!(
        label.contains('6'),
        "identity must show the cell y: {label}"
    );
    assert!(
        label.contains('2'),
        "identity must show the storey level: {label}",
    );
    assert!(
        label.contains("Gang 1"),
        "identity must show the faction index: {label}",
    );
}

/// Each stance posture renders its own distinct word — discriminating: a
/// mis-wired posture would surface the wrong word here.
#[test]
fn stance_label_renders_each_posture() {
    assert!(stance_label(Stance::new(StanceKind::Standing)).contains("Standing"));
    assert!(stance_label(Stance::new(StanceKind::Crouching)).contains("Crouching"));
    assert!(stance_label(Stance::new(StanceKind::Prone)).contains("Prone"));
}

/// The TU line renders the current pool over the max as `cur/max`.
#[test]
fn tu_label_is_current_over_max() {
    let label = tu_label(Tu::new(7), TuMax::new(10));
    assert!(
        label.contains("7/10"),
        "TU line must read current/max: {label}",
    );
}

/// A spent-down TU pool renders the new current, never the max — guards against
/// a line wired to the wrong component (max where current was meant).
#[test]
fn tu_label_reflects_spent_pool() {
    let full = tu_label(Tu::new(10), TuMax::new(10));
    let spent = tu_label(Tu::new(3), TuMax::new(10));
    assert!(full.contains("10/10"), "full pool: {full}");
    assert!(spent.contains("3/10"), "spent pool: {spent}");
    assert_ne!(full, spent, "spending TU must change the rendered line");
}

/// The HP line shows the current HP and the Wounds count — no fabricated max.
#[test]
fn hp_label_shows_current_hp_and_wounds() {
    let label = hp_label(Hp::new(8), Wounds::new(3));
    assert!(label.contains('8'), "HP line must show current HP: {label}");
    assert!(
        label.contains('3'),
        "HP line must show the Wounds count: {label}",
    );
}

/// Each life state renders its own distinct word.
#[test]
fn life_label_renders_each_state() {
    assert!(life_label(LifeState::Alive).contains("Alive"));
    assert!(life_label(LifeState::Downed).contains("Downed"));
    assert!(life_label(LifeState::Dead).contains("Dead"));
}

/// AC1 — an ARMED ganger's weapon line names its `WeaponName` (the rendered string
/// comes from the real component, not a fixture); a different name would surface here.
#[test]
fn weapon_name_label_names_an_armed_weapon() {
    let weapon = WeaponName::new("autogun".to_owned());
    let label = weapon_name_label(Some(&weapon));
    assert!(
        label.contains("autogun"),
        "the weapon line must name the carried weapon: {label}",
    );
}

/// AC1 — an UNARMED selection (no `WeaponName`) renders the unarmed fallback, NOT a
/// fabricated name and NOT a panic.
#[test]
fn weapon_name_label_falls_back_when_unarmed() {
    let label = weapon_name_label(None);
    assert!(
        label.contains(UNARMED),
        "an unarmed selection must show the unarmed fallback: {label}",
    );
}
