//! The editor's QA **snapshot service** — the editor model read and serialized into the
//! ADR 0007 query views (GTW-805).
//!
//! The service is a [`SystemParam`](bevy::ecs::system::SystemParam)
//! ([`EditorQaModel`]) the request drain takes, plus the pure builders over it, rather than a
//! separate system: a reply is written into the request's own
//! [`Responder`](gdtf_net_qa_transport::Responder), which the drain owns for exactly the
//! frame it drains the request in, so the read must happen there. Everything below the model
//! read is a pure function of the model — no world mutation, no ordering against another
//! system, and unit-reachable from a test that inserts the resources it names.
//!
//! Every model resource is read as `Option<Res<…>>`, because none of them is guaranteed
//! present — but they are not absent for the same reason, and NOT over the same window
//! (GTW-879):
//!
//! - `EditorMode` / `MapEditorSession` / the drafts are state-scoped to
//!   `EditorState::Editing` (bevy-traps #1), so they exist only while the editor is `Editing`.
//! - [`ContentIntegrityReport`](gdtf_assets::ContentIntegrityReport) is `init_resource`'d
//!   while the app is still being BUILT, so it exists from the editor's first frame, `Load`
//!   included; its companion `ContentChecksComplete` marker lands mid-`Load`, once the checks
//!   have actually run.
//!
//! An absent resource means the topic is NOT offered — never an empty view standing in for an
//! answer. See [`topics`] for what that makes live when.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`model`] — [`EditorQaModel`], the one read of the editor's state + model resources.
//! - [`readiness`] — the `Load` / `Editing` readiness mirror every reply carries.
//! - [`topics`] — the availability filter + the options reply.
//! - [`answer`] — the per-topic answer dispatch.
//! - [`mode`] — the MODE topic (and the shared `EditorMode` → wire mapping).
//! - [`session`] — the SESSION topic.
//! - [`validation`] — the VALIDATION topic.
//! - [`draft`] — the DRAFT topic, one file per authoring mode.

mod answer;
mod draft;
mod mode;
mod model;
mod readiness;
mod session;
mod topics;
mod validation;

pub(super) use answer::answer_topic;
pub(super) use model::EditorQaModel;
pub(super) use topics::{options_view, topic_available};
