use gdtf_qa_protocol::{
    command::{CommandName, RonShape, ShapeBody, ShapeDoc, ShapeName},
    message::{QaRequest, QaResponse},
};

use super::{
    command_exchange::{APP_PHASE, exchange},
    socket_support::{TestResult, game_app_listening},
};

fn parsed(document: &str, side: &str) -> ShapeDoc {
    let Ok(doc) = ron::de::from_str::<ShapeDoc>(document) else {
        unreachable!("the published {side} document is a RON shape, got `{document}`");
    };
    doc
}

#[test]
fn the_catalogue_row_carries_the_traced_shapes() -> TestResult {
    let reply = exchange(game_app_listening, QaRequest::Catalogue)?;
    let QaResponse::Catalogue(catalogue) = reply else {
        unreachable!("a Catalogue request is answered with a catalogue, got {reply:?}");
    };
    let Some(entry) = catalogue
        .entries
        .iter()
        .find(|entry| entry.command == CommandName::from_static(APP_PHASE))
    else {
        unreachable!("the catalogue carries an `{APP_PHASE}` row: {catalogue:?}");
    };

    let arguments = parsed(entry.arguments.as_str(), "argument");
    assert_eq!(
        arguments.root(),
        &RonShape::Named(ShapeName::from_static("AppPhaseArgs")),
        "the argument shape is traced from the command's own Args type: {arguments:?}",
    );
    assert_eq!(
        arguments.root_body(),
        Some(&ShapeBody::Record(Vec::new())),
        "app.phase takes no arguments, so its record is empty: {arguments:?}",
    );

    let reply_shape = parsed(entry.reply.as_str(), "reply");
    let Some(ShapeBody::Record(fields)) = reply_shape.root_body() else {
        unreachable!("the reply shape's root is a record: {reply_shape:?}");
    };
    let Some(phase) = fields.iter().find(|field| field.name().as_str() == "phase") else {
        unreachable!("the reply shape names the `phase` field: {reply_shape:?}");
    };
    let RonShape::Named(nested) = phase.shape() else {
        unreachable!("the `phase` field points at a named record: {reply_shape:?}");
    };
    let Some(ShapeBody::Record(levels)) = reply_shape.body_of(nested) else {
        unreachable!("the nested phase record is defined in the same document: {reply_shape:?}");
    };
    let Some(app_level) = levels.iter().find(|field| field.name().as_str() == "app") else {
        unreachable!("the phase record names its `app` level: {reply_shape:?}");
    };
    let RonShape::Named(lifecycle) = app_level.shape() else {
        unreachable!("the `app` level points at a named enum: {reply_shape:?}");
    };
    let Some(ShapeBody::Choice(variants)) = reply_shape.body_of(lifecycle) else {
        unreachable!("the lifecycle enum is defined in the same document: {reply_shape:?}");
    };
    let named: Vec<&str> = variants
        .iter()
        .map(|variant| variant.name().as_str())
        .collect();
    assert_eq!(
        named,
        vec!["Init", "Load", "Intro", "Running", "Teardown"],
        "the reply shape reaches every lifecycle variant an agent could be told about: \
         {reply_shape:?}",
    );
    Ok(())
}
