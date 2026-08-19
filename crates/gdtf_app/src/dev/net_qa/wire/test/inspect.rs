use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, HeightBand},
    entity::TerrainPieceKind,
};

use super::{assert_ron_round_trip, roster::a_card};
use crate::dev::net_qa::wire::inspect::{
    CoverBlockNet, CoverHpNet, HardnessNet, HeightBandNet, InspectShownNet, ProtectionNet,
};

fn a_block() -> CoverBlockNet {
    CoverBlockNet {
        hardness:   HardnessNet::new(2),
        protection: ProtectionNet::new(-1),
        height:     HeightBandNet::Mid,
        hp:         CoverHpNet::new(4),
        hp_max:     CoverHpNet::new(10),
    }
}

#[test]
fn cover_scalars_round_trip() {
    assert_ron_round_trip(&HardnessNet::new(-3));
    assert_ron_round_trip(&ProtectionNet::new(7));
    assert_ron_round_trip(&CoverHpNet::new(0));
    for band in [HeightBandNet::Low, HeightBandNet::Mid, HeightBandNet::High] {
        match band {
            HeightBandNet::Low | HeightBandNet::Mid | HeightBandNet::High => {}
        }
        assert_ron_round_trip(&band);
    }
}

#[test]
fn a_cover_block_round_trips() {
    let block: CoverBlockNet = a_block();
    assert_ron_round_trip(&block);
}

#[test]
fn every_inspect_outcome_round_trips() {
    assert_ron_round_trip(&InspectShownNet::Ganger(a_card()));
    assert_ron_round_trip(&InspectShownNet::Cover(a_block()));
    assert_ron_round_trip(&InspectShownNet::Nothing);
}

#[test]
fn a_sim_cover_entry_mirrors_onto_the_wire() {
    let entry = CoverEntry::seeded(
        CoverHp::new(9),
        HeightBand::High,
        ArmorProtection::new(3),
        ArmorHardness::new(5),
        TerrainPieceKind::Wall,
    );
    let mirrored = CoverBlockNet::from_sim(entry);
    assert_eq!(
        mirrored,
        CoverBlockNet {
            hardness:   HardnessNet::new(5),
            protection: ProtectionNet::new(3),
            height:     HeightBandNet::High,
            hp:         CoverHpNet::new(9),
            hp_max:     CoverHpNet::new(9),
        },
        "a freshly seeded entry mirrors at full HP with its own hardness and protection",
    );
}
