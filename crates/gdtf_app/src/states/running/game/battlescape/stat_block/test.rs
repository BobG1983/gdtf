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

#[test]
fn portrait_index_is_deterministic_for_a_name() {
    let name = GangerName::new("Vex Harker".to_owned());
    let a = PortraitIndex::for_name(Some(&name));
    let b = PortraitIndex::for_name(Some(&name));
    assert_eq!(*a, *b, "the same name must map to the same portrait face");
}

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

#[test]
fn portrait_index_falls_back_to_zero_when_nameless() {
    assert_eq!(*PortraitIndex::for_name(None), 0, "no name -> face 0");
}

#[test]
fn name_label_shows_name_or_fallback() {
    let name = GangerName::new("Alex Mercer".to_owned());
    assert!(name_label(Some(&name)).contains("Alex Mercer"));
    assert_eq!(name_label(None), NAMELESS);
}

#[test]
fn faction_label_shows_the_gang_index() {
    assert!(faction_label(Faction::new(1)).contains("Gang 1"));
}

#[test]
fn stance_label_renders_each_posture() {
    assert!(stance_label(Stance::new(StanceKind::Standing)).contains("Standing"));
    assert!(stance_label(Stance::new(StanceKind::Crouching)).contains("Crouching"));
    assert!(stance_label(Stance::new(StanceKind::Prone)).contains("Prone"));
}

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


#[derive(bevy::prelude::Resource)]
struct ProbeRefs(super::components::StatBlockRefs);

fn probe_stat_block(
    query: bevy::prelude::Query<super::update::StatBlockData>,
    refs: bevy::prelude::Res<ProbeRefs>,
    mut widgets: super::update::StatBlockWidgets,
) {
    for data in &query {
        super::update::update_stat_block(refs.0, &data, &mut widgets);
    }
}

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
