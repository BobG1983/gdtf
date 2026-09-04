use super::assert_ron_round_trip;
use crate::dev::mcp::wire::{
    act_payload::{AimNet, FacingNet, MeleeTargetNet, StanceNet},
    cell::{CellLevelNet, CellNet, CellXNet, CellYNet, LevelNet},
    cost::{CostActNet, CostLegalNet, CostRefusalNet},
    misc::ModeKindNet,
    token::{DoorToken, EmplacementToken, GangerToken},
};

fn at(x: i32, y: i32, level: u8) -> CellLevelNet {
    CellLevelNet::new(
        CellNet::new(CellXNet::new(x), CellYNet::new(y)),
        LevelNet::new(level),
    )
}

#[test]
fn every_cost_act_round_trips() {
    assert_ron_round_trip(&CostActNet::Move { dest: at(4, 9, 1) });
    assert_ron_round_trip(&CostActNet::Fire {
        target: at(12, 3, 0),
        mode:   ModeKindNet::Burst,
    });
    assert_ron_round_trip(&CostActNet::Reload);
    assert_ron_round_trip(&CostActNet::SetStance {
        stance: StanceNet::Prone,
    });
    assert_ron_round_trip(&CostActNet::SetFacing {
        facing: FacingNet::SouthWest,
    });
    assert_ron_round_trip(&CostActNet::SetAiming {
        aim: AimNet::new(true),
    });
    assert_ron_round_trip(&CostActNet::Shove {
        target: GangerToken::new(7),
    });
    assert_ron_round_trip(&CostActNet::OpenDoor {
        target: DoorToken::new(11),
    });
    assert_ron_round_trip(&CostActNet::EnterEmplacement {
        target: EmplacementToken::new(13),
    });
    assert_ron_round_trip(&CostActNet::ExitEmplacement {
        target: EmplacementToken::new(13),
    });
    assert_ron_round_trip(&CostActNet::ThrowGrenade {
        target: at(2, 2, 2),
    });
    assert_ron_round_trip(&CostActNet::Melee {
        target: MeleeTargetNet::Ganger(GangerToken::new(5)),
    });
    assert_ron_round_trip(&CostActNet::Melee {
        target: MeleeTargetNet::Structure(at(1, 1, 0)),
    });
}

#[test]
fn cost_legality_and_refusals_round_trip() {
    assert_ron_round_trip(&CostLegalNet::new(true));
    assert_ron_round_trip(&CostLegalNet::new(false));
    for refusal in [
        CostRefusalNet::NoSuchGanger,
        CostRefusalNet::NotYourGanger,
        CostRefusalNet::NoPathToCell,
        CostRefusalNet::Suppressed,
        CostRefusalNet::NoLineOfSight,
        CostRefusalNet::CannotAfford,
        CostRefusalNet::ActNotAllowed,
    ] {
        assert_ron_round_trip(&refusal);
    }
}

#[test]
fn legality_is_false_exactly_when_a_refusal_is_present() {
    assert!(
        *CostLegalNet::from_refusal(None),
        "an act nothing refused is legal",
    );
    for refusal in [
        CostRefusalNet::NoSuchGanger,
        CostRefusalNet::NotYourGanger,
        CostRefusalNet::NoPathToCell,
        CostRefusalNet::Suppressed,
        CostRefusalNet::NoLineOfSight,
        CostRefusalNet::CannotAfford,
        CostRefusalNet::ActNotAllowed,
    ] {
        assert!(
            !*CostLegalNet::from_refusal(Some(refusal)),
            "{refusal:?} must make the act illegal",
        );
    }
}
