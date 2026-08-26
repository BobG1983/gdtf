use super::super::{apply, members};
use crate::{
    gang_form::GangDraft,
    net_qa::{
        commands::write::form_fault::FormWriteFault,
        wire::{EditorListIndexNet, EditorListMemberNet, EditorListOpNet, TerrainFacingNet},
    },
};

// Which refusal an operation answered and the line it carries, or that it ran instead.
fn refusal(result: Result<(), FormWriteFault>) -> (&'static str, String) {
    match result {
        Ok(()) => unreachable!("expected a refusal, and the operation ran"),
        Err(FormWriteFault::ForeignArm) => ("ForeignArm", String::new()),
        Err(FormWriteFault::Gated(note)) => ("Gated", note.as_str().to_owned()),
        Err(FormWriteFault::MissingModel(note)) => ("MissingModel", note.as_str().to_owned()),
        Err(FormWriteFault::BadArguments(detail)) => ("BadArguments", detail.as_str().to_owned()),
    }
}

#[test]
fn an_add_then_a_remove_leaves_the_draft_it_started_from() {
    let mut draft = GangDraft::new_gang();
    let before = draft.clone();

    let added = apply(&mut draft, EditorListOpNet::Add);
    assert!(added.is_ok(), "the roster's Add button appends a member");
    assert_eq!(draft.members().len(), 1);

    let removed = apply(
        &mut draft,
        EditorListOpNet::Remove(EditorListIndexNet::new(0)),
    );
    assert!(removed.is_ok(), "the row's own Remove button takes it off");
    assert_eq!(
        draft, before,
        "neither Add nor Remove touches the autoload flag, so the draft is the one it started \
         from",
    );
}

#[test]
fn removing_the_only_member_leaves_an_empty_roster() {
    let mut draft = GangDraft::new_gang();
    let added = apply(&mut draft, EditorListOpNet::Add);
    assert!(added.is_ok());

    let removed = apply(
        &mut draft,
        EditorListOpNet::Remove(EditorListIndexNet::new(0)),
    );

    assert!(
        removed.is_ok(),
        "the roster keeps no minimum, so the last member comes off like any other",
    );
    assert!(draft.members().is_empty());
    assert!(members(&draft).is_empty());
}

#[test]
fn the_reply_reads_the_member_names_back_in_order() {
    let mut draft = GangDraft::new_gang();
    for _ in 0..2 {
        let added = apply(&mut draft, EditorListOpNet::Add);
        assert!(added.is_ok());
    }
    let Some(second) = draft.members_mut().get_mut(1) else {
        unreachable!("two members were added");
    };
    second.name = gdtf_battle_sim::ganger::GangerName::new("Kez".to_owned());

    let read = members(&draft);

    let names: Vec<EditorListMemberNet> = draft
        .members()
        .iter()
        .map(|member| {
            EditorListMemberNet::GangMember(crate::net_qa::wire::EditorDraftNameNet::new(
                member.name.as_str(),
            ))
        })
        .collect();
    assert_eq!(read, names, "the reply names the members the draft holds");
}

#[test]
fn a_remove_past_the_end_is_refused_and_the_roster_is_unchanged() {
    let mut draft = GangDraft::new_gang();
    let added = apply(&mut draft, EditorListOpNet::Add);
    assert!(added.is_ok());
    let before = draft.clone();

    let (kind, line) = refusal(apply(
        &mut draft,
        EditorListOpNet::Remove(EditorListIndexNet::new(1)),
    ));

    assert_eq!(kind, "BadArguments", "got `{line}`");
    assert!(line.contains("past the end"), "got `{line}`");
    assert_eq!(draft, before, "the refused remove took nothing off");
}

#[test]
fn every_operation_the_roster_draws_no_button_for_is_refused_naming_the_list() {
    let mut draft = GangDraft::new_gang();
    let index = EditorListIndexNet::new(0);
    for op in [
        EditorListOpNet::Toggle(EditorListMemberNet::EntrySide(TerrainFacingNet::East)),
        EditorListOpNet::SetAt(
            index,
            EditorListMemberNet::EntrySide(TerrainFacingNet::East),
        ),
        EditorListOpNet::MoveUp(index),
        EditorListOpNet::MoveDown(index),
    ] {
        let (kind, line) = refusal(apply(&mut draft, op));
        assert_eq!(kind, "BadArguments", "got `{line}`");
        assert!(
            line.contains("GangMembers"),
            "the line names the list: `{line}`"
        );
    }
    assert!(
        draft.members().is_empty(),
        "no refused op touched the roster"
    );
}
