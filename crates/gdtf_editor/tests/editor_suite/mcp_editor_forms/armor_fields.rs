use bevy::app::App;
use gdtf_battle_sim::armor::{ArmorPiece, ArmorType, BodyPart};
use gdtf_editor::{ArmorDraft, EditorMode};

use crate::{
    mcp_editor_forms::{
        rows::{ArmorFieldRow, FieldRow, SetFieldReplyRow},
        setup::{armor_draft, form_tab_app_and_client, set_field},
        values::{ArmorTypeRow, BodyPartRow},
    },
    mcp_shared::{
        socket::Client,
        support::{TestError, TestResult},
    },
};

// The piece the draft holds for one part, read off the spec the save would write.
const fn piece(draft: &ArmorDraft, part: BodyPart) -> ArmorPiece {
    let spec = draft.spec();
    match part {
        BodyPart::Head => spec.head,
        BodyPart::Torso => spec.torso,
        BodyPart::LeftArm => spec.left_arm,
        BodyPart::RightArm => spec.right_arm,
        BodyPart::LeftLeg => spec.left_leg,
        BodyPart::RightLeg => spec.right_leg,
    }
}

const fn part_row(part: BodyPart) -> BodyPartRow {
    match part {
        BodyPart::Head => BodyPartRow::Head,
        BodyPart::Torso => BodyPartRow::Torso,
        BodyPart::LeftArm => BodyPartRow::LeftArm,
        BodyPart::RightArm => BodyPartRow::RightArm,
        BodyPart::LeftLeg => BodyPartRow::LeftLeg,
        BodyPart::RightLeg => BodyPartRow::RightLeg,
    }
}

// A value no other part of the same case writes, so a mis-wired arm cannot pass.
const fn stat_for(part: BodyPart) -> i32 {
    match part {
        BodyPart::Head => 1,
        BodyPart::Torso => 2,
        BodyPart::LeftArm => 3,
        BodyPart::RightArm => 4,
        BodyPart::LeftLeg => 5,
        BodyPart::RightLeg => 6,
    }
}

fn write_stat(
    app: &mut App,
    client: &mut Client,
    arm: &str,
    part: BodyPart,
    value: i32,
) -> Result<SetFieldReplyRow, TestError> {
    let arguments = format!("(field: Armor({arm}(part: {part:?}, value: {value})))");
    set_field(app, client, &arguments)
}

#[test]
fn every_armor_field_arm_writes_its_own_piece_and_stat() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Armor)?;

    let named = set_field(&mut app, &mut client, "(field: Armor(Name(\"flak vest\")))")?;
    assert_eq!(
        named.field,
        FieldRow::Armor(ArmorFieldRow::Name("flak vest".to_owned())),
        "the reply reads the name back off the draft",
    );
    assert_eq!(
        armor_draft(&app)?.name(),
        "flak vest",
        "the world's own Armor draft holds the name on the frame that answered",
    );

    for part in BodyPart::ALL {
        let value = stat_for(part);
        let row = part_row(part);
        let floor = write_stat(&mut app, &mut client, "Floor", part, value)?;
        assert_eq!(
            floor.field,
            FieldRow::Armor(ArmorFieldRow::Floor { part: row, value }),
            "the reply names the piece and the stat it wrote",
        );
        write_stat(&mut app, &mut client, "Protection", part, value + 10)?;
        write_stat(&mut app, &mut client, "Hardness", part, value + 20)?;
        write_stat(&mut app, &mut client, "Integrity", part, value + 500)?;

        let arguments = format!("(field: Armor(Type(part: {part:?}, value: Ceramic)))");
        let typed = set_field(&mut app, &mut client, &arguments)?;
        assert_eq!(
            typed.field,
            FieldRow::Armor(ArmorFieldRow::Type {
                part:  row,
                value: ArmorTypeRow::Ceramic,
            }),
            "the reply names the piece and the material it wrote",
        );
    }

    let draft = armor_draft(&app)?;
    for part in BodyPart::ALL {
        let value = stat_for(part);
        let held = piece(&draft, part);
        assert_eq!(
            (
                *held.floor,
                *held.protection,
                *held.hardness,
                *held.integrity
            ),
            (value, value + 10, value + 20, value + 500),
            "each {part:?} stat holds the value its own arm wrote, so an arm wired to another \
             piece or another stat fails here",
        );
        assert_eq!(
            held.armor_type,
            ArmorType::Ceramic,
            "the {part:?} material holds what its arm wrote",
        );
    }
    Ok(())
}
