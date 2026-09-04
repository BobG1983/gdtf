use gdtf_battle_sim::{
    effects::fields::FieldKey,
    ganger::{Faction, GangName, GangerName},
    metric::{Cell, CellLevel, Level},
    situation::{FieldSpawn, RosterMember, Situation},
};

use super::super::write_situation_in;

// A situation carrying two roster members and two field spawns.
fn fixture_situation() -> Situation {
    let mut situation = Situation::new();
    situation.rosters = vec![
        RosterMember::new(
            GangName::new("scav_pack".to_owned()),
            GangerName::new("Grist".to_owned()),
            Faction::new(0),
        ),
        RosterMember::new(
            GangName::new("iron_creed".to_owned()),
            GangerName::new("Vane".to_owned()),
            Faction::new(1),
        ),
    ];
    situation.fields = vec![
        FieldSpawn::new(
            CellLevel::new(Cell::new(1, 1), Level::new(0)),
            FieldKey::new("toxic_waste_pool".to_owned()),
        ),
        FieldSpawn::new(
            CellLevel::new(Cell::new(4, 2), Level::new(1)),
            FieldKey::new("incendiary_fire".to_owned()),
        ),
    ];
    situation
}

#[test]
fn a_written_situation_reads_back_with_every_roster_member_and_field_spawn() {
    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "creating the TempDir assets root must succeed");
    let Ok(dir) = dir else { return };

    let situation = fixture_situation();
    let written = write_situation_in(dir.path(), &situation);
    assert!(
        written.is_ok(),
        "the real situation write must succeed: {:?}",
        written.as_ref().err(),
    );
    let Ok(path) = written else { return };

    let text = std::fs::read_to_string(&path);
    assert!(text.is_ok(), "the writer left a readable file at {path:?}");
    let Ok(text) = text else { return };

    let reloaded = ron::de::from_str::<Situation>(&text);
    assert!(
        reloaded.is_ok(),
        "the written file must parse back as a Situation: {:?}",
        reloaded.as_ref().err(),
    );
    let Ok(reloaded) = reloaded else { return };

    assert_eq!(
        reloaded.rosters, situation.rosters,
        "a writer that serializes only the list it is editing loses the rosters",
    );
    assert_eq!(
        reloaded.fields, situation.fields,
        "both field spawns must come back, in the order they were authored",
    );
}

#[test]
fn the_situation_writer_never_reaches_the_shipped_tree() {
    let source = include_str!("../../save.rs");
    assert!(
        !source.contains("workspace_assets_root"),
        "a situation writer takes an explicit root: reaching workspace_assets_root would write \
         the shipped assets/content/situations/skirmish.ron from an editor session",
    );
}
