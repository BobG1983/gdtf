//! The DEV-ONLY in-app gang-editor scene (GTW-420) — the foundation of the GTW-403
//! gang-editor track.
//!
//! Owns [`RunningState::DebugEditor`](super::RunningState)'s screen, the editable
//! [`EditableGang`](model::EditableGang) model, and the gang-name-edit / add-member systems.
//! The ENTRY point (the "Gang Editor" main-menu button) is `cfg(debug_assertions)`-gated in the
//! menu scene, so this editor is unreachable in a release binary even though the state variant
//! exists.

mod plugin;
mod systems;
pub(in crate::states::running) use plugin::EditorScenePlugin;

mod model;
// The editable model + member types are named by the headless integration tests through
// `crate::test_support`; carry them up the chain under the `test-support` feature (gated so the
// binary build, compiled WITHOUT test-support, stays `unreachable_pub`-clean — the menu-marker
// precedent).
#[cfg(feature = "test-support")]
crate::support_use!(model::{EditableGang, EditableMember};);

mod components;
#[cfg(feature = "test-support")]
crate::support_use! {
    components::{AddMemberButton, EditorScreenRoot, GangNameField, MemberListHost, MemberRow};
}

// DEV-ONLY editor self-screenshot QA hook (C6), double-gated on the dev cfg + its own env var
// (the GTW-419 / GTW-297 capture discipline). Compiled in only for a `dev_capture` debug build.
#[cfg(all(debug_assertions, feature = "dev_capture"))]
mod capture;
