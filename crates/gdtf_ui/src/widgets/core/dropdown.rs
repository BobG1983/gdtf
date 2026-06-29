//! The [`Dropdown`] widget: a closed combobox control that opens a FLOATING option
//! list, generic over the option IDENTITY (GTW-410).
//!
//! A dropdown is the reusable combobox the menu / loadout work needs: a THEMED closed
//! CONTROL (a [`Button`]) showing the current selection, that on click opens a FLOATING
//! option list ABOVE every sibling panel. Selecting an option closes the list, MUTATES
//! the closed control's shown label in place ([[ui-mutate-not-respawn]]), and emits a
//! typed [`DropdownSelectionChanged<T>`] carrying the chosen option's IDENTITY. Clicking
//! OUTSIDE the list dismisses it WITHOUT changing the selection.
//!
//! ## Generic over the option identity
//!
//! The widget is generic over a caller `T: OptionId` (e.g. a weapon name, an armor name,
//! a level theme). It DEFINES the message but never knows what a choice MEANS: the
//! downstream listener reads `MessageReader<DropdownSelectionChanged<WeaponName>>` and
//! maps the statically-typed id to an action. Keeping `T` a real type (not an erased
//! string) means an armor listener never sees a weapon selection.
//!
//! ## Floating popup, above sibling panels (bevy-traps #8)
//!
//! The popup is NOT a child of the closed control: it is a TOP-LEVEL absolutely-positioned
//! node carrying `GlobalZIndex(DROPDOWN_POPUP_Z)` — strictly ABOVE the contextual
//! panel's `GlobalZIndex(20)`, so the floating list is never occluded by an opaque sibling
//! (the GTW-294 lesson). [`position_dropdown_popups`] reads the trigger's
//! [`UiGlobalTransform`] + [`ComputedNode`] to place the popup flush below the closed
//! control in window space. A full-screen invisible [`DropdownBackdrop`] sits one band
//! BELOW the popup (`DROPDOWN_BACKDROP_Z`); its press is the outside-click dismiss.
//!
//! ## Keyboard / focus (AC3) — via the existing focus helpers, not reinvented
//!
//! Open focuses the first option ([`set_initial_focus`](crate::focus_nav::set_initial_focus)).
//! The option [`Button`]s are wired into the existing
//! [`DirectionalNavigation`](bevy::input_focus::directional_navigation::DirectionalNavigation)
//! graph (N/S neighbors), so the existing `ArrowUp`/`ArrowDown` →
//! [`NavigateRequest`](crate::focus_nav::NavigateRequest) → `apply_navigation` pipeline moves
//! the highlight with NO new nav code. `Enter` arrives as the existing
//! [`FocusActivated`](crate::focus_nav::FocusActivated) message, which
//! [`activate_focused_option`] turns into a selection. `Escape` is dropdown-specific, so
//! [`dismiss_dropdowns_on_escape`] (gated on an open dropdown existing) emits the
//! dropdown-owned [`DropdownDismissRequest`].

use bevy::{
    input_focus::{InputFocus, directional_navigation::DirectionalNavigationMap},
    math::CompassOctant,
    prelude::*,
    ui::{
        AlignItems, BackgroundColor, FlexDirection, GlobalZIndex, Interaction, JustifyContent,
        Node, Overflow, PositionType, UiRect, Val, widget::Button,
    },
};

use crate::focus_nav::{FocusActivated, set_initial_focus};

/// Implemented by every dropdown option identity type.
///
/// The bound set (`Clone + PartialEq + Send + Sync + 'static` + [`core::fmt::Debug`]) is
/// exactly what a [`Component`] + [`Message`] payload needs, so any caller enum / newtype
/// satisfies it through the blanket impl with no methods to write. `T` carries the
/// MEANING of a choice; the widget never inspects it (it only `==`-compares and clones it).
pub trait OptionId: Clone + PartialEq + Send + Sync + core::fmt::Debug + 'static {}

impl<T: Clone + PartialEq + Send + Sync + core::fmt::Debug + 'static> OptionId for T {}

/// One selectable option: its IDENTITY (`T`) plus its DISPLAY label.
///
/// A named pair rather than a bare `(T, String)`: the identity is what the widget reports
/// in [`DropdownSelectionChanged`], the label is only what the closed control + the option
/// row SHOW. Both inners are private (no-bare-types rule 5), read through accessors.
#[expect(
    clippy::derive_partial_eq_without_eq,
    reason = "T: OptionId is only PartialEq-bound (a float-bearing id need not be Eq), so \
    Eq cannot be derived without over-constraining callers"
)]
#[derive(Clone, PartialEq, Debug)]
pub struct DropdownOption<T: OptionId> {
    /// The option's identity — reported on selection, never shown.
    id:    T,
    /// The option's human-readable label — shown on the row + the closed control.
    label: DropdownOptionLabel,
}

impl<T: OptionId> DropdownOption<T> {
    /// Builds an option from its identity and its display label.
    #[must_use]
    pub fn new(id: T, label: impl Into<String>) -> Self {
        Self {
            id,
            label: DropdownOptionLabel::new(label),
        }
    }

    /// The option's identity.
    #[must_use]
    pub const fn id(&self) -> &T {
        &self.id
    }

    /// The option's display label.
    #[must_use]
    pub const fn label(&self) -> &DropdownOptionLabel {
        &self.label
    }
}

/// The display label of a [`DropdownOption`] (and the closed control's current text).
///
/// A named newtype over the label string rather than a bare `String` (no-bare-types rule),
/// mirroring [`ButtonLabel`](super::ButtonLabel) / [`SegmentLabel`](super::SegmentLabel): a
/// dropdown label is a widget-level value, not arbitrary text.
#[derive(Deref, Clone, PartialEq, Eq, Debug)]
pub struct DropdownOptionLabel(String);

impl DropdownOptionLabel {
    /// Wraps a caption into a [`DropdownOptionLabel`].
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self(label.into())
    }
}

/// The full option list of a [`Dropdown<T>`], stored as a [`Component`] on the root.
///
/// Held on the closed control so [`open_dropdown`] can build the floating list from it
/// without the caller re-passing the options, and so a selection can re-derive the shown
/// label from the chosen index. A named newtype over the `Vec` (no-bare-types rule),
/// read through [`options`](DropdownOptions::options).
#[derive(Component, Clone, PartialEq, Debug)]
pub struct DropdownOptions<T: OptionId>(Vec<DropdownOption<T>>);

impl<T: OptionId> DropdownOptions<T> {
    /// Wraps the caller's option list.
    #[must_use]
    pub const fn new(options: Vec<DropdownOption<T>>) -> Self {
        Self(options)
    }

    /// The options, in display order.
    #[must_use]
    pub fn options(&self) -> &[DropdownOption<T>] {
        &self.0
    }
}

/// The currently-selected option index of a [`Dropdown<T>`], held on the root.
///
/// A named newtype over the index (no-bare-types rule) — the slot into
/// [`DropdownOptions`] the closed control currently SHOWS. `select_dropdown_option`
/// mutates it (and the shown label) on a selection. An out-of-range value simply shows no
/// label (degraded, never panics).
#[derive(Component, Deref, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SelectedIndex(usize);

impl SelectedIndex {
    /// Wraps a selection index.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }
}

/// Marker on the ROOT (the clickable closed control, a [`Button`]) of a [`Dropdown<T>`].
///
/// Generic over the option identity so the open/select drivers query the right widget
/// family. The caller attaches its own identity marker alongside this so the
/// [`DropdownSelectionChanged`] listener can map the control to an action.
///
/// A unit marker — presence alone is the signal (no-bare-types rule). The `PhantomData`
/// uses `fn() -> T` so the unused parameter carries no drop / auto-trait obligations.
#[derive(Component, Debug)]
pub struct Dropdown<T: OptionId>(core::marker::PhantomData<fn() -> T>);

impl<T: OptionId> Default for Dropdown<T> {
    fn default() -> Self {
        Self(core::marker::PhantomData)
    }
}

/// The open/closed state of a [`Dropdown<T>`], held on the root.
///
/// While [`Open`](DropdownState::Open) it records the spawned popup + backdrop entities so
/// the close path can despawn exactly them. A named two-state vocabulary rather than a
/// bare `bool` / `Option<(Entity, Entity)>` (no-bare-types rule): the driver reads
/// `DropdownState::Closed` at the call site, not an opaque flag.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DropdownState {
    /// The list is closed — only the control shows.
    #[default]
    Closed,
    /// The list is open — the floating popup + its dismiss backdrop are live.
    Open {
        /// The floating option-list node.
        popup:    Entity,
        /// The full-screen outside-click dismiss node.
        backdrop: Entity,
    },
}

/// Marker on the LABEL text child of a [`Dropdown`] closed control.
///
/// `select_dropdown_option` mutates its [`Text`] in place to the newly-selected option's
/// label — never despawning it ([[ui-mutate-not-respawn]]). A unit marker (no-bare-types
/// rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct DropdownLabel;

/// Marker on the FLOATING option-list popup of a [`Dropdown`].
///
/// A top-level absolutely-positioned node carrying `GlobalZIndex(DROPDOWN_POPUP_Z)`
/// so it stacks above sibling panels; it carries [`DropdownAnchor`] (the trigger it belongs
/// to) so [`position_dropdown_popups`] can place it flush below the control. A unit marker
/// (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct DropdownPopup;

/// Marker on the full-screen outside-click DISMISS backdrop of an open [`Dropdown`].
///
/// An invisible [`Button`] covering the window one z-band BELOW the popup
/// (`DROPDOWN_BACKDROP_Z`); its [`Interaction::Pressed`](bevy::ui::Interaction::Pressed)
/// is the outside-click that closes the list WITHOUT changing the selection. It carries
/// [`DropdownAnchor`] (the trigger it dismisses). A unit marker (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct DropdownBackdrop;

/// The [`Dropdown`] root a popup / backdrop belongs to.
///
/// Carried on the popup and backdrop so their drivers can reach the owning closed control
/// (to reset its [`DropdownState`], or to read its [`SelectedIndex`]). A named newtype over
/// the [`Entity`] (no-bare-types rule), read through [`trigger`](DropdownAnchor::trigger).
#[derive(Component, Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct DropdownAnchor(Entity);

impl DropdownAnchor {
    /// Wraps the owning trigger entity.
    #[must_use]
    pub const fn new(trigger: Entity) -> Self {
        Self(trigger)
    }

    /// The owning closed-control (trigger) entity.
    #[must_use]
    pub const fn trigger(&self) -> Entity {
        self.0
    }
}

/// Non-generic marker on EVERY option row (a [`Button`]) inside a [`DropdownPopup`].
///
/// Spawned alongside the generic [`DropdownItem<T>`] on each option row. It exists so the
/// NON-generic [`theme_interaction`](crate::theme_interaction) painter can EXCLUDE option
/// rows (`Without<DropdownItemMarker>`) — the generic `DropdownItem<T>` cannot appear in a
/// non-generic query filter. Without this exclusion the shared button painter would clobber
/// the dropdown-owned `option_bg` / `option_highlight_bg` fills with the global theme's
/// hover/press colors, leaving a stray highlight bar and no per-option highlight (GTW-499).
/// The option-row look is OWNED by [`paint_dropdown_option_highlight`], the
/// [`ActiveButton`](crate::ActiveButton) / [`Segment`](crate::Segment) exclusion precedent.
/// A unit marker — presence alone is the signal (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct DropdownItemMarker;

/// Marker on one OPTION row (a [`Button`]) inside a [`DropdownPopup`].
///
/// Generic over the identity. Carries the option's [`id`](DropdownItem::id), its
/// [`index`](DropdownItem::index) into [`DropdownOptions`], and the owning
/// [`DropdownAnchor`] root, so [`select_option_on_press`] / [`activate_focused_option`] can
/// report the chosen option back to its control. The option button is EXCLUDED from the
/// generic button-interaction painter via the sibling [`DropdownItemMarker`] so its look is
/// not clobbered (the segment-exclusion precedent), and painted by
/// [`paint_dropdown_option_highlight`] instead.
#[expect(
    clippy::derive_partial_eq_without_eq,
    reason = "T: OptionId is only PartialEq-bound, so Eq cannot be derived without \
    over-constraining callers"
)]
#[derive(Component, Clone, PartialEq, Debug)]
pub struct DropdownItem<T: OptionId> {
    /// The option's identity.
    id:    T,
    /// The option's slot into [`DropdownOptions`].
    index: SelectedIndex,
}

impl<T: OptionId> DropdownItem<T> {
    /// Builds an option-row marker from its identity and slot index.
    #[must_use]
    const fn new(id: T, index: usize) -> Self {
        Self {
            id,
            index: SelectedIndex::new(index),
        }
    }

    /// The option's identity.
    #[must_use]
    pub const fn id(&self) -> &T {
        &self.id
    }

    /// The option's slot into [`DropdownOptions`].
    #[must_use]
    pub const fn index(&self) -> SelectedIndex {
        self.index
    }
}

/// The color set a [`Dropdown<T>`] paints itself with.
///
/// Pure UI plumbing ([`Color`](bevy::prelude::Color)s): the closed control's fill + text,
/// the popup's fill, each option row's resting fill + text, and the option row HIGHLIGHT
/// fill. Stored as a [`Component`] on the root so [`open_dropdown`] can re-derive the popup
/// look from it without the caller re-passing colors. The derived [`Default`]
/// (all-transparent) is a spawn-seed sentinel only — a builder always supplies real colors.
#[derive(Component, Clone, Copy, PartialEq, Debug, Default)]
pub struct DropdownColors {
    /// The closed control's background fill.
    pub control_bg:          Color,
    /// The closed control's (and option rows') text color.
    pub text:                Color,
    /// The floating popup's background fill.
    pub popup_bg:            Color,
    /// Each option row's RESTING background fill (when neither hovered/pressed nor selected).
    pub option_bg:           Color,
    /// The background fill of the option row that is HOVERED / focused / pressed, or is the
    /// currently-selected option — a DISTINCT highlight color so the active row reads as
    /// highlighted (GTW-499). [`paint_dropdown_option_highlight`] writes this onto the
    /// matching row and `option_bg` onto every other row, each frame an open list exists.
    pub option_highlight_bg: Color,
}

/// A buffered Bevy **message** emitted when a [`Dropdown<T>`] selection changes
/// (bevy-traps rule 4: buffered events are messages in 0.19).
///
/// Carries the control's [`Entity`] (its identity — it holds the caller's marker) and the
/// chosen option's `T` id. `gdtf_ui` cannot know what a choice MEANS, so it reports only
/// *which* control and *which* id; a statically-typed downstream
/// `MessageReader<DropdownSelectionChanged<T>>` maps it to an action.
#[expect(
    clippy::derive_partial_eq_without_eq,
    reason = "T: OptionId is only PartialEq-bound, so Eq cannot be derived without \
    over-constraining callers"
)]
#[derive(Message, Clone, PartialEq, Debug)]
pub struct DropdownSelectionChanged<T: OptionId> {
    /// The dropdown root that changed (carries the caller's identity marker).
    control: Entity,
    /// The newly-selected option's identity.
    id:      T,
}

impl<T: OptionId> DropdownSelectionChanged<T> {
    /// Build a selection-changed message naming the control [`Entity`] and the chosen `T`
    /// id — the producer-side constructor (symmetric with the field commits' `new`), so a
    /// caller that drives a selection (or a test exercising a selection listener's real code
    /// path) can raise one without reaching the private fields.
    #[must_use]
    pub const fn new(control: Entity, id: T) -> Self {
        Self { control, id }
    }

    /// The dropdown control entity whose selection changed.
    #[must_use]
    pub const fn control(&self) -> Entity {
        self.control
    }

    /// The newly-selected option's identity.
    #[must_use]
    pub const fn id(&self) -> &T {
        &self.id
    }
}

/// A buffered Bevy **message** requesting that all OPEN dropdowns close (an `Escape` press).
///
/// Dropdown-OWNED (not part of the shared focus bridge): [`dismiss_dropdowns_on_escape`]
/// emits it only when an open dropdown exists, and [`close_dropdowns_on_dismiss_request`]
/// consumes it to close WITHOUT changing the selection — the keyboard twin of the backdrop
/// outside-click. A [`Message`] (bevy-traps rule 4) so it is synthesizable in a test.
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct DropdownDismissRequest;

/// Spawns a [`Dropdown<T>`] closed control showing the current selection and returns the
/// ROOT [`Entity`].
///
/// `options` is the full option list (identity + label); `selected` is the initially-shown
/// option index (clamped); `colors` are the control / popup / option fills + text;
/// `marker` is any [`Bundle`] the caller wants on the root — typically its own identity
/// marker so the [`DropdownSelectionChanged`] listener can map it to an action.
///
/// The control is a [`Button`] (so `bevy_ui`'s built-in `ui_focus_system` drives its
/// [`Interaction`](bevy::ui::Interaction) from the mouse — bevy-traps #6), carrying its
/// option list, colors, [`SelectedIndex`], a [`Closed`](DropdownState::Closed)
/// [`DropdownState`], and a [`DropdownLabel`] text child showing the selected option's
/// label. The popup is NOT built here — [`open_dropdown`] builds it on click and
/// `select_dropdown_option` mutates the shown label in place on a selection (never a
/// respawn — [[ui-mutate-not-respawn]]).
///
/// An empty `options` yields an inert control (it shows nothing and opens an empty list).
pub fn spawn_dropdown<T: OptionId>(
    commands: &mut Commands,
    options: Vec<DropdownOption<T>>,
    selected: usize,
    colors: DropdownColors,
    marker: impl Bundle,
) -> Entity {
    let selected = clamp_index(selected, options.len());
    let current = options
        .get(selected)
        .map_or_else(String::new, |opt| opt.label().to_string());
    // The closed control: a Button row showing the current label. The generic `Dropdown<T>`,
    // `DropdownOptions<T>`, and the caller's `marker` are generic / non-`bsn!`-grammar types,
    // so the whole control is authored with plain `Commands::spawn` (the generic-bundle case
    // the segment / switch builders sidestep with `.insert` — here the entire widget is
    // generic, so plain spawn is the cleaner mirror).
    let control = commands
        .spawn((
            Dropdown::<T>::default(),
            Button,
            DropdownState::Closed,
            DropdownOptions::new(options),
            SelectedIndex::new(selected),
            colors,
            BackgroundColor(colors.control_bg),
            control_node(),
        ))
        .insert(marker)
        .id();
    let label = commands
        .spawn((DropdownLabel, Text::new(current), TextColor(colors.text)))
        .id();
    commands.entity(control).add_child(label);
    control
}

/// Read-write [`Query`] data for one clicked closed control: its [`Entity`], its
/// [`Interaction`](bevy::ui::Interaction), its [`DropdownState`], its [`DropdownOptions`],
/// and its [`DropdownColors`].
///
/// Named to keep [`open_dropdown`]'s signature legible (clippy `type_complexity`).
type ClickedDropdown<T> = (
    Entity,
    &'static Interaction,
    &'static mut DropdownState,
    &'static DropdownOptions<T>,
    &'static DropdownColors,
);

/// The [`Query`] filter selecting a clicked [`Dropdown<T>`] this frame — named so
/// [`open_dropdown`]'s signature stays under the clippy `type_complexity` threshold.
type ClickedDropdownFilter<T> = (Changed<Interaction>, With<Dropdown<T>>);

/// Opens / closes a clicked [`Dropdown<T>`]: on the press edge it toggles the
/// [`DropdownState`], SPAWNING the floating popup + dismiss backdrop when opening (and
/// despawning them when closing — the same control pressed again).
///
/// `Changed<Interaction>` + the explicit `== Pressed` test means one toggle per click (the
/// press edge), never one per frame held. Opening spawns the popup as a TOP-LEVEL
/// absolutely-positioned node with `GlobalZIndex(DROPDOWN_POPUP_Z)` (above sibling
/// panels — bevy-traps #8) and one option [`Button`] per [`DropdownOption`], focuses the
/// first option via the existing [`set_initial_focus`](crate::focus_nav::set_initial_focus),
/// and registers the option rows as a vertical neighbor chain in the existing
/// [`DirectionalNavigationMap`] graph (so the existing arrow-key pipeline moves the
/// highlight — AC3). [`position_dropdown_popups`] (run after) reads the trigger geometry and
/// places the popup flush below the control.
///
/// Param-only — no `&mut World` (bevy-traps rule 7); the spawns go through [`Commands`].
/// Registered by [`UiPlugin`](crate::UiPlugin) in [`Update`].
pub fn open_dropdown<T: OptionId>(
    mut triggers: Query<ClickedDropdown<T>, ClickedDropdownFilter<T>>,
    mut commands: Commands,
    mut nav_map: ResMut<DirectionalNavigationMap>,
) {
    for (control, interaction, mut state, options, colors) in &mut triggers {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match *state {
            DropdownState::Open { popup, backdrop } => {
                // The same control pressed again: close it (the toggle), reverting to Closed.
                despawn_if_exists(&mut commands, popup);
                despawn_if_exists(&mut commands, backdrop);
                *state = DropdownState::Closed;
            }
            DropdownState::Closed => {
                let (popup, backdrop, rows) =
                    spawn_open_list(&mut commands, control, options, *colors);
                *state = DropdownState::Open { popup, backdrop };
                wire_option_navigation(&mut nav_map, &rows);
                if let Some(&first) = rows.first() {
                    set_initial_focus(&mut commands, first);
                }
            }
        }
    }
}

/// Spawns the floating popup (one option [`Button`] per option), the dismiss backdrop, and
/// returns `(popup, backdrop, option_row_entities)`.
///
/// The popup is a TOP-LEVEL node (no parent) so its [`PositionType::Absolute`] coordinates
/// are WINDOW-relative; [`GlobalZIndex`] propagates to the option-row children, so they
/// inherit the above-panels z without per-row annotation.
fn spawn_open_list<T: OptionId>(
    commands: &mut Commands,
    control: Entity,
    options: &DropdownOptions<T>,
    colors: DropdownColors,
) -> (Entity, Entity, Vec<Entity>) {
    // The full-screen invisible dismiss backdrop, one z-band below the popup.
    let backdrop = commands
        .spawn((
            DropdownBackdrop,
            DropdownAnchor::new(control),
            Button,
            BackgroundColor(Color::NONE),
            GlobalZIndex(DROPDOWN_BACKDROP_Z),
            backdrop_node(),
        ))
        .id();
    // The floating popup container.
    let popup = commands
        .spawn((
            DropdownPopup,
            DropdownAnchor::new(control),
            BackgroundColor(colors.popup_bg),
            GlobalZIndex(DROPDOWN_POPUP_Z),
            popup_node(),
        ))
        .id();
    // One option row per option, parented under the popup.
    let mut rows = Vec::with_capacity(options.options().len());
    for (index, option) in options.options().iter().enumerate() {
        let row = commands
            .spawn((
                DropdownItem::new(option.id().clone(), index),
                DropdownItemMarker,
                DropdownAnchor::new(control),
                Button,
                BackgroundColor(colors.option_bg),
                option_row_node(),
            ))
            .id();
        let text = commands
            .spawn((
                Text::new(option.label().to_string()),
                TextColor(colors.text),
            ))
            .id();
        commands.entity(row).add_child(text);
        commands.entity(popup).add_child(row);
        rows.push(row);
    }
    (popup, backdrop, rows)
}

/// Wires the option rows as a vertical neighbor chain in the existing
/// [`DirectionalNavigationMap`] graph, so the existing `ArrowUp`/`ArrowDown` → navigation
/// pipeline (AC3) moves the highlight DOWN / UP the list with no new nav code.
///
/// [`DirectionalNavigationMap::add_edges`] adds SYMMETRICAL edges between each consecutive
/// pair in the slice for the given octant — passing the rows top-to-bottom with
/// [`CompassOctant::South`] makes `ArrowDown` step DOWN the list and `ArrowUp` step UP it
/// (the opposite edge is added for free), so one call wires the whole chain both ways.
fn wire_option_navigation(nav_map: &mut DirectionalNavigationMap, rows: &[Entity]) {
    nav_map.add_edges(rows, CompassOctant::South);
}

/// Read-only [`Query`] data for one pressed OPTION row: its [`DropdownItem`] and its owning
/// [`DropdownAnchor`].
///
/// Named to keep [`select_option_on_press`]'s signature legible (clippy `type_complexity`).
type PressedOption<T> = (
    &'static DropdownItem<T>,
    &'static DropdownAnchor,
    &'static Interaction,
);

/// The [`Query`] filter selecting a pressed option row this frame — named so
/// [`select_option_on_press`]'s signature stays under the clippy `type_complexity` threshold.
type PressedOptionFilter<T> = (Changed<Interaction>, With<DropdownItem<T>>);

/// Selects a [`Dropdown<T>`] option when its row is PRESSED — closing the list, mutating the
/// shown label, and emitting [`DropdownSelectionChanged`].
///
/// `Changed<Interaction>` + the explicit `== Pressed` test means one selection per click.
/// Ordered `.before(`[`open_dropdown`]`)` by [`UiPlugin`](crate::UiPlugin) so the close it
/// performs is not re-opened by the trigger handler the same frame (the recipe's ordering).
///
/// Param-only — no `&mut World` (bevy-traps rule 7). Delegates the close + label mutate +
/// emit to `select_dropdown_option`.
pub fn select_option_on_press<T: OptionId>(
    options: Query<PressedOption<T>, PressedOptionFilter<T>>,
    mut controls: Query<(&mut DropdownState, &mut SelectedIndex), With<Dropdown<T>>>,
    labels: Query<&Children>,
    mut texts: Query<&mut Text, With<DropdownLabel>>,
    options_lists: Query<&DropdownOptions<T>>,
    mut commands: Commands,
    mut changed: MessageWriter<DropdownSelectionChanged<T>>,
) {
    for (item, anchor, interaction) in &options {
        if *interaction != Interaction::Pressed {
            continue;
        }
        select_dropdown_option(
            anchor.trigger(),
            item.index(),
            item.id().clone(),
            &mut controls,
            &labels,
            &mut texts,
            &options_lists,
            &mut commands,
            &mut changed,
        );
    }
}

/// Turns a focused option's `Enter` activation into a selection (AC3 keyboard select).
///
/// Reads the existing [`FocusActivated`](crate::focus_nav::FocusActivated) message
/// (emitted by the shared keyboard bridge on `Enter`), and if the activated entity is a
/// [`DropdownItem<T>`] performs the SAME selection as a click. Ordered
/// `.after(FocusNavSystems::Bridge)` by [`UiPlugin`](crate::UiPlugin) so the message is
/// populated before it is drained (bevy-traps rule 3).
#[expect(
    clippy::too_many_arguments,
    reason = "mirrors select_option_on_press: the close + label-mutate + emit needs the \
    control state, the label text, the options list, Commands, and the message writer; \
    the FocusActivated reader replaces the press query"
)]
pub fn activate_focused_option<T: OptionId>(
    mut activations: MessageReader<FocusActivated>,
    items: Query<(&DropdownItem<T>, &DropdownAnchor)>,
    mut controls: Query<(&mut DropdownState, &mut SelectedIndex), With<Dropdown<T>>>,
    labels: Query<&Children>,
    mut texts: Query<&mut Text, With<DropdownLabel>>,
    options_lists: Query<&DropdownOptions<T>>,
    mut commands: Commands,
    mut changed: MessageWriter<DropdownSelectionChanged<T>>,
) {
    for activated in activations.read() {
        let Ok((item, anchor)) = items.get(**activated) else {
            continue;
        };
        select_dropdown_option(
            anchor.trigger(),
            item.index(),
            item.id().clone(),
            &mut controls,
            &labels,
            &mut texts,
            &options_lists,
            &mut commands,
            &mut changed,
        );
    }
}

/// The shared selection path: closes the open list (despawns the popup + backdrop), MUTATES
/// the control's [`SelectedIndex`] + the shown [`DropdownLabel`] text in place
/// ([[ui-mutate-not-respawn]]), and emits [`DropdownSelectionChanged`] — but only when the
/// selection actually CHANGES (the `set_if_neq` no-op: re-picking the current option closes
/// the list and emits nothing).
#[expect(
    clippy::too_many_arguments,
    reason = "the one shared select path that both the press and the keyboard-activate \
    drivers delegate to; it needs the control state + index, the label text, the options \
    list, Commands (to despawn the popup), and the message writer"
)]
fn select_dropdown_option<T: OptionId>(
    control: Entity,
    index: SelectedIndex,
    id: T,
    controls: &mut Query<(&mut DropdownState, &mut SelectedIndex), With<Dropdown<T>>>,
    labels: &Query<&Children>,
    texts: &mut Query<&mut Text, With<DropdownLabel>>,
    options_lists: &Query<&DropdownOptions<T>>,
    commands: &mut Commands,
    changed: &mut MessageWriter<DropdownSelectionChanged<T>>,
) {
    let Ok((mut state, mut selected)) = controls.get_mut(control) else {
        return;
    };
    // Close the list regardless (a click / Enter on a row always closes).
    if let DropdownState::Open { popup, backdrop } = *state {
        despawn_if_exists(commands, popup);
        despawn_if_exists(commands, backdrop);
    }
    *state = DropdownState::Closed;
    // Only emit + relabel on a REAL selection change.
    if !selected.set_if_neq(index) {
        return;
    }
    // Mutate the shown label in place to the new option's label.
    if let Ok(opts) = options_lists.get(control)
        && let Some(option) = opts.options().get(*index)
        && let Ok(kids) = labels.get(control)
    {
        for &kid in kids {
            if let Ok(mut text) = texts.get_mut(kid) {
                let want = option.label().to_string();
                if text.0 != want {
                    text.0 = want;
                }
            }
        }
    }
    changed.write(DropdownSelectionChanged { control, id });
}

/// Read-only [`Query`] data for one option row's highlight paint: its
/// [`DropdownItem<T>`] (for the option index), its owning [`DropdownAnchor`] (to read the
/// control's selected index), its [`Interaction`](bevy::ui::Interaction) (hover / press),
/// its [`Entity`] (to compare against [`InputFocus`]), and its mutable
/// [`BackgroundColor`](bevy::ui::BackgroundColor) (the fill it writes).
type OptionHighlight<T> = (
    &'static DropdownItem<T>,
    &'static DropdownAnchor,
    &'static Interaction,
    Entity,
    &'static mut BackgroundColor,
);

/// Paints every open [`Dropdown<T>`] option row from the dropdown's OWN
/// [`DropdownColors`] each frame: the HIGHLIGHTED row gets
/// [`option_highlight_bg`](DropdownColors::option_highlight_bg), every other row gets
/// [`option_bg`](DropdownColors::option_bg) (GTW-499).
///
/// A row is HIGHLIGHTED when it is hovered / pressed ([`Interaction`](bevy::ui::Interaction)
/// is not [`None`](bevy::ui::Interaction::None)), OR it is the
/// [`InputFocus`](bevy::input_focus::InputFocus) target (keyboard / hover-driven focus — the
/// existing arrow-nav pipeline moves this), OR it is the control's currently-selected option
/// ([`SelectedIndex`]). This is the dropdown-OWNED look the shared
/// [`theme_interaction`](crate::theme_interaction) painter no longer touches (it excludes
/// [`DropdownItemMarker`]), so the highlight is a styled, dropdown-defined color — never the
/// global theme fill and never absent. Rows that are not part of an open list have no
/// `DropdownItem`, so they are untouched (a CLOSED control shows no option rows at all, so no
/// stray highlight leaks into the collapsed row).
///
/// Param-only — no `&mut World` (bevy-traps rule 7). Registered per option-id type by
/// [`register_dropdown`](crate::register_dropdown) in [`Update`]. It runs each frame (not
/// change-gated) so a freshly-spawned popup, an arrow-nav focus move, and a re-layout all repaint
/// deterministically.
pub fn paint_dropdown_option_highlight<T: OptionId>(
    mut rows: Query<OptionHighlight<T>, With<DropdownItemMarker>>,
    controls: Query<(&SelectedIndex, &DropdownColors), With<Dropdown<T>>>,
    focus: Option<Res<InputFocus>>,
) {
    let focused = focus.and_then(|f| f.get());
    for (item, anchor, interaction, entity, mut background) in &mut rows {
        let Ok((selected, colors)) = controls.get(anchor.trigger()) else {
            continue;
        };
        let highlighted = *interaction != Interaction::None
            || focused == Some(entity)
            || *item.index() == **selected;
        let want = if highlighted {
            colors.option_highlight_bg
        } else {
            colors.option_bg
        };
        if background.0 != want {
            background.0 = want;
        }
    }
}

/// Read-only [`Query`] data for one pressed dismiss backdrop: its owning [`DropdownAnchor`]
/// and its [`Interaction`](bevy::ui::Interaction).
type PressedBackdrop = (&'static DropdownAnchor, &'static Interaction);

/// Dismisses a [`Dropdown`] on an OUTSIDE click — the backdrop press — closing the list
/// WITHOUT changing the selection (AC2).
///
/// `Changed<Interaction>` + the explicit `== Pressed` test means one dismiss per click. It
/// resets the owning control's [`DropdownState`] to [`Closed`](DropdownState::Closed) and
/// despawns the popup + backdrop; it never touches [`SelectedIndex`] or emits
/// [`DropdownSelectionChanged`], so the selection is unchanged. Generic so it reaches the
/// right control family. Param-only (bevy-traps rule 7).
pub fn dismiss_on_backdrop_press<T: OptionId>(
    backdrops: Query<PressedBackdrop, (Changed<Interaction>, With<DropdownBackdrop>)>,
    mut controls: Query<&mut DropdownState, With<Dropdown<T>>>,
    mut commands: Commands,
) {
    for (anchor, interaction) in &backdrops {
        if *interaction != Interaction::Pressed {
            continue;
        }
        close_dropdown(anchor.trigger(), &mut controls, &mut commands);
    }
}

/// Emits a [`DropdownDismissRequest`] when `Escape` is pressed (AC3 keyboard close).
///
/// Dropdown-OWNED (not in the shared keyboard bridge, which owns arrows / Enter): `Escape`
/// is dropdown-specific, so it lives here rather than overloading
/// [`bridge_keyboard_navigation`](crate::focus_nav::bridge_keyboard_navigation). It emits the
/// request unconditionally on the press edge; the per-type
/// [`close_dropdowns_on_dismiss_request`] consumer is a no-op when no dropdown is open, so an
/// `Escape` with nothing open is harmless. (The [`any_dropdown_open`] run-condition is
/// available for a caller that prefers to gate the EMIT instead.) Takes
/// `Option<Res<ButtonInput<KeyCode>>>` so it is inert under a `MinimalPlugins` harness with no
/// `InputPlugin` (bevy-traps rule 1).
pub fn dismiss_dropdowns_on_escape(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut dismiss: MessageWriter<DropdownDismissRequest>,
) {
    let Some(keys) = keys else {
        return;
    };
    if keys.just_pressed(KeyCode::Escape) {
        dismiss.write(DropdownDismissRequest);
    }
}

/// Closes every OPEN [`Dropdown<T>`] on a [`DropdownDismissRequest`] — the keyboard twin of
/// the backdrop dismiss, WITHOUT changing the selection.
///
/// Drains the dismiss request (drops the count — one or many, all open dropdowns close) and
/// closes each control whose [`DropdownState`] is [`Open`](DropdownState::Open). Generic so
/// it reaches the right family. Param-only (bevy-traps rule 7).
pub fn close_dropdowns_on_dismiss_request<T: OptionId>(
    mut requests: MessageReader<DropdownDismissRequest>,
    mut controls: Query<(Entity, &mut DropdownState), With<Dropdown<T>>>,
    mut commands: Commands,
) {
    if requests.read().count() == 0 {
        return;
    }
    let to_close: Vec<Entity> = controls
        .iter()
        .filter_map(|(entity, state)| {
            matches!(*state, DropdownState::Open { .. }).then_some(entity)
        })
        .collect();
    for control in to_close {
        if let Ok((_, mut state)) = controls.get_mut(control)
            && let DropdownState::Open { popup, backdrop } = *state
        {
            despawn_if_exists(&mut commands, popup);
            despawn_if_exists(&mut commands, backdrop);
            *state = DropdownState::Closed;
        }
    }
}

/// Whether any [`Dropdown<T>`] is currently open — the run condition that gates
/// [`dismiss_dropdowns_on_escape`] so `Escape` only fires while a popup is live.
#[must_use]
pub fn any_dropdown_open<T: OptionId>(controls: Query<&DropdownState, With<Dropdown<T>>>) -> bool {
    controls
        .iter()
        .any(|state| matches!(*state, DropdownState::Open { .. }))
}

/// Read-only [`Query`] data for positioning one open popup: the trigger's
/// [`UiGlobalTransform`] (its screen-space CENTER) and its [`ComputedNode`] (its size).
type TriggerGeometry = (&'static UiGlobalTransform, &'static ComputedNode);

/// Places each open [`DropdownPopup`] flush BELOW its owning closed control, in WINDOW space
/// (bevy-traps #8 — UI positions come from [`UiGlobalTransform`] / [`ComputedNode`], NOT
/// [`GlobalTransform`]).
///
/// The trigger's [`UiGlobalTransform`] translation is its screen-space CENTER (UI transforms
/// are center-based in 0.19) and [`ComputedNode::size`] is its logical-pixel extent, so the
/// popup's `left` aligns to the control's left edge (`center.x - width/2`) and its `top` sits
/// at the control's bottom edge (`center.y + height/2`). The popup is a TOP-LEVEL node, so
/// these absolute coordinates are window-relative. Runs each frame so a resize / re-layout
/// re-places an open popup. Param-only (bevy-traps rule 7).
pub fn position_dropdown_popups(
    mut popups: Query<(&DropdownAnchor, &mut Node), With<DropdownPopup>>,
    triggers: Query<TriggerGeometry>,
) {
    for (anchor, mut node) in &mut popups {
        let Ok((transform, computed)) = triggers.get(anchor.trigger()) else {
            continue;
        };
        let center = transform.translation;
        let size = computed.size();
        let left = center.x - size.x / 2.0;
        let top = center.y + size.y / 2.0;
        node.left = Val::Px(left);
        node.top = Val::Px(top);
    }
}

/// Closes one control: despawns its popup + backdrop and resets its
/// [`DropdownState`] to [`Closed`](DropdownState::Closed). The shared close used by the
/// backdrop dismiss; it never touches [`SelectedIndex`] (the selection is unchanged).
fn close_dropdown<T: OptionId>(
    control: Entity,
    controls: &mut Query<&mut DropdownState, With<Dropdown<T>>>,
    commands: &mut Commands,
) {
    let Ok(mut state) = controls.get_mut(control) else {
        return;
    };
    if let DropdownState::Open { popup, backdrop } = *state {
        despawn_if_exists(commands, popup);
        despawn_if_exists(commands, backdrop);
    }
    *state = DropdownState::Closed;
}

/// Despawns `entity` (and its descendants) through [`Commands`] if it still exists.
///
/// `Commands::despawn` on a stale id is a logged no-op in 0.19, but the guard keeps the
/// close path tidy when a popup was already removed by a sibling close.
fn despawn_if_exists(commands: &mut Commands, entity: Entity) {
    if let Ok(mut entity_commands) = commands.get_entity(entity) {
        entity_commands.despawn();
    }
}

/// Clamps a requested index to a valid option slot (last option if out of range, `0` if the
/// list is empty — an inert value the label lookup degrades on).
const fn clamp_index(index: usize, count: usize) -> usize {
    if count == 0 {
        0
    } else if index >= count {
        count - 1
    } else {
        index
    }
}

/// The closed control's [`Node`]: a padded row that sizes to its label, with a subtle
/// rounded box. Relative units (`Vw`/`Vh`) per the responsive-UI rule.
fn control_node() -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceBetween,
        padding: UiRect::axes(Val::Vw(CONTROL_PAD_X_VW), Val::Vh(CONTROL_PAD_Y_VH)),
        column_gap: Val::Vw(CONTROL_PAD_X_VW),
        ..default()
    }
}

/// The floating popup's [`Node`]: a top-level absolutely-positioned column that sizes to its
/// option rows. `left`/`top` are set each frame by [`position_dropdown_popups`].
fn popup_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        flex_direction: FlexDirection::Column,
        min_width: Val::Vw(POPUP_MIN_WIDTH_VW),
        overflow: Overflow::clip(),
        ..default()
    }
}

/// One option row's [`Node`]: a padded full-width row inside the popup column.
fn option_row_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        padding: UiRect::axes(Val::Vw(CONTROL_PAD_X_VW), Val::Vh(CONTROL_PAD_Y_VH)),
        ..default()
    }
}

/// The dismiss backdrop's [`Node`]: a full-window absolutely-positioned invisible cover.
fn backdrop_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::ZERO,
        top: Val::ZERO,
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        ..default()
    }
}

/// The [`GlobalZIndex`] band the floating popup renders on — a framework-plumbing `const`
/// fed straight to a [`GlobalZIndex`] (the framework carve-out, not a domain value).
///
/// Set to `30` — one clean band STRICTLY ABOVE the highest existing UI layer (the
/// contextual panel's `GlobalZIndex(20)` = `CONTEXTUAL_PANEL_Z`; the bottom bar is `10`,
/// the weapon/stance cluster + combat log `11`). With a default `GlobalZIndex` (`0`) the
/// floating list would be painted OVER by those opaque siblings and draw nothing despite
/// being laid out — the GTW-294 occlusion trap (`bevy-traps.md` #8). The popup carries this
/// z and it PROPAGATES to the option-row children, so the whole list clears the panels.
const DROPDOWN_POPUP_Z: i32 = 30;

/// The [`GlobalZIndex`] band the dismiss backdrop renders on — a framework-plumbing `const`
/// (the framework carve-out, not a domain value).
///
/// Set to `29` — one band BELOW `DROPDOWN_POPUP_Z` (so the popup draws over the backdrop)
/// yet ABOVE the contextual panel's `20`, so a click anywhere outside the popup but over a
/// panel still lands on the backdrop and dismisses the list.
const DROPDOWN_BACKDROP_Z: i32 = 29;

/// The closed control's horizontal inner padding, in viewport-width units (also the option
/// rows' horizontal padding + the control's label/chevron gap). Calibrated 8px / 1280 * 100.
const CONTROL_PAD_X_VW: f32 = 0.625;

/// The closed control's vertical inner padding, in viewport-height units (also the option
/// rows' vertical padding). Calibrated 6px / 720 * 100.
const CONTROL_PAD_Y_VH: f32 = 0.83333;

/// The floating popup's minimum width, in viewport-width units, so a short option list still
/// reads as a list rather than collapsing to its narrowest label. Calibrated 120px / 1280 * 100.
const POPUP_MIN_WIDTH_VW: f32 = 9.375;
