//! The lists that draw no set-at button, and the field arm each refusal sends a client to
//! instead.

use gdtf_content_editor::EditorMode;

use crate::{
    bad_arguments::bad_arguments_detail,
    setup::{form_tab_app_and_client, try_list_op},
    support::TestResult,
};

// Each list that refuses set-at, with every field arm its detail must spell.
const REFUSALS: [(EditorMode, &str, &[&str]); 4] = [
    (
        EditorMode::Attachment,
        "(list: AttachmentEffects, op: SetAt(0, AttachmentEffect(Silence)))",
        &["`Attachment(Effect(index: n, effect: …))`"],
    ),
    (
        EditorMode::Injury,
        "(list: InjuryEffects, op: SetAt(0, InjuryEffect(DisableHand)))",
        &["`Injury(Effect(index: n, effect: …))`"],
    ),
    (
        EditorMode::Sprite,
        "(list: SpriteFrames, op: SetAt(0, SpriteFrame(File(\"sprites/a.png\"))))",
        &["`Sprite(Frame(index: n, source: …))`"],
    ),
    (
        EditorMode::Gang,
        "(list: GangMembers, op: SetAt(0, GangMember(\"scab\")))",
        &[
            "`Gang(MemberName(…))`",
            "`Gang(MemberAttribute(…))`",
            "`Gang(MemberWeapon(…))`",
            "`Gang(MemberArmor(…))`",
            "`Gang(MemberMeleeWeapon(…))`",
        ],
    ),
];

#[test]
fn a_set_at_refusal_names_the_field_arm_a_client_sends_instead() -> TestResult {
    for (mode, arguments, arms) in REFUSALS {
        let (mut app, mut client) = form_tab_app_and_client(mode)?;

        let reply = try_list_op(&mut app, &mut client, arguments)?;

        let detail = bad_arguments_detail(&reply)?;
        for wanted in arms {
            assert!(
                detail.contains(wanted),
                "the {mode:?} set-at refusal must spell {wanted}, one of the field arms that do \
                 the job, got `{detail}`",
            );
        }
    }
    Ok(())
}
