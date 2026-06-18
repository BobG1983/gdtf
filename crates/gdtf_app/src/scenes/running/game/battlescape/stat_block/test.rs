//! In-crate unit tests for the stat block's pure helpers (GTW-278): the deterministic
//! portrait derivation and the name / faction / stance / wound-name format helpers.
//!
//! The widget-wiring (spawn + mutate on the real flow) is covered by the headless
//! integration tests in `crates/gdtf_app/tests/stat_panels.rs`.

use gdtf_battle_sim::{
    BodyPart, Faction, GangerName, InflictedWound, Severity, Stance, StanceKind,
};

use super::{
    labels::{NAMELESS, faction_label, name_label, stance_label, wound_label},
    portrait::PortraitIndex,
};

/// The portrait index is DETERMINISTIC: the SAME name maps to the SAME face across two
/// independent computations (the fixed-seed-hash contract — a `RandomState` hasher would
/// fail this, since its seed differs per call).
#[test]
fn portrait_index_is_deterministic_for_a_name() {
    let name = GangerName::new("Vex Harker".to_owned());
    let a = PortraitIndex::for_name(Some(&name));
    let b = PortraitIndex::for_name(Some(&name));
    assert_eq!(*a, *b, "the same name must map to the same portrait face");
}

/// Different names generally map to different faces (discriminating: a derivation that
/// ignored the name would collide here). Two names with distinct hashes are chosen.
#[test]
fn portrait_index_varies_by_name() {
    let alex = GangerName::new("Alex Mercer".to_owned());
    let vex = GangerName::new("Vex Harker".to_owned());
    assert_ne!(
        *PortraitIndex::for_name(Some(&alex)),
        *PortraitIndex::for_name(Some(&vex)),
        "distinct names should map to distinct portrait faces",
    );
}

/// The portrait index is always in range (`0..100`) — the `% PORTRAIT_COUNT` guarantee,
/// so a constructed index can always address the 10×10 atlas.
#[test]
fn portrait_index_is_in_range() {
    for name in [
        "Alex Mercer",
        "Vex Harker",
        "",
        "a very long ganger name indeed",
    ] {
        let name = GangerName::new(name.to_owned());
        assert!(
            *PortraitIndex::for_name(Some(&name)) < 100,
            "portrait index must be in 0..100",
        );
    }
}

/// A nameless ganger falls back to face 0 (the contract's no-name rule).
#[test]
fn portrait_index_falls_back_to_zero_when_nameless() {
    assert_eq!(*PortraitIndex::for_name(None), 0, "no name -> face 0");
}

/// The name title shows the ganger's name; a nameless ganger shows the `NAMELESS`
/// fallback.
#[test]
fn name_label_shows_name_or_fallback() {
    let name = GangerName::new("Alex Mercer".to_owned());
    assert!(name_label(Some(&name)).contains("Alex Mercer"));
    assert_eq!(name_label(None), NAMELESS);
}

/// The faction line shows the gang index.
#[test]
fn faction_label_shows_the_gang_index() {
    assert!(faction_label(Faction::new(1)).contains("Gang 1"));
}

/// Each stance posture renders its own distinct word.
#[test]
fn stance_label_renders_each_posture() {
    assert!(stance_label(Stance::new(StanceKind::Standing)).contains("Standing"));
    assert!(stance_label(Stance::new(StanceKind::Crouching)).contains("Crouching"));
    assert!(stance_label(Stance::new(StanceKind::Prone)).contains("Prone"));
}

/// A wound renders as `"{tier} — {location}"` with the named vocabulary
/// (e.g. "Minor — Left Arm") — discriminating: a mis-mapped tier or part would surface
/// the wrong word.
#[test]
fn wound_label_renders_tier_and_location() {
    let wound = InflictedWound::new(Severity::Minor, BodyPart::LeftArm);
    let label = wound_label(wound);
    assert!(label.contains("Minor"), "tier word: {label}");
    assert!(label.contains("Left Arm"), "location word: {label}");

    let critical = wound_label(InflictedWound::new(Severity::Critical, BodyPart::Head));
    assert!(critical.contains("Critical"), "tier word: {critical}");
    assert!(critical.contains("Head"), "location word: {critical}");
}
