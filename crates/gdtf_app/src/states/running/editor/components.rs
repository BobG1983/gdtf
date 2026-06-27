//! Marker components for the gang-editor screen (GTW-420).
//!
//! Each marker is a unit struct — presence alone is the signal (no-bare-types rule). They let
//! the editor systems and the headless tests find the screen root, the gang-name field, the
//! "Add member" button, and a member-list row by meaning rather than by spawn order.
//!
//! Visibility follows the crate's test-only-surface convention (GTW-145): each marker is
//! declared through [`crate::support_item!`], which widens it to `pub` under the `test-support`
//! feature — so the external integration tests can name it through
//! [`crate::test_support`](crate::test_support) — and keeps it `pub(crate)` otherwise, so the
//! binary build (compiled WITHOUT `test-support`) stays `unreachable_pub`-clean.

use bevy::prelude::*;

crate::support_item! {
    /// Marks the editor screen's ROOT node (the themed panel layout).
    ///
    /// Carries [`DespawnOnExit(RunningState::DebugEditor)`](bevy::prelude::DespawnOnExit) so the
    /// whole screen tears down on leave. The headless test asserts this root EXISTS in
    /// `DebugEditor` and is GONE after the transition away (C1 / C5).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct EditorScreenRoot;
}

crate::support_item! {
    /// Marks the gang-NAME text field (the [`spawn_text_field`](gdtf_ui::spawn_text_field)
    /// widget root) so the [`TextFieldCommitted`](gdtf_ui::TextFieldCommitted) listener can map a
    /// commit on THIS field to the model name (AC3).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct GangNameField;
}

crate::support_item! {
    /// Marks the "Add member" button so the add-member system can read its
    /// [`Interaction`](bevy::ui::Interaction) press (AC4).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct AddMemberButton;
}

crate::support_item! {
    /// Marks the host node the per-member rows are spawned UNDER (the member-list SHELL).
    ///
    /// "Add member" spawns a minimal [`MemberRow`] under this host. A single marker for the
    /// list container so the add-member system can parent a new row without re-querying the
    /// whole tree.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct MemberListHost;
}

crate::support_item! {
    /// Marks one member-list ROW in the SHELL — a minimal-but-real row per member (GTW-420
    /// scaffold scope; the rich collapsed-row view is GTW-425).
    ///
    /// The headless test counts these rows and asserts the count grows by one per "Add member".
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct MemberRow;
}
