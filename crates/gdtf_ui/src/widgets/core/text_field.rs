//! The [`TextField`] / [`NumericField`] widgets: focus-driven, keyboard-editable text
//! and clamped-numeric input fields (GTW-411).
//!
//! Bevy 0.19 ships NO built-in text-input control, so these widgets ASSEMBLE one from the
//! [`bevy_input_focus`](bevy::input_focus) primitives the rest of the hand-rolled UI already
//! rides. A field is a themed [`Button`] (so `bevy_ui`'s built-in `ui_focus_system` drives its
//! [`Interaction`](bevy::ui::Interaction) from the mouse — bevy-traps #6) carrying an
//! [`EditBuffer`] (the in-progress text) and a [`Text`] child that mirrors the buffer plus a
//! thin [`Caret`] node drawn at the text END. Pressing the control captures
//! [`InputFocus`](bevy::input_focus::InputFocus); while focused, a
//! [`FocusedInput<KeyboardInput>`](bevy::input_focus::FocusedInput) OBSERVER appends typed
//! characters / handles Backspace / commits on Enter / reverts on Escape.
//!
//! ## Two field flavors, both typed (no-bare-types)
//!
//! - [`TextField`] commits its edited buffer as a [`CommittedTextValue`] — a domain newtype
//!   over the string, never a bare `String` — in a [`TextFieldCommitted`] message.
//! - [`NumericField`] CLAMPS its parsed buffer to a [`NumericRange`] and commits a
//!   [`CommittedNumericValue`] (a newtype over the number) in a [`NumericFieldCommitted`]
//!   message. Invalid / empty input is REVERTED to the last committed value (never panics —
//!   the workspace denies `unwrap`/`expect`/`panic`).
//!
//! ## Editing scope (faithful to the GTW-411 ACs)
//!
//! The ACs require focus + keyboard editing + a rendered caret + a typed clamped/validated
//! commit. They do NOT require text SELECTION, MID-buffer cursor movement, or IME candidate
//! windows — those are disclosed Bevy 0.19 gaps and are deliberately out of scope. The
//! editing model is therefore END-ONLY: typed characters APPEND, Backspace pops the last
//! character, and the caret sits at the text end. That is the full AC scope, not a reduction.
//!
//! ## Keyboard via an observer, not a polling system
//!
//! `bevy_input_focus`'s `dispatch_focused_input::<KeyboardInput>` (in `DefaultPlugins`,
//! `PreUpdate`) triggers a [`FocusedInput<KeyboardInput>`](bevy::input_focus::FocusedInput)
//! ENTITY-EVENT at the focused entity each keypress. [`handle_text_field_key`] is a GLOBAL
//! observer ([`register_text_field`] adds it via `App::add_observer`) that filters to a
//! focused [`TextField`] / [`NumericField`] and mutates its [`EditBuffer`]. The observer fires
//! inline in `PreUpdate`, so the `Update`-scheduled [`sync_edit_buffer_to_text`] sees the
//! mutated buffer the SAME frame (bevy-traps rule 3).
//!
//! ## Commit on Enter AND on blur
//!
//! Enter commits inside the observer. Losing focus also commits: [`commit_on_focus_lost`] is a
//! global observer on the [`FocusLost`](bevy::input_focus::FocusLost) entity-event, so
//! clicking away from a field still reports its edited value (the menu-form expectation).

use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::{FocusCause, FocusLost, FocusedInput, InputFocus},
    prelude::*,
    ui::{
        AlignItems, BackgroundColor, FlexDirection, Interaction, Node, UiRect, Val, widget::Button,
    },
};

/// The in-progress edited text of a [`TextField`] / [`NumericField`], held on the field root.
///
/// A named newtype over the buffer string rather than a bare `String` (no-bare-types rule 5):
/// the inner is PRIVATE and mutated only through [`push_str`](EditBuffer::push_str) /
/// [`pop`](EditBuffer::pop) / [`clear`](EditBuffer::clear) / [`set`](EditBuffer::set), so the
/// editing invariants (end-only append, no mid-buffer surgery) can never be sidestepped. The
/// `Changed<EditBuffer>` filter on [`sync_edit_buffer_to_text`] redraws the shown text only on
/// a real edit.
#[derive(Component, Deref, Clone, PartialEq, Eq, Debug, Default)]
pub struct EditBuffer(String);

impl EditBuffer {
    /// Wraps an initial buffer value.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// Appends `text` at the END of the buffer (the end-only editing model — AC scope).
    pub fn push_str(&mut self, text: &str) {
        self.0.push_str(text);
    }

    /// Removes the last character (Backspace). A no-op on an empty buffer (never panics).
    pub fn pop(&mut self) {
        self.0.pop();
    }

    /// Empties the buffer.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Replaces the buffer with `text` (the Escape revert / commit-normalize path).
    pub fn set(&mut self, text: impl Into<String>) {
        self.0 = text.into();
    }

    /// The current buffer contents.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Marker on the ROOT (a [`Button`]) of a [`spawn_text_field`] widget.
///
/// Presence alone is the signal (no-bare-types rule): the keyboard observer and the
/// commit/sync systems query `With<TextField>` to reach the editable text fields. The caller
/// attaches its own identity marker alongside this (via the builder's `marker: impl Bundle`)
/// so the [`TextFieldCommitted`] listener can map a commit to an action.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct TextField;

/// Marker on the ROOT (a [`Button`]) of a [`spawn_numeric_field`] widget.
///
/// Presence alone is the signal (no-bare-types rule). A numeric field carries a
/// [`NumericRange`] (its clamp bounds) and a [`CommittedNumericValue`] (its last good value);
/// the keyboard observer routes a focused numeric field's Enter / blur through the parse +
/// clamp path rather than the plain-text commit.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct NumericField;

/// Marker on the [`Text`] child of a field, mirroring the [`EditBuffer`].
///
/// [`sync_edit_buffer_to_text`] mutates its [`Text`] in place to the current buffer — never
/// despawning it ([[ui-mutate-not-respawn]]). A unit marker (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct FieldText;

/// Marker on the thin CARET child of a field — a solid sliver drawn AFTER the text child so it
/// sits at the text END (the end-only editing model).
///
/// A unit marker (no-bare-types rule). A static visible caret satisfies AC4; no blink timer is
/// added (it is explicitly optional and out of the AC set).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Caret;

/// The committed value of a [`TextField`] — the edited buffer reported on commit.
///
/// A named newtype over the string rather than a bare `String` (no-bare-types rule 5): the
/// inner is PRIVATE, read through [`value`](CommittedTextValue::value). A text-field commit is
/// a domain value (a gang name, a ganger name, a prefab name), not arbitrary text.
#[derive(Component, Deref, Clone, PartialEq, Eq, Debug, Default)]
pub struct CommittedTextValue(String);

impl CommittedTextValue {
    /// Wraps a committed text value.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The committed text.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}

/// The numeric value type a [`NumericField`] edits, clamps, and commits.
///
/// A field is generic over the integer it represents; the bound set
/// (`Copy + PartialOrd + FromStr + Display + Send + Sync + 'static`) is exactly what the
/// parse → clamp → display pipeline needs, satisfied through the blanket impl by `i64` /
/// `u32` / the caller's own integer newtype with no methods to write.
pub trait NumericValue:
    Copy + PartialOrd + core::str::FromStr + core::fmt::Display + Send + Sync + 'static
{
}

impl<T: Copy + PartialOrd + core::str::FromStr + core::fmt::Display + Send + Sync + 'static>
    NumericValue for T
{
}

/// The inclusive valid range a [`NumericField`] clamps its committed value into.
///
/// A named pair rather than a bare `(N, N)`: both inner bounds are PRIVATE (no-bare-types
/// rule 5), set through [`new`](NumericRange::new) and applied through
/// [`clamp`](NumericRange::clamp). The component is held on the field root so the keyboard
/// observer can clamp without the caller re-passing the bounds.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct NumericRange<N: NumericValue> {
    /// The lowest accepted value (inclusive).
    min: N,
    /// The highest accepted value (inclusive).
    max: N,
}

impl<N: NumericValue> NumericRange<N> {
    /// Builds a range from its inclusive `min` / `max` bounds.
    ///
    /// If the caller passes them reversed, [`clamp`](NumericRange::clamp) still behaves
    /// (it pins below `min` first, then above `max`), so a degenerate range never panics.
    #[must_use]
    pub const fn new(min: N, max: N) -> Self {
        Self { min, max }
    }

    /// The inclusive lower bound.
    #[must_use]
    pub const fn min(&self) -> N {
        self.min
    }

    /// The inclusive upper bound.
    #[must_use]
    pub const fn max(&self) -> N {
        self.max
    }

    /// Clamps `value` into `[min, max]` (`PartialOrd`, so a NaN-like incomparable value passes
    /// through unchanged rather than panicking — integers can never hit that path).
    #[must_use]
    pub fn clamp(&self, value: N) -> N {
        if value < self.min {
            self.min
        } else if value > self.max {
            self.max
        } else {
            value
        }
    }
}

/// The committed value of a [`NumericField`] — the parsed, CLAMPED number reported on commit.
///
/// A named newtype over the number rather than a bare integer (no-bare-types rule 5): the
/// inner is PRIVATE, read through [`value`](CommittedNumericValue::value). It is held on the
/// field root as the LAST-GOOD value, so an invalid / empty buffer reverts to it.
#[derive(Component, Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct CommittedNumericValue<N: NumericValue>(N);

impl<N: NumericValue> CommittedNumericValue<N> {
    /// Wraps a committed numeric value.
    #[must_use]
    pub const fn new(value: N) -> Self {
        Self(value)
    }

    /// The committed number.
    #[must_use]
    pub const fn value(&self) -> N {
        self.0
    }
}

/// The color set a field paints itself with — pure UI plumbing
/// ([`Color`](bevy::prelude::Color)s), not domain values.
///
/// Held on the field root so a re-derive can re-read it without the caller re-passing colors.
/// The derived [`Default`] (all-transparent) is a spawn-seed sentinel only — a builder always
/// supplies real colors.
#[derive(Component, Clone, Copy, PartialEq, Debug, Default)]
pub struct FieldColors {
    /// The field's background fill.
    pub background: Color,
    /// The edited text color.
    pub text:       Color,
    /// The caret sliver color.
    pub caret:      Color,
}

/// A buffered Bevy **message** emitted when a [`TextField`] COMMITS its edited value
/// (bevy-traps rule 4: buffered events are messages in 0.19).
///
/// Carries the field's [`Entity`] (its identity — it holds the caller's marker) and the
/// committed [`CommittedTextValue`]. Emitted on Enter and on blur. Both inners are PRIVATE,
/// read through accessors (no-bare-types rule 5).
#[derive(Message, Clone, PartialEq, Eq, Debug)]
pub struct TextFieldCommitted {
    /// The text-field root that committed (carries the caller's identity marker).
    field: Entity,
    /// The committed text value.
    value: CommittedTextValue,
}

impl TextFieldCommitted {
    /// Build a commit message naming the field that committed and its
    /// [`CommittedTextValue`] — the producer-side constructor (symmetric with
    /// [`FocusActivated::new`](crate::focus_nav::FocusActivated)), so a caller that drives a
    /// commit (or a test exercising a commit listener's real code path) can raise one without
    /// reaching the private fields.
    #[must_use]
    pub const fn new(field: Entity, value: CommittedTextValue) -> Self {
        Self { field, value }
    }

    /// The text-field entity whose value committed.
    #[must_use]
    pub const fn field(&self) -> Entity {
        self.field
    }

    /// The committed text value.
    #[must_use]
    pub const fn value(&self) -> &CommittedTextValue {
        &self.value
    }
}

/// A buffered Bevy **message** emitted when a [`NumericField`] COMMITS its clamped value
/// (bevy-traps rule 4).
///
/// Carries the field's [`Entity`] and the committed (clamped) [`CommittedNumericValue`].
/// Emitted on Enter and on blur. Both inners are PRIVATE, read through accessors
/// (no-bare-types rule 5).
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub struct NumericFieldCommitted<N: NumericValue> {
    /// The numeric-field root that committed (carries the caller's identity marker).
    field: Entity,
    /// The committed, clamped numeric value.
    value: CommittedNumericValue<N>,
}

impl<N: NumericValue> NumericFieldCommitted<N> {
    /// Build a commit message naming the field that committed and its (already-clamped)
    /// [`CommittedNumericValue`] — the producer-side constructor (symmetric with
    /// [`TextFieldCommitted::new`]), so a caller that drives a commit (or a test exercising a
    /// commit listener's real code path) can raise one without reaching the private fields.
    #[must_use]
    pub const fn new(field: Entity, value: CommittedNumericValue<N>) -> Self {
        Self { field, value }
    }

    /// The numeric-field entity whose value committed.
    #[must_use]
    pub const fn field(&self) -> Entity {
        self.field
    }

    /// The committed, clamped numeric value.
    #[must_use]
    pub const fn value(&self) -> CommittedNumericValue<N> {
        self.value
    }
}

/// Spawns a [`TextField`] editable text widget seeded with `initial` and returns the ROOT
/// [`Entity`].
///
/// `initial` is the starting [`CommittedTextValue`] (also the shown text + the Escape-revert
/// target); `colors` are the background / text / caret colors; `marker` is any [`Bundle`] the
/// caller wants on the root — typically its own identity marker so the [`TextFieldCommitted`]
/// listener can map it to an action.
///
/// The root is a [`Button`] (so `ui_focus_system` drives its
/// [`Interaction`](bevy::ui::Interaction) — bevy-traps #6) carrying the [`EditBuffer`] (seeded
/// from `initial`), the [`CommittedTextValue`] (the last-good value), the colors, a
/// [`FieldText`] text child mirroring the buffer, and a [`Caret`] sliver child after it.
#[must_use]
pub fn spawn_text_field(
    commands: &mut Commands,
    initial: CommittedTextValue,
    colors: FieldColors,
    marker: impl Bundle,
) -> Entity {
    let buffer = EditBuffer::new(initial.value());
    let field = commands
        .spawn((
            TextField,
            Button,
            buffer,
            initial,
            colors,
            BackgroundColor(colors.background),
            field_node(),
        ))
        .insert(marker)
        .id();
    spawn_field_children(commands, field, colors);
    field
}

/// Spawns a [`NumericField`] clamped-numeric widget and returns the ROOT [`Entity`].
///
/// `initial` is the starting numeric value (CLAMPED into `range` immediately, then shown +
/// stored as the last-good value); `range` is the inclusive clamp bounds; `colors` /  `marker`
/// match [`spawn_text_field`]. The root carries the [`NumericField`] marker, the
/// [`NumericRange`], the clamped [`CommittedNumericValue`], an [`EditBuffer`] seeded with the
/// clamped value's text, and the same text + caret children.
#[must_use]
pub fn spawn_numeric_field<N: NumericValue>(
    commands: &mut Commands,
    initial: N,
    range: NumericRange<N>,
    colors: FieldColors,
    marker: impl Bundle,
) -> Entity {
    let clamped = range.clamp(initial);
    let buffer = EditBuffer::new(clamped.to_string());
    let field = commands
        .spawn((
            NumericField,
            Button,
            buffer,
            range,
            CommittedNumericValue::new(clamped),
            colors,
            BackgroundColor(colors.background),
            field_node(),
        ))
        .insert(marker)
        .id();
    spawn_field_children(commands, field, colors);
    field
}

/// Spawns the shared [`FieldText`] + [`Caret`] children under a field root.
///
/// The text child mirrors the buffer; the caret is a thin solid sliver placed AFTER the text
/// (in the row's flex order) so it reads as an end-of-text cursor.
fn spawn_field_children(commands: &mut Commands, field: Entity, colors: FieldColors) {
    let text = commands
        .spawn((FieldText, Text::new(String::new()), TextColor(colors.text)))
        .id();
    let caret = commands
        .spawn((Caret, BackgroundColor(colors.caret), caret_node()))
        .id();
    commands.entity(field).add_children(&[text, caret]);
}

/// Read-write [`Query`] data for the keyboard observer's target field: its optional
/// [`NumericField`] marker (to route the commit), its [`EditBuffer`], its last-good
/// [`CommittedTextValue`], and its children (to find the [`FieldText`]).
///
/// Named so [`handle_text_field_key`]'s signature stays under the clippy `type_complexity`
/// threshold.
type FocusedField<'a> = (
    &'a mut EditBuffer,
    Option<&'a CommittedTextValue>,
    Has<NumericField>,
);

/// The [`Query`] filter matching EITHER field flavor — named so the blur / focus systems'
/// signatures stay under the clippy `type_complexity` threshold.
type AnyField = Or<(With<TextField>, With<NumericField>)>;

/// The keyboard OBSERVER: appends typed characters / handles Backspace / commits on Enter /
/// reverts on Escape for the FOCUSED text or numeric field (AC1, AC3).
///
/// Triggered by `bevy_input_focus`'s `dispatch_focused_input::<KeyboardInput>` (a
/// [`FocusedInput<KeyboardInput>`](bevy::input_focus::FocusedInput) entity-event) at the
/// focused entity each keypress. It filters to the event's `focused_entity`, ignores
/// [`ButtonState::Released`], then:
///
/// - [`Key::Character`] → appends `text` (the IME-correct
///   [`SmolStr`](bevy::input::keyboard::Key) Bevy resolved) to the [`EditBuffer`]. The raw
///   `input.text` (not the logical key string) is used so the typed glyph respects locale.
/// - [`Key::Space`] → appends a space (Space is a NAMED key, not a `Character`).
/// - [`Key::Backspace`] → pops the last character.
/// - [`Key::Enter`] → commits via the shared commit path (text or clamped numeric).
/// - [`Key::Escape`] → reverts the buffer to the last-good committed value (no commit).
///
/// It NEVER panics: a numeric parse uses `parse::<N>().ok()` and an unparseable / empty buffer
/// reverts to the last-good value (handled in the commit path). Added as a global observer by
/// [`register_text_field`].
pub fn handle_text_field_key(
    key_event: On<FocusedInput<KeyboardInput>>,
    mut fields: Query<FocusedField>,
    text_commits: Option<ResMut<Messages<TextFieldCommitted>>>,
    mut commands: Commands,
) {
    let field = key_event.focused_entity;
    let input = &key_event.input;
    if input.state == ButtonState::Released {
        return;
    }
    let Ok((mut buffer, committed, is_numeric)) = fields.get_mut(field) else {
        return;
    };
    match &input.logical_key {
        Key::Character(_) => {
            // Prefer the IME-resolved `text` (locale-correct); fall back to the logical key
            // string when a platform omits `text` for a character key.
            if let Some(text) = input.text.as_deref() {
                buffer.push_str(text);
            } else if let Key::Character(s) = &input.logical_key {
                buffer.push_str(s);
            }
        }
        Key::Space => buffer.push_str(" "),
        Key::Backspace => buffer.pop(),
        Key::Enter => {
            if is_numeric {
                // The numeric commit needs the typed `N`; it cannot run from this
                // type-erased observer, so it is queued as a one-shot command that resolves
                // the field's concrete `NumericField`-typed commit (registered per `N`).
                commands.queue(move |world: &mut World| {
                    commit_numeric_field_entity(world, field);
                });
            } else {
                // Text commit: emit the message + normalize the last-good value in place
                // (the last-good write is queued because it mutates the same `CommittedTextValue`
                // the Escape branch reads through the immutable query borrow).
                let value = CommittedTextValue::new(buffer.as_str());
                if let Some(mut commits) = text_commits {
                    commits.write(TextFieldCommitted {
                        field,
                        value: value.clone(),
                    });
                }
                commands.queue(move |world: &mut World| {
                    if let Some(mut committed) = world.get_mut::<CommittedTextValue>(field) {
                        *committed = value;
                    }
                });
            }
        }
        Key::Escape => {
            // Revert the buffer to the last-good value WITHOUT committing.
            if is_numeric {
                commands.queue(move |world: &mut World| {
                    revert_numeric_field_entity(world, field);
                });
            } else if let Some(committed) = committed {
                buffer.set(committed.value());
            }
        }
        _ => {}
    }
}

/// Global observer committing a [`TextField`] / [`NumericField`] on BLUR
/// ([`FocusLost`](bevy::input_focus::FocusLost)) — clicking away still reports the edited
/// value (AC1 commit-on-blur).
///
/// For a text field it emits a [`TextFieldCommitted`] + updates the last-good value; for a
/// numeric field it queues the typed clamped commit. A [`FocusLost`] for a non-field entity is
/// ignored. Added as a global observer by [`register_text_field`].
pub fn commit_on_focus_lost(
    lost: On<FocusLost>,
    fields: Query<(&EditBuffer, Has<NumericField>), AnyField>,
    text_commits: Option<ResMut<Messages<TextFieldCommitted>>>,
    mut commands: Commands,
) {
    let field = lost.entity;
    let Ok((buffer, is_numeric)) = fields.get(field) else {
        return;
    };
    if is_numeric {
        commands.queue(move |world: &mut World| {
            commit_numeric_field_entity(world, field);
        });
        return;
    }
    let value = CommittedTextValue::new(buffer.as_str());
    if let Some(mut commits) = text_commits {
        commits.write(TextFieldCommitted {
            field,
            value: value.clone(),
        });
    }
    commands.queue(move |world: &mut World| {
        if let Some(mut committed) = world.get_mut::<CommittedTextValue>(field) {
            *committed = value;
        }
    });
}

/// Parses, CLAMPS, commits, and normalizes one numeric field's buffer (the shared numeric
/// commit path Enter / blur both route through). Registered per concrete `N` by
/// [`register_numeric_field`].
///
/// Parses `buffer.as_str().trim()` via `parse::<N>().ok()` (NO panic); an unparseable / empty
/// buffer REVERTS to the last-good [`CommittedNumericValue`]. A parsed value is clamped into
/// the [`NumericRange`], stored as the new last-good value, written to the
/// [`NumericFieldCommitted<N>`] buffer, and the buffer text is normalized to the clamped
/// number's display form.
///
/// `&mut World` is justified here (bevy-traps rule 7): the type-erased keyboard observer
/// cannot name `N`, so it queues this concrete-`N` command that reads the field's components,
/// the typed message buffer, and re-writes them together — work that genuinely cannot be a
/// normal param system reachable from the erased observer.
fn commit_numeric_field<N: NumericValue>(world: &mut World, field: Entity) {
    let Some(range) = world.get::<NumericRange<N>>(field).copied() else {
        return;
    };
    let Some(last_good) = world.get::<CommittedNumericValue<N>>(field).copied() else {
        return;
    };
    let parsed = world
        .get::<EditBuffer>(field)
        .and_then(|buffer| buffer.as_str().trim().parse::<N>().ok());
    // Invalid / empty input reverts to the last-good value (the documented reject policy);
    // a valid value is clamped into range.
    let committed = parsed.map_or_else(|| last_good.value(), |value| range.clamp(value));
    if let Some(mut stored) = world.get_mut::<CommittedNumericValue<N>>(field) {
        *stored = CommittedNumericValue::new(committed);
    }
    if let Some(mut buffer) = world.get_mut::<EditBuffer>(field) {
        buffer.set(committed.to_string());
    }
    if let Some(mut commits) = world.get_resource_mut::<Messages<NumericFieldCommitted<N>>>() {
        commits.write(NumericFieldCommitted {
            field,
            value: CommittedNumericValue::new(committed),
        });
    }
}

/// Reverts one numeric field's buffer to its last-good value WITHOUT committing (the Escape
/// path). Registered per concrete `N` by [`register_numeric_field`].
///
/// `&mut World` is justified as for [`commit_numeric_field`]: the type-erased observer queues
/// this concrete-`N` command.
fn revert_numeric_field<N: NumericValue>(world: &mut World, field: Entity) {
    let Some(last_good) = world.get::<CommittedNumericValue<N>>(field).copied() else {
        return;
    };
    if let Some(mut buffer) = world.get_mut::<EditBuffer>(field) {
        buffer.set(last_good.value().to_string());
    }
}

/// Type-erased entry points the keyboard observer queues; [`register_numeric_field`]
/// MONOMORPHIZES these to the concrete `N` so the erased observer can route Enter / Escape /
/// blur to the right numeric commit without naming `N`.
type NumericCommitFn = fn(&mut World, Entity);

/// A [`Resource`] holding the per-`N` numeric commit + revert function pointers a registered
/// numeric field needs, so the type-erased observer can dispatch to the concrete `N`.
///
/// One entry per registered `N`. [`commit_numeric_field_entity`] / [`revert_numeric_field_entity`]
/// look up the field's registered functions by trying each registered `N` (a field carries
/// exactly one `NumericRange<N>`, so exactly one entry resolves it).
#[derive(Resource, Default)]
struct NumericFieldRegistry {
    /// The registered `(commit, revert)` function pairs, one per `N`.
    handlers: Vec<(NumericCommitFn, NumericCommitFn)>,
}

/// Routes a queued Enter / blur numeric commit to every registered `N`'s commit fn; the fn for
/// the field's actual `N` does the work, the rest early-return (the field lacks their
/// `NumericRange<N>`).
fn commit_numeric_field_entity(world: &mut World, field: Entity) {
    let handlers = world
        .get_resource::<NumericFieldRegistry>()
        .map(|registry| registry.handlers.clone())
        .unwrap_or_default();
    for (commit, _revert) in handlers {
        commit(world, field);
    }
}

/// Routes a queued Escape revert to every registered `N`'s revert fn (only the field's actual
/// `N` mutates; the rest early-return).
fn revert_numeric_field_entity(world: &mut World, field: Entity) {
    let handlers = world
        .get_resource::<NumericFieldRegistry>()
        .map(|registry| registry.handlers.clone())
        .unwrap_or_default();
    for (_commit, revert) in handlers {
        revert(world, field);
    }
}

/// Mirrors each field's [`EditBuffer`] into its [`FieldText`] child's [`Text`] IN PLACE — the
/// mutate-not-respawn convention ([[ui-mutate-not-respawn]]).
///
/// `Changed<EditBuffer>` so it only redraws on a real edit. It walks the field's children to
/// the [`FieldText`] node and writes the buffer string, so the caret (a sibling laid out after
/// the text) always sits at the text END. Registered in [`Update`] by [`register_text_field`].
pub fn sync_edit_buffer_to_text(
    fields: Query<(&EditBuffer, &Children), Changed<EditBuffer>>,
    mut texts: Query<&mut Text, With<FieldText>>,
) {
    for (buffer, children) in &fields {
        for &child in children {
            if let Ok(mut text) = texts.get_mut(child)
                && text.0 != buffer.as_str()
            {
                buffer.as_str().clone_into(&mut text.0);
            }
        }
    }
}

/// Captures [`InputFocus`](bevy::input_focus::InputFocus) onto a field when its control is
/// PRESSED (the click-to-focus path, AC1).
///
/// `Changed<Interaction>` + the explicit `== Pressed` test means one focus capture per click
/// (the press edge). Sets [`InputFocus`](bevy::input_focus::InputFocus) to the pressed field
/// via [`FocusCause::Navigated`], so the keyboard observer then dispatches keypresses to it.
/// Registered in [`Update`] by [`register_text_field`].
pub fn focus_field_on_press(
    fields: Query<(Entity, &Interaction), (Changed<Interaction>, AnyField)>,
    mut focus: ResMut<InputFocus>,
) {
    for (field, interaction) in &fields {
        if *interaction == Interaction::Pressed {
            focus.set(field, FocusCause::Navigated);
        }
    }
}

/// Registers the field widgets' TYPE-AGNOSTIC pieces (mirrors the dropdown's split): the
/// [`TextFieldCommitted`] message, the keyboard / blur OBSERVERS, and the [`Update`] systems
/// ([`sync_edit_buffer_to_text`], [`focus_field_on_press`]).
///
/// Call this ONCE per app. The per-`N` numeric pieces (the
/// [`NumericFieldCommitted<N>`] message + the concrete commit/revert handlers) are registered
/// separately via [`register_numeric_field::<N>`] — a generic system / message cannot be added
/// for an unknown `N`, so each numeric `N` a caller spawns needs its own registration (an
/// unregistered handler is a dead feature — gate 4b). Without [`register_text_field`] the
/// observers never run.
pub fn register_text_field(app: &mut App) {
    app.add_message::<TextFieldCommitted>()
        .init_resource::<NumericFieldRegistry>()
        .add_observer(handle_text_field_key)
        .add_observer(commit_on_focus_lost)
        .add_systems(Update, (sync_edit_buffer_to_text, focus_field_on_press));
}

/// Registers ONE concrete numeric value type `N` for [`NumericField`] (mirrors
/// `register_dropdown::<T>`): the [`NumericFieldCommitted<N>`] message + the `N`-typed
/// commit / revert handlers the type-erased keyboard observer dispatches to.
///
/// Call once per `N` a caller spawns a [`spawn_numeric_field::<N>`] for. Requires
/// [`register_text_field`] to have run first (it initializes the private numeric-field handler
/// registry this pushes the `N`-typed commit / revert function pointers into).
pub fn register_numeric_field<N: NumericValue>(app: &mut App) {
    app.add_message::<NumericFieldCommitted<N>>();
    let mut registry = app
        .world_mut()
        .get_resource_or_insert_with(NumericFieldRegistry::default);
    registry
        .handlers
        .push((commit_numeric_field::<N>, revert_numeric_field::<N>));
}

/// A field root's [`Node`]: a padded row that lays its text child then its caret sliver in a
/// left-to-right flow. Relative units per the responsive-UI rule.
fn field_node() -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        min_width: Val::Vw(FIELD_MIN_WIDTH_VW),
        padding: UiRect::axes(Val::Vw(FIELD_PAD_X_VW), Val::Vh(FIELD_PAD_Y_VH)),
        ..default()
    }
}

/// The caret sliver's [`Node`]: a thin full-height bar drawn after the text child so it marks
/// the text END.
fn caret_node() -> Node {
    Node {
        width: Val::Px(CARET_WIDTH_PX),
        height: Val::Percent(CARET_HEIGHT_PCT),
        ..default()
    }
}

/// A field's minimum width, in viewport-width units, so an empty field still reads as an input
/// box rather than collapsing to the caret. Calibrated 120px / 1280 * 100.
const FIELD_MIN_WIDTH_VW: f32 = 9.375;

/// A field's horizontal inner padding, in viewport-width units. Calibrated 8px / 1280 * 100.
const FIELD_PAD_X_VW: f32 = 0.625;

/// A field's vertical inner padding, in viewport-height units. Calibrated 6px / 720 * 100.
const FIELD_PAD_Y_VH: f32 = 0.83333;

/// The caret sliver's width in logical pixels — a framework-plumbing `const` fed straight to a
/// [`Val::Px`] (the framework carve-out, not a domain value). A 2px bar reads as a text cursor.
const CARET_WIDTH_PX: f32 = 2.0;

/// The caret sliver's height as a percent of the field row — a framework-plumbing `const`. A
/// near-full-height bar reads as an inline caret without touching the padding edges.
const CARET_HEIGHT_PCT: f32 = 60.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_range_clamps_below_and_above() {
        let range = NumericRange::new(1_i64, 10_i64);
        assert_eq!(range.clamp(0), 1, "below-min clamps to min");
        assert_eq!(range.clamp(99), 10, "above-max clamps to max");
        assert_eq!(range.clamp(5), 5, "in-range passes through");
    }

    #[test]
    fn edit_buffer_append_and_pop_are_end_only() {
        let mut buffer = EditBuffer::new("ab");
        buffer.push_str("c");
        assert_eq!(buffer.as_str(), "abc", "append goes to the end");
        buffer.pop();
        assert_eq!(buffer.as_str(), "ab", "pop removes the last char");
        buffer.pop();
        buffer.pop();
        buffer.pop();
        assert_eq!(buffer.as_str(), "", "pop on empty is a no-op, never panics");
    }
}
