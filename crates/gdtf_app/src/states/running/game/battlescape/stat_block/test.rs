//! In-crate unit tests for the stat block's pure helpers (GTW-278): the deterministic
//! portrait derivation and the name / faction / stance / wound-name format helpers.
//!
//! The widget-wiring (spawn + mutate on the real flow) is covered by the headless
//! integration tests in `crates/gdtf_app/tests/stat_panels.rs`.

use gdtf_battle_sim::{
    armor::BodyPart,
    ganger::GangerName,
    inflicted_wound::InflictedWound,
    prelude::{Faction, Stance, StanceKind},
    severity::Severity,
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

// ── GTW-727 T18: the block shows CURSOR TIME ────────────────────────────────────

/// The widget ids one probe run writes into — a [`Resource`] so the test system can reach
/// them (a system cannot take a plain argument).
#[derive(bevy::prelude::Resource)]
struct ProbeRefs(super::components::StatBlockRefs);

/// Run the REAL [`update_stat_block`](super::update::update_stat_block) over every ganger
/// in the world, writing into the fixture's widget ids.
fn probe_stat_block(
    query: bevy::prelude::Query<super::update::StatBlockData>,
    refs: bevy::prelude::Res<ProbeRefs>,
    mut widgets: super::update::StatBlockWidgets,
) {
    for data in &query {
        super::update::update_stat_block(refs.0, &data, &mut widgets);
    }
}

/// **T18 — CLAUSE (b) THROUGH THE REAL PANEL.** The stat block must show the HP the
/// presenter has SHOWN, not the HP the sim has already reached.
///
/// This is the reported symptom stated as an assertion: in the filed repro the HUD already
/// read the reduced HP while the bolt that caused it was still mid-flight. The drawn mirror
/// is what fixes it, and the fallback keeps every pre-existing behaviour intact — with no
/// mirror present the block reads live state exactly as it always did, which is why no
/// existing stat-block test had to change.
#[test]
fn the_stat_block_shows_drawn_hp_while_a_wound_is_unshown() {
    use bevy::{ecs::system::RunSystemOnce, prelude::*};
    use gdtf_battle_presenter::DrawnVitals;
    use gdtf_battle_sim::{
        act_log::VitalsFacts,
        ganger::{Faction, Hp, HpMax, Stance, StanceKind, Tu, TuMax, Wounds},
        inflicted_wound::InflictedWounds,
        injuries::InflictedInjuries,
    };

    let mut app = App::new();
    // One real Text widget for the HP label; every other ref points at a spare entity, so
    // the other writes are harmless no-ops and this test pins exactly one readout.
    let hp_label = app.world_mut().spawn(Text::default()).id();
    let spare = app.world_mut().spawn_empty().id();
    let refs = super::components::StatBlockRefs {
        portrait: spare,
        name: spare,
        faction: spare,
        stance: spare,
        tu_bar: spare,
        tu_label: spare,
        hp_bar: spare,
        hp_label,
        wounds: spare,
        wound_list: spare,
        injury_list: spare,
    };
    app.insert_resource(ProbeRefs(refs));

    // A ganger the SIM has already wounded to 2 HP, while the presenter is still SHOWING
    // the 9 HP it had before the round played.
    let ganger = app
        .world_mut()
        .spawn((
            Faction::new(0),
            Stance::new(StanceKind::Standing),
            Tu::new(5),
            TuMax::new(10),
            Hp::new(2),
            HpMax::new(10),
            Wounds::new(3),
            DrawnVitals::new(VitalsFacts::new(
                Tu::new(5),
                Hp::new(9),
                Wounds::new(3),
                InflictedWounds::default(),
                InflictedInjuries::default(),
            )),
        ))
        .id();

    let ran = app.world_mut().run_system_once(probe_stat_block);
    assert!(ran.is_ok(), "the probe must run");
    let shown = app
        .world()
        .get::<Text>(hp_label)
        .map(|text| text.0.clone())
        .unwrap_or_default();
    assert_eq!(
        shown, "9/10",
        "the block must read the DRAWN hit points while the wound is still unshown — \
         reading live state here is the reported defect: the HP dropping before the bolt \
         that caused it has landed",
    );

    // Remove the mirror: with no presenter the block falls back to live state, unchanged
    // from every build before this one.
    app.world_mut().entity_mut(ganger).remove::<DrawnVitals>();
    let ran = app.world_mut().run_system_once(probe_stat_block);
    assert!(ran.is_ok(), "the probe must run again");
    let live = app
        .world()
        .get::<Text>(hp_label)
        .map(|text| text.0.clone())
        .unwrap_or_default();
    assert_eq!(
        live, "2/10",
        "with no drawn mirror the block reads live state — the fallback that keeps a \
         presenter-less app (and every pre-existing test) behaving exactly as before",
    );
}
