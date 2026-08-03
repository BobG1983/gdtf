use bevy::scene::{Scene, SceneList, bsn, bsn_list, template_value};

use crate::armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorSpec, BodyPart,
};

pub(super) fn worn_piece_scenes(spec: &ArmorSpec) -> impl SceneList {
    let pieces = spec.pieces();
    bsn_list! {
        worn_piece_scene(BodyPart::Head, pieces[BodyPart::Head.index()]),
        worn_piece_scene(BodyPart::Torso, pieces[BodyPart::Torso.index()]),
        worn_piece_scene(BodyPart::LeftArm, pieces[BodyPart::LeftArm.index()]),
        worn_piece_scene(BodyPart::RightArm, pieces[BodyPart::RightArm.index()]),
        worn_piece_scene(BodyPart::LeftLeg, pieces[BodyPart::LeftLeg.index()]),
        worn_piece_scene(BodyPart::RightLeg, pieces[BodyPart::RightLeg.index()]),
    }
}

fn worn_piece_scene(part: BodyPart, piece: crate::armor::ArmorPiece) -> impl Scene {
    let floor = *piece.floor;
    let protection = *piece.protection;
    let integrity = *piece.integrity;
    let hardness = *piece.hardness;
    let armor_type = piece.armor_type;
    (
        bsn! {
            ArmorFloor::new(floor)
            ArmorProtection::new(protection)
            ArmorIntegrity::new(integrity)
            ArmorHardness::new(hardness)
        },
        template_value(part),
        template_value(armor_type),
    )
}
