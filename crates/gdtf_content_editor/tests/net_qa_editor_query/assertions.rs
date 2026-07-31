//! The per-phase assertions over the collected replies (GTW-805; re-grounded on the real
//! editor in GTW-879).

use gdtf_qa_protocol::{
    envelope::{ProtocolVersion, QaError, QaResponse},
    view::{
        EditorModeNet, EditorQueryKind, EditorQueryOptionsView, EditorQueryReply, EditorQueryView,
        EditorReadinessNet,
    },
};

use crate::support::LOAD_PHASE_REPLIES;

/// The options view inside an options reply.
fn options(reply: &QaResponse) -> &EditorQueryOptionsView {
    let QaResponse::EditorQueryOptions(view) = reply else {
        unreachable!("expected an EditorQueryOptions reply, got {reply:?}");
    };
    view
}

/// The reply an ANSWERED topic query carries.
///
/// A refusal reaching this is a broken contract rather than a shape mismatch: the availability
/// filter decides BOTH what the options reply advertises and whether a query is answered
/// (`net_qa/router.rs`), so a topic the options reply offered can only be answered.
fn answered_query(reply: &QaResponse) -> &EditorQueryReply {
    let QaResponse::EditorQuery(answer) = reply else {
        unreachable!("expected an answered EditorQuery reply, got {reply:?}");
    };
    answer
}

/// The topics an options reply offers, in order.
fn offered_topics(reply: &QaResponse) -> Vec<EditorQueryKind> {
    options(reply)
        .topics
        .iter()
        .map(|topic| topic.kind)
        .collect()
}

/// The `Load`-phase assertions (see the suite doc).
///
/// The FIRST options reply carries the whole `Load`-phase claim, and it is a `Load`
/// observation BY CONSTRUCTION rather than by luck: the client puts that request on the wire
/// before the test body runs a single frame, so it is answered while the editor's content
/// folder loads have only just been requested in `Startup`.
///
/// The FIVE exchanges after it each cost a frame, and the editor's asset pass can finish in
/// as few as six frames under parallel `cargo` contention, so they are asserted through facts
/// that hold in EITHER readiness: the readiness each reply carries, whether the topic is
/// answered at all, and the offered-topic list of the options reply that closes the window.
/// [`EditorState`](gdtf_content_editor::EditorState)
/// only ever goes `Load` → `Editing` and never back, so that closing list brackets everything
/// asked before it.
///
/// The parameter is a FIXED-SIZE array rather than a slice: `client::drive` builds the same
/// `[QaResponse; LOAD_PHASE_REPLIES]`, so an exchange added to the wire without an assertion
/// arm here fails to compile. GTW-879 first landed with a six-element wire list against a
/// five-element slice pattern, and the pattern's fallback arm swallowed every assertion below
/// it — including both phases' — on every run.
pub(crate) fn assert_load_phase(replies: &[QaResponse; LOAD_PHASE_REPLIES]) {
    let [
        hello,
        first_options,
        readiness,
        mode,
        validation,
        closing_options,
    ] = replies;

    // The handshake opens the connection (GTW-940) and is answered in the LISTENER THREAD, so
    // it spends no frame of the `Load` window the rest of this phase measures — and the facts
    // it carries are the editor's own, handed to the listener by the plugin.
    assert!(
        matches!(hello, QaResponse::HelloOk(facts) if facts.protocol == ProtocolVersion::CURRENT),
        "expected the editor's HelloOk at the CURRENT protocol version, got {hello:?}",
    );

    assert_eq!(
        options(first_options).readiness,
        EditorReadinessNet::Load,
        "the first options reply is answered on one of the editor's first frames, before any \
         content registry can have resolved — it must report the Load asset pass",
    );
    assert_eq!(
        offered_topics(first_options),
        vec![EditorQueryKind::Readiness, EditorQueryKind::Validation],
        "during Load the editor offers the two topics whose resources already exist: Readiness \
         (always answerable) and Validation (ContentIntegrityReport is init_resource'd while the \
         app is built, so the report exists from frame 0 and its checks_complete flag is what \
         says the checks have not run yet). Mode / Session / Draft read Editing-scoped \
         resources and are correctly withheld",
    );

    let readiness = answered_query(readiness);
    assert_eq!(
        readiness.view,
        EditorQueryView::Readiness(readiness.readiness),
        "the readiness topic answers the readiness the reply itself carries — it is the one \
         topic observable in EVERY editor state",
    );

    // The availability filter and the answer run off one model read, so what the options reply
    // offers and what a query is answered with can never disagree (ADR 0007). The Mode query
    // sits BETWEEN the two options replies, and readiness only ever moves `Load` → `Editing`:
    // so when the CLOSING reply still withholds Mode, the whole window was `Load` and the
    // refusal is exact. When it offers Mode the asset pass finished somewhere inside the
    // window — the state at the moment of the query is then genuinely unknown, and the only
    // true claim is that the answer is one of the two shapes. (The `Editing`-phase assertions
    // pin the answered shape; the FIRST options reply above pins the withheld one.)
    if offered_topics(closing_options).contains(&EditorQueryKind::Mode) {
        assert!(
            matches!(
                mode,
                QaResponse::Error(QaError::BadRequest) | QaResponse::EditorQuery(_)
            ),
            "the editor entered Editing inside the Load-phase window, so the Mode query is \
             either the Load refusal or the Editing answer — never anything else: got {mode:?}",
        );
    } else {
        assert!(
            matches!(mode, QaResponse::Error(QaError::BadRequest)),
            "Mode was still withheld when the window closed, so it was withheld for the whole \
             window: the query must be refused, never answered with an invented empty view — \
             got {mode:?}",
        );
    }

    // VALIDATION is the topic this ticket exists for, and this is the exchange that proves the
    // `Load`-phase offer above is real rather than advertised. It needs no bracket to be
    // answered: the availability filter is ONE predicate for both the advertisement and the
    // answer (`net_qa/router.rs`), and it offers Validation in EVERY editor state because the
    // `ContentIntegrityReport` the topic reads is `init_resource`'d while the app is BUILT
    // (`load/injuries.rs`, `gdtf_assets`'s family registration). So whichever side of the
    // transition the frames put this query on, it is ANSWERED — never the `BadRequest` the
    // Mode query above may still be getting in the same window.
    let validation = answered_query(validation);
    let EditorQueryView::Validation(validation_view) = &validation.view else {
        unreachable!(
            "the Validation topic must be answered with a Validation view, got {:?}",
            validation.view,
        );
    };
    // The flag itself brackets on the readiness the reply carries. `Editing` means the checks
    // have run: the editor's validation window opens once the nine registries its checks read
    // are present (`validate/register.rs`), the `Load → Editing` gate waits on those same nine
    // PLUS `InjuryTables` (`load/transition.rs`), and both run in `Update` — so the window is
    // open no later than the frame the transition is set, and the marker is stamped in that
    // frame. A reply carrying `Load` may report either, which is precisely why the flag is on
    // the wire: an empty finding list before the checks run means "not checked", not "clean".
    assert!(
        validation.readiness == EditorReadinessNet::Load || *validation_view.checks_complete,
        "a Validation reply carrying readiness Editing must report checks_complete — the \
         checks' window opens no later than the frame the Load → Editing transition is set: \
         got {validation:?}",
    );
}

/// The `Editing`-phase assertions (see the suite doc).
pub(crate) fn assert_editing_phase(replies: &[QaResponse]) {
    let [options_reply, rest @ ..] = replies else {
        unreachable!("expected an options reply and one per topic, got {replies:?}");
    };
    assert_eq!(
        offered_topics(options_reply),
        EditorQueryKind::ALL.to_vec(),
        "in Editing every topic in the EditorQueryKind enum is offered — the options list \
         and the enum stay in step",
    );
    assert_eq!(rest.len(), EditorQueryKind::ALL.len());

    for (kind, reply) in EditorQueryKind::ALL.into_iter().zip(rest) {
        let QaResponse::EditorQuery(reply) = reply else {
            unreachable!("expected an EditorQuery reply for {kind:?}, got {reply:?}");
        };
        assert_eq!(
            reply.readiness,
            EditorReadinessNet::Editing,
            "every editor query reply carries the readiness that produced it",
        );
        assert_topic_view(kind, &reply.view);
    }
}

/// Assert one topic's answer matches the model the REAL editor opened with.
///
/// Every value here is what a freshly opened editor produces: the Workbench's default
/// `Prefab` mode, the session's default `60×60×8` drawable area with the theme
/// `seed_default_theme` resolved off the shipped theme registry, the untouched prefab map on
/// the ground storey, and a completed validation pass over the shipped content.
fn assert_topic_view(kind: EditorQueryKind, view: &EditorQueryView) {
    match (kind, view) {
        (EditorQueryKind::Readiness, EditorQueryView::Readiness(readiness)) => {
            assert_eq!(*readiness, EditorReadinessNet::Editing);
        }
        (EditorQueryKind::Mode, EditorQueryView::Mode(mode)) => {
            assert_eq!(
                mode.active,
                EditorModeNet::Prefab,
                "the editor opens in its DEFAULT mode — Prefab, the painter the Workbench grew \
                 around (EditorMode::default)",
            );
            assert_eq!(*mode.label, "PREFAB");
            assert_eq!(
                *mode.tab_index, 2,
                "PREFAB is the third tab: TERRAIN | THEME | PREFAB | …",
            );
        }
        (EditorQueryKind::Session, EditorQueryView::Session(session)) => {
            assert_eq!(*session.grid_size.width, 60);
            assert_eq!(*session.grid_size.height, 60);
            assert_eq!(
                *session.grid_size.levels, 8,
                "a fresh session opens on the full 60x60x8 drawable area \
                 (MapEditorSession::default)",
            );
            assert!(
                session.default_floor.is_some(),
                "seed_default_theme resolves the first shipped theme and its default floor once \
                 the registry lands, so the session's floor must ride the view: {session:?}",
            );
            assert!(
                session.selected_tile.is_none(),
                "nothing has been picked off the palette in a freshly opened editor",
            );
        }
        (EditorQueryKind::Draft, EditorQueryView::Draft(draft)) => {
            assert_eq!(
                draft.mode,
                EditorModeNet::Prefab,
                "the draft topic answers the ACTIVE mode's draft, and the active mode is Prefab",
            );
            let fields: Vec<(&str, &str)> = draft
                .fields
                .iter()
                .map(|field| (&**field.name, &**field.value))
                .collect();
            assert_eq!(
                fields,
                vec![
                    ("painted_cells", "0"),
                    ("current_level", "Some(CurrentEditLevel(Level(0)))"),
                ],
                "PREFAB's draft is the painted map itself: nothing painted yet, on the ground \
                 storey the canvas opens at",
            );
        }
        (EditorQueryKind::Validation, EditorQueryView::Validation(validation)) => {
            assert!(
                *validation.checks_complete,
                "the editor's validation window opens as soon as the registries the checks read \
                 resolve, which is no later than the frame the Load → Editing transition is set \
                 — so by Editing the checks have run: {validation:?}",
            );
            assert!(
                validation.findings.is_empty(),
                "the shipped content passes every registered reference check — a finding here is \
                 a real content defect, not a test fixture: {:?}",
                validation.findings,
            );
        }
        (kind, view) => unreachable!("{kind:?} was answered with the wrong view: {view:?}"),
    }
}
