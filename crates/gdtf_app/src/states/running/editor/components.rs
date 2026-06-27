//! Marker components for the gang-editor screen (GTW-420 scaffold + GTW-425 collapsed rows).
//!
//! Each unit marker is the signal by presence alone (no-bare-types rule); the row-index / pip /
//! row-ref newtypes wrap their inner value privately (rule 5). They let the editor systems and
//! the headless tests find the screen root, the gang-name field, the "Add member" button, the
//! member-list shell, and each collapsed row's controls by meaning rather than by spawn order.
//!
//! Visibility follows the crate's test-only-surface convention (GTW-145): each marker is
//! declared through [`crate::support_item!`], which widens it to `pub` under the `test-support`
//! feature — so the external integration tests can name it through
//! [`crate::test_support`](crate::test_support) — and keeps it `pub(crate)` otherwise, so the
//! binary build (compiled WITHOUT `test-support`) stays `unreachable_pub`-clean.

use bevy::prelude::*;
// `Button` is referenced only by the intra-doc links below; the import keeps `cargo doc`'s
// `broken_intra_doc_links` lint satisfied without being used in code.
#[expect(
    unused_imports,
    reason = "Button is referenced only by intra-doc links"
)]
use bevy::ui::widget::Button;

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
    /// Marks the GTW-412 [`ScrollList`](gdtf_ui::ScrollList) ROOT FRAME the per-member rows
    /// scroll inside of (the member-list SHELL).
    ///
    /// `spawn_scroll_list` puts THIS marker on the grid root frame and RETURNS the
    /// [`ScrollListArea`](gdtf_ui::ScrollListArea) viewport; the rows hang in the area child, NOT
    /// the marker frame (the GTW-421/GTW-422 scroll-list parenting rule), so they top-anchor and
    /// scroll on overflow instead of being bottom-cramped off-screen.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct MemberListHost;
}

crate::support_item! {
    /// Marks one **collapsed member ROW** in the scrollable list — the GTW-425 row root.
    ///
    /// One row per model member, carrying its [`MemberRowIndex`] so a commit / selection / delete
    /// maps back to the right member. The row holds (left→right) an [`ExpandPip`] toggle, a
    /// [`MemberPortrait`] placeholder, the inline [`MemberNameField`] beside a [`MemberNameText`]
    /// echo, the [`MemberWeaponText`] + [`MemberWeaponDropdown`], the [`MemberArmorText`] +
    /// [`MemberArmorDropdown`], and a [`DeleteMemberButton`].
    ///
    /// The headless test counts these rows and asserts the count grows by one per "Add member"
    /// and shrinks by one per delete.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct MemberRow;
}

crate::support_item! {
    /// The index of a [`MemberRow`] (and its child controls) into the model's member list.
    ///
    /// A named newtype over the index (no-bare-types rule 5: a row's slot is a domain value, not a
    /// bare `usize`) — carried on the row root AND on each editing control (the pip, name field,
    /// dropdowns, delete button) so a commit / selection / press resolves which member to mutate
    /// WITHOUT relying on the entity tree. After a delete the surviving rows are re-keyed in place
    /// by [`delete_member_on_press`](super::systems::delete_member_on_press).
    #[derive(Component, Deref, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct MemberRowIndex(usize);
}

impl MemberRowIndex {
    crate::support_item! {
        /// Build a row index.
        #[must_use]
        const fn new(index: usize) -> Self {
            Self(index)
        }
    }
}

crate::support_item! {
    /// The expand/collapse state of a member row's [`ExpandPip`] toggle.
    ///
    /// A named newtype over the flag (no-bare-types rule 5: the pip's open/closed state is a
    /// domain value, not a bare `bool`) — held on the [`ExpandPip`] so its press toggles it and a
    /// reader can render the pip's `+` / `-` glyph. GTW-425 renders the pip + its toggle state
    /// ONLY; the EXPANDED per-member stat panel it would gate is GTW-428 (out of scope).
    #[derive(Component, Deref, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct PipExpanded(bool);
}

impl PipExpanded {
    crate::support_item! {
        /// Build an expand-toggle state (`false` = collapsed, the default).
        #[must_use]
        const fn new(expanded: bool) -> Self {
            Self(expanded)
        }
    }

    /// Flip the state, returning the new value — the pip press toggle.
    pub(in crate::states::running::editor) const fn toggle(&mut self) -> bool {
        self.0 = !self.0;
        self.0
    }
}

crate::support_item! {
    /// Marks a member row's **expand-toggle pip** — the `+` / `-` control (a [`Button`]) at the
    /// row's LEFT (C1).
    ///
    /// Carries a [`PipExpanded`] (its open/closed state) + the row's [`MemberRowIndex`]. Pressing
    /// it flips the state and re-glyphs the pip (`+` collapsed, `-` expanded). GTW-425 renders the
    /// pip + its toggle; the expanded per-member stat table it would reveal is GTW-428.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ExpandPip;
}

crate::support_item! {
    /// Marks a member row's **portrait placeholder** — a plain colored node (no portrait system
    /// exists yet, C1).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct MemberPortrait;
}

crate::support_item! {
    /// Marks the [`Text`](bevy::prelude::Text) node showing a member's NAME in its collapsed row
    /// (C1).
    ///
    /// The inline name field's commit MUTATES this text in place (C3/C5) — never a respawn. The
    /// test reads this node's text to confirm the row reflects the edited model name.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct MemberNameText;
}

crate::support_item! {
    /// Marks the [`Text`](bevy::prelude::Text) node showing a member's WEAPON key in its collapsed
    /// row (C1).
    ///
    /// The weapon dropdown's commit MUTATES this text in place (C2/C5). The test reads it to
    /// confirm the row reflects the edited model weapon.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct MemberWeaponText;
}

crate::support_item! {
    /// Marks the [`Text`](bevy::prelude::Text) node showing a member's ARMOR key in its collapsed
    /// row (C1).
    ///
    /// The armor dropdown's commit MUTATES this text in place (C2/C5). The test reads it to
    /// confirm the row reflects the edited model armor.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct MemberArmorText;
}

crate::support_item! {
    /// Marks a member row's inline **name field** (the [`spawn_text_field`](gdtf_ui::spawn_text_field)
    /// widget root) — distinct from the gang-wide [`GangNameField`] (C3).
    ///
    /// Carries the row's [`MemberRowIndex`] so a [`TextFieldCommitted`](gdtf_ui::TextFieldCommitted)
    /// on THIS field maps to that member's name.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct MemberNameField;
}

crate::support_item! {
    /// Marks a member row's **weapon dropdown** (the [`spawn_dropdown`](gdtf_ui::spawn_dropdown)
    /// closed-control root) listing all loaded [`WeaponName`](gdtf_battle_sim::WeaponName) keys
    /// (C2).
    ///
    /// Carries the row's [`MemberRowIndex`] so a
    /// [`DropdownSelectionChanged`](gdtf_ui::DropdownSelectionChanged)`<WeaponName>` on THIS
    /// control maps to that member's weapon loadout.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct MemberWeaponDropdown;
}

crate::support_item! {
    /// Marks a member row's **armor dropdown** (the [`spawn_dropdown`](gdtf_ui::spawn_dropdown)
    /// closed-control root) listing all loaded [`ArmorName`](gdtf_battle_sim::ArmorName) keys
    /// (C2).
    ///
    /// Carries the row's [`MemberRowIndex`] so a
    /// [`DropdownSelectionChanged`](gdtf_ui::DropdownSelectionChanged)`<ArmorName>` on THIS control
    /// maps to that member's armor loadout.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct MemberArmorDropdown;
}

crate::support_item! {
    /// Marks a member row's **delete button** (a [`Button`]) — its press removes the member from
    /// the model AND despawns that row (C4).
    ///
    /// Carries the row's [`MemberRowIndex`] (so the delete maps to the right member) and a
    /// [`MemberRowRef`] (so the press can despawn exactly that [`MemberRow`] root).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct DeleteMemberButton;
}

crate::support_item! {
    /// The [`MemberRow`] root [`Entity`](bevy::prelude::Entity) a control belongs to.
    ///
    /// A named newtype over the entity (no-bare-types rule 5) — carried on the
    /// [`DeleteMemberButton`] so its press can despawn exactly that row root (C4), without walking
    /// the tree to find the row from the button.
    #[derive(Component, Deref, Clone, Copy, PartialEq, Eq, Debug)]
    struct MemberRowRef(Entity);
}

impl MemberRowRef {
    crate::support_item! {
        /// Build a row reference from the row root entity.
        #[must_use]
        const fn new(row: Entity) -> Self {
            Self(row)
        }
    }
}
