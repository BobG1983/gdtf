use gdtf_assets::{ContentIntegrityReport, ContentValidationDone};

use crate::{
    mcp_editor_reads::{names::EDITOR_VALIDATION, rows::ValidationReplyRow},
    mcp_shared::{
        harness::editing_app_and_client, load_case::reply_answered_during_load, outcome::ran_body,
        socket::run_editor, support::TestResult,
    },
};

#[test]
fn a_load_pass_reply_reports_the_report_as_unpublished() -> TestResult {
    let reply = reply_answered_during_load(
        run_editor(EDITOR_VALIDATION, "()"),
        "the editor.validation run",
    )?;
    let body: ValidationReplyRow = ran_body(&reply, EDITOR_VALIDATION)?;

    assert!(
        !body.published,
        "the publish system inserts ContentValidationDone, and it has not run before the first \
         frame. A client that read only `findings` would take this empty list for a clean \
         bill of health: {body:?}",
    );
    assert!(
        !body.checks_complete,
        "the reference checks have not run either, so the reply says so rather than implying \
         a finished pass: {body:?}",
    );
    Ok(())
}

#[test]
fn an_editing_reply_reports_a_published_pass_and_the_findings_the_world_holds() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let reply = client.exchange(&mut app, &run_editor(EDITOR_VALIDATION, "()"))?;
    let body: ValidationReplyRow = ran_body(&reply, EDITOR_VALIDATION)?;

    assert!(
        body.checks_complete && body.published,
        "the editor reached Editing, so the validation pass has both run its checks and \
         published: {body:?}",
    );
    let Some(report) = app.world().get_resource::<ContentIntegrityReport>() else {
        return Err("the report is inserted at plugin build, so it is in the world".into());
    };
    let rendered: Vec<String> = report
        .findings()
        .iter()
        .map(std::string::ToString::to_string)
        .collect();
    assert_eq!(
        body.findings, rendered,
        "the reply carries the world's own findings, rendered through the same Display the \
         editor's log prints. A reply built from anything else would drift from the log",
    );
    assert!(
        app.world()
            .get_resource::<ContentValidationDone>()
            .is_some(),
        "the published marker is the resource the reply's `published` flag mirrors",
    );
    Ok(())
}
