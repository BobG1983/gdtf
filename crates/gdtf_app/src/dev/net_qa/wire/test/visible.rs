use super::assert_ron_round_trip;
use crate::dev::net_qa::wire::{
    cell::{CellLevelNet, CellNet, CellXNet, CellYNet, LevelNet},
    inspect::{CoverBlockNet, CoverHpNet, HardnessNet, HeightBandNet, ProtectionNet},
    token::{DoorToken, GangerToken},
    visible::{DoorOpenNet, VisibleCoverNet, VisibleDoorNet, VisibleGangerNet},
};

fn a_cell() -> CellLevelNet {
    CellLevelNet::new(
        CellNet::new(CellXNet::new(4), CellYNet::new(-2)),
        LevelNet::new(1),
    )
}

#[test]
fn door_flags_round_trip() {
    assert_ron_round_trip(&DoorOpenNet::new(true));
    assert_ron_round_trip(&DoorOpenNet::new(false));
}

#[test]
fn lit_area_entries_round_trip() {
    let ganger: VisibleGangerNet = VisibleGangerNet::new(GangerToken::new(12), a_cell());
    let door: VisibleDoorNet =
        VisibleDoorNet::new(DoorToken::new(13), a_cell(), DoorOpenNet::new(true));
    let cover: VisibleCoverNet = VisibleCoverNet::new(
        a_cell(),
        CoverBlockNet {
            hardness:   HardnessNet::new(1),
            protection: ProtectionNet::new(2),
            height:     HeightBandNet::Low,
            hp:         CoverHpNet::new(3),
            hp_max:     CoverHpNet::new(4),
        },
    );
    assert_ron_round_trip(&ganger);
    assert_ron_round_trip(&door);
    assert_ron_round_trip(&cover);
}

#[test]
fn a_visible_ganger_refuses_an_unknown_field() {
    let hostile = "(token:1,at:(cell:(x:0,y:0),level:0),extra:2)";
    assert!(
        ron::de::from_str::<VisibleGangerNet>(hostile).is_err(),
        "`{hostile}` carries an unknown field and must not decode",
    );
}
