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

/// Marks the "Save gang" button so the GTW-429 save system can read its
/// [`Interaction`](bevy::ui::Interaction) press (C4) and write the edited
/// [`EditableGang`](super::model::EditableGang) to a `*.gang.ron` gang file on disk (C1).
///
/// The save action's live-play trigger: pressing it serializes the full edited model into the
/// GTW-415 [`GangRoster`](gdtf_battle_sim::GangRoster) schema and writes it to
/// `assets/content/gangs/<gang_name>.gang.ron` (the extension derived from the gangs family's
/// canonical one since GTW-621). A plain `pub(in …editor)` marker (NOT
/// [`crate::support_item!`]): no EXTERNAL test names it — the GTW-429 round-trip test is in-crate
/// and exercises the model-projection + loader path directly, not the button widget.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(in crate::states::running::editor) struct SaveGangButton;

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
    /// [`MemberPortrait`] placeholder, the inline [`MemberNameField`], the [`MemberWeaponDropdown`],
    /// the [`MemberArmorDropdown`], and a [`DeleteMemberButton`] — exactly ONE editable control per
    /// field (GTW-499 C1: the redundant static echo labels were removed).
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
    /// it flips the state, re-glyphs the pip (`+` collapsed, `-` expanded), AND drives the matching
    /// [`MemberStatPanel`]'s GTW-416 [`AccordionContent`](gdtf_ui::AccordionContent) toggle so the
    /// per-member stat table lerps open / closed (GTW-428 C1).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ExpandPip;
}

crate::support_item! {
    /// Marks a member row's **expanded stat panel** — the GTW-416
    /// [`AccordionContent`](gdtf_ui::AccordionContent) the pip lerps open / closed (GTW-428 C1).
    ///
    /// It IS the accordion content node (it also carries
    /// [`AccordionAnim`](gdtf_ui::AccordionAnim) / [`AccordionProgress`](gdtf_ui::AccordionProgress)
    /// so the shared `drive_accordions` lerps its height), plus this marker + the row's
    /// [`MemberRowIndex`] so a pip press drives the panel for the SAME member. It hosts the eight
    /// editable [`AttributeField`] numeric fields and the readonly [`DerivedStatText`] displays.
    /// The headless test asserts a pip press flips this panel's accordion target / progress.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct MemberStatPanel;
}

crate::support_item! {
    /// Marks a member row's **portrait placeholder** — a plain colored node (no portrait system
    /// exists yet, C1).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct MemberPortrait;
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

crate::support_item! {
    /// Which of a ganger's eight EDITABLE direct attributes one
    /// [`AttributeField`] numeric field edits (GTW-428 C2).
    ///
    /// A closed named vocabulary (an enum IS a named domain type — no-bare-types) over the eight
    /// `docs/combat/stats.md` direct attributes the editor exposes as numeric fields. The
    /// attribute-edit system reads it off a committed field to route the new value to the right
    /// [`EditableMember`](super::model::EditableMember) attribute, then re-derives. Carried on each
    /// attribute field alongside the row's [`MemberRowIndex`].
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
    enum BaseAttribute {
        /// **Speed** — quickness; drives the derived TU budget + Fight / Reactions.
        Speed,
        /// **Aim** — marksmanship; the dominant Shooting term.
        Aim,
        /// **Strength** — physical power; a Fight term.
        Strength,
        /// **Toughness** — damage resistance; an HP-derivation + severity term.
        Toughness,
        /// **Reflexes** — reaction speed; a Shooting + Reactions term.
        Reflexes,
        /// **Cool** — nerves; the broad Shooting / Fight / Reactions / HP / Morale contributor.
        Cool,
        /// **Grit** — resilience; the dominant HP + Morale term, and a Fight term.
        Grit,
        /// **Luck** — directional fortune; feeds the severity roll only, never the computed stats.
        Luck,
    }
}

impl BaseAttribute {
    crate::support_item! {
        /// The eight editable attributes in display order (Speed → Luck) — the order the expanded
        /// panel lays its numeric fields, and the order the attribute-edit system iterates. Widened
        /// for the external GTW-428 test to iterate the full attribute set.
        const ALL: [Self; 8] = [
            Self::Speed,
            Self::Aim,
            Self::Strength,
            Self::Toughness,
            Self::Reflexes,
            Self::Cool,
            Self::Grit,
            Self::Luck,
        ];
    }

    crate::support_item! {
        /// The attribute's short display label — the text beside its numeric field.
        #[must_use]
        const fn label(self) -> &'static str {
            match self {
                Self::Speed => "Speed",
                Self::Aim => "Aim",
                Self::Strength => "Strength",
                Self::Toughness => "Toughness",
                Self::Reflexes => "Reflexes",
                Self::Cool => "Cool",
                Self::Grit => "Grit",
                Self::Luck => "Luck",
            }
        }
    }
}

crate::support_item! {
    /// Marks one EDITABLE base-attribute numeric field in a member's expanded stat panel
    /// (GTW-428 C2) — a [`spawn_numeric_field`](gdtf_ui::spawn_numeric_field) widget root.
    ///
    /// Carries the row's [`MemberRowIndex`] + a [`BaseAttribute`] so a
    /// [`NumericFieldCommitted`](gdtf_ui::NumericFieldCommitted)`<f32>` on THIS field maps to the
    /// right member's right attribute. Its value is clamped via the field's
    /// [`NumericRange`](gdtf_ui::NumericRange) (C2).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct AttributeField;
}

crate::support_item! {
    /// Which READONLY derived combat stat one [`DerivedStatText`] node displays (GTW-428 C2 / C3).
    ///
    /// A closed named vocabulary (an enum IS a named domain type — no-bare-types) over the
    /// `docs/combat/stats.md` computed stats the GTW-384 [`derive_stats`](gdtf_battle_sim::derive_stats)
    /// pipeline yields. The recompute system formats the matching
    /// [`DerivedStats`](gdtf_battle_sim::DerivedStats) field into the node carrying this kind +
    /// the row's [`MemberRowIndex`], so the shown value equals the pipeline output for the live
    /// attributes (C3).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
    enum DerivedStat {
        /// `Shooting` — the ranged-to-hit skill (live).
        Shooting,
        /// `Fight` — the melee skill (dormant).
        Fight,
        /// `Reactions` — enemy-turn responses (dormant).
        Reactions,
        /// `Morale` — the psychological damage pool (dormant).
        Morale,
        /// `Tu` — the per-turn action budget (live).
        Tu,
        /// `Hp` — the in-battle knock-down pool (live).
        Hp,
        /// `Wounds` — the life pool (live).
        Wounds,
        /// `Bottle` — the psychological life pool (dormant).
        Bottle,
    }
}

impl DerivedStat {
    crate::support_item! {
        /// The eight derived stats in display order — the order the expanded panel lays its
        /// readonly displays, and the order the recompute system iterates. Widened for the external
        /// GTW-428 test to iterate the full derived-stat set.
        const ALL: [Self; 8] = [
            Self::Shooting,
            Self::Fight,
            Self::Reactions,
            Self::Morale,
            Self::Tu,
            Self::Hp,
            Self::Wounds,
            Self::Bottle,
        ];
    }

    crate::support_item! {
        /// The stat's short display label — the text beside its readonly value.
        #[must_use]
        const fn label(self) -> &'static str {
            match self {
                Self::Shooting => "Shooting",
                Self::Fight => "Fight",
                Self::Reactions => "Reactions",
                Self::Morale => "Morale",
                Self::Tu => "TU",
                Self::Hp => "HP",
                Self::Wounds => "Wounds",
                Self::Bottle => "Bottle",
            }
        }
    }
}

crate::support_item! {
    /// Marks one READONLY derived-stat display in a member's expanded stat panel (GTW-428 C2 /
    /// C3) — a [`Text`](bevy::prelude::Text) node the recompute MUTATES in place.
    ///
    /// Carries the row's [`MemberRowIndex`] + a [`DerivedStat`] so the recompute system finds the
    /// matching node and writes the live pipeline output (the ui-mutate rule — never a respawn).
    /// The headless test reads this node and asserts it equals the GTW-384 pipeline output for the
    /// current attributes (C3).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct DerivedStatText;
}
