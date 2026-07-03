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

mod components;
/// Test-support re-exports for this scene (GTW-569 one-hop ledger): the editable
/// gang/member model (GTW-420) plus the editor screen / row / field markers the headless
/// integration tests name through `crate::test_support`. The crate-root ledger
/// (`src/test_support.rs`) re-exports these by explicit name directly from here — no
/// intermediate `mod.rs` climb. `pub(crate)` on the module (not `pub`) because the parent
/// chain is `pub(crate)`, so a `pub mod` here trips the workspace `unreachable_pub = deny`.
#[cfg(feature = "test-support")]
pub(crate) mod test_support {
    pub use super::{
        components::{
            AddMemberButton, AttributeField, BaseAttribute, DeleteMemberButton, DerivedStat,
            DerivedStatText, EditorScreenRoot, ExpandPip, GangNameField, MemberArmorDropdown,
            MemberListHost, MemberNameField, MemberPortrait, MemberRow, MemberRowIndex,
            MemberRowRef, MemberStatPanel, MemberWeaponDropdown, PipExpanded,
        },
        model::{EditableGang, EditableMember},
    };
}

// DEV-ONLY editor self-screenshot QA hook (C6), double-gated on the dev cfg + its own env var
// (the GTW-419 / GTW-297 capture discipline). Compiled in only for a `dev_capture` debug build.
#[cfg(all(debug_assertions, feature = "dev_capture"))]
mod capture;
