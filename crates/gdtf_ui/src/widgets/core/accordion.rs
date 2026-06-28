//! The [`Accordion`] widget: a vertical stack of expand/collapse rows whose
//! content height LERPS open and closed (GTW-416).
//!
//! An accordion is a column of ROWS, each a clickable [`AccordionHeader`] over an
//! [`AccordionContent`] section. Clicking a header TOGGLES that row: its content
//! height animates — over several frames, a visible LERP, never a snap — toward the
//! expanded or collapsed target, and the flex column repositions the sibling rows
//! below it automatically as the toggled row grows or shrinks (no manual reposition
//! math; the layout engine does it).
//!
//! ## Built ATOP the GTW-412 scroll list
//!
//! The row stack is spawned INSIDE a [`spawn_scroll_list`](super::spawn_scroll_list),
//! so when one (or several) rows are expanded and the combined content exceeds the
//! frame the whole accordion scrolls + clips — the gang-member list (GTW-425/428)
//! reveals a stat table per row this way. [`spawn_accordion`] returns the
//! [`Accordion`] ROOT (the row-stack column it parents onto the scroll area); the
//! caller adds rows via [`spawn_accordion_row`].
//!
//! ## The lerp — a relative height interpolated, snapped at the ends
//!
//! Each row content carries an [`AccordionAnim`] (its toggle TARGET) and an
//! [`AccordionProgress`] (a `0.0..=1.0` newtype). [`drive_accordions`] advances the
//! progress each frame by `Time::delta * `[`ACCORDION_LERP_PER_SEC`] toward the
//! target (1.0 expanded / 0.0 collapsed) and writes the content node's
//! [`height`](bevy::ui::Node::height) to the interpolated RELATIVE value between
//! [`ACCORDION_COLLAPSED_VH`] and the row's expanded target — a PER-INSTANCE
//! [`AccordionExpandedVh`] when present, else the shared default [`ACCORDION_EXPANDED_VH`].
//! A row that hosts tall content (the GTW-428 stat table) opts into a larger target so its
//! content fits the open height; every other row keeps the default. A float lerp does NOT
//! land on its endpoint exactly (bevy-ui-render-and-test-gotchas), so when the
//! progress comes within [`ACCORDION_SETTLE_EPSILON`] of an end it SNAPS to the exact
//! `0.0`/`1.0` and the animation is marked settled — it stops deterministically at the
//! collapsed / expanded height rather than creeping forever. No sleeps.
//!
//! ## Relative sizing (C3)
//!
//! Every dimension is RELATIVE (`Vh`/`Percent`/flex): the collapsed + expanded content
//! heights are `Vh` consts, the interpolated height is a `Vh` value, and the row /
//! header / stack fill their parents by `Percent`. The lone fixed-px values reachable
//! through an accordion are the scroll-list's two API-forced scrollbar-chrome px (see
//! [`spawn_scroll_list`](super::spawn_scroll_list)) and any font size — both the
//! documented carve-outs to the no-fixed-px rule.

use bevy::{
    prelude::*,
    ui::{
        AlignItems, BackgroundColor, Display, FlexDirection, Interaction, Node, Overflow, UiRect,
        Val, widget::Button,
    },
};

use super::{ScrollListColors, spawn_scroll_list};

/// The color set an [`Accordion`]'s rows paint themselves with.
///
/// Pure UI plumbing ([`Color`](bevy::prelude::Color)s): the scroll-list frame's
/// area / track / thumb fills (forwarded to [`spawn_scroll_list`](super::spawn_scroll_list)),
/// the clickable header fill, and the expandable content fill. Mirrors the sanctioned
/// `*Colors` builder pattern of [`ScrollListColors`](super::ScrollListColors) /
/// [`SwitchColors`](super::SwitchColors). The derived [`Default`] (all-transparent) is
/// a spawn-seed sentinel only — a builder always supplies real colors.
#[derive(Component, Clone, Copy, PartialEq, Debug, Default)]
pub struct AccordionColors {
    /// The scroll-area (clipping content viewport) background fill.
    pub area:    Color,
    /// The scrollbar track background fill.
    pub track:   Color,
    /// The draggable scrollbar thumb fill.
    pub thumb:   Color,
    /// The clickable row-header fill.
    pub header:  Color,
    /// The expandable row-content fill.
    pub content: Color,
}

impl AccordionColors {
    /// The scroll-list color subset (area / track / thumb) this accordion forwards to
    /// its underlying [`spawn_scroll_list`](super::spawn_scroll_list).
    #[must_use]
    pub const fn scroll_list_colors(&self) -> ScrollListColors {
        ScrollListColors {
            area:  self.area,
            track: self.track,
            thumb: self.thumb,
        }
    }
}

/// Marker on the ROW-STACK root of an [`Accordion`] — the flex COLUMN that holds the
/// rows and is parented onto the scroll-list area.
///
/// This is the [`Entity`] [`spawn_accordion`] returns; the caller adds rows to it via
/// [`spawn_accordion_row`]. A unit marker (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Accordion;

/// Marker on ONE accordion ROW root (a flex column of a header + a content section).
///
/// A unit marker — presence alone is the signal (no-bare-types rule). The caller may
/// attach its own identity marker alongside it.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct AccordionRow;

/// Marker on the clickable HEADER of an accordion row (a [`Button`], so `bevy_ui`'s
/// built-in `ui_focus_system` drives its [`Interaction`](bevy::ui::Interaction) from
/// the mouse — bevy-traps).
///
/// It carries the [`Entity`] of its row's [`AccordionContent`] in an
/// [`AccordionTarget`] so a header press flips the right content. A unit marker
/// (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct AccordionHeader;

/// Marker on the expandable CONTENT section of an accordion row — the node whose
/// [`height`](bevy::ui::Node::height) lerps.
///
/// It carries an [`AccordionAnim`] (its toggle state / target) and an
/// [`AccordionProgress`] (the `0.0..=1.0` lerp parameter) that [`drive_accordions`]
/// advances, and OPTIONALLY an [`AccordionExpandedVh`] naming a per-instance expanded
/// height (the shared [`ACCORDION_EXPANDED_VH`] default when absent). A unit marker
/// (no-bare-types rule); the caller parents its own revealed content (e.g. a stat table)
/// under it.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct AccordionContent;

/// The [`Entity`] of the [`AccordionContent`] a given [`AccordionHeader`] toggles.
///
/// A [`Component`] newtype over [`Entity`] (not a bare field) so a header press maps
/// to its OWN row's content without a positional lookup. The inner is private; read it
/// through [`content`](AccordionTarget::content).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct AccordionTarget(Entity);

impl AccordionTarget {
    /// Wraps the content [`Entity`] this header toggles.
    #[must_use]
    pub const fn new(content: Entity) -> Self {
        Self(content)
    }

    /// The [`AccordionContent`] entity this header toggles.
    #[must_use]
    pub const fn content(self) -> Entity {
        self.0
    }
}

/// The toggle STATE / direction of one accordion row's content lerp.
///
/// A named four-state vocabulary rather than a bare `bool` + flag: a row is either at
/// rest (`Collapsed` / `Expanded`) or animating toward an end (`Expanding` /
/// `Collapsing`). [`drive_accordions`] only advances the two animating states and
/// settles them to the matching rest state. UI plumbing, not a game-domain value.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AccordionAnim {
    /// Fully collapsed and at rest — content at [`ACCORDION_COLLAPSED_VH`].
    #[default]
    Collapsed,
    /// Animating OPEN — content height lerping up toward [`ACCORDION_EXPANDED_VH`].
    Expanding,
    /// Fully expanded and at rest — content at [`ACCORDION_EXPANDED_VH`].
    Expanded,
    /// Animating CLOSED — content height lerping down toward [`ACCORDION_COLLAPSED_VH`].
    Collapsing,
}

impl AccordionAnim {
    /// The state this row enters when its header is TOGGLED.
    ///
    /// A rest state begins animating the OTHER way; an in-flight animation REVERSES
    /// (so a mid-lerp click smoothly turns around rather than snapping).
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Collapsed | Self::Collapsing => Self::Expanding,
            Self::Expanded | Self::Expanding => Self::Collapsing,
        }
    }

    /// The lerp TARGET this state moves toward: `1.0` for the two expanding/expanded
    /// states, `0.0` for the two collapsing/collapsed states.
    #[must_use]
    const fn target(self) -> f32 {
        match self {
            Self::Expanding | Self::Expanded => 1.0,
            Self::Collapsing | Self::Collapsed => 0.0,
        }
    }

    /// Whether this state is still ANIMATING (so [`drive_accordions`] should advance
    /// it); rest states return `false`.
    #[must_use]
    const fn is_animating(self) -> bool {
        matches!(self, Self::Expanding | Self::Collapsing)
    }

    /// Whether this state is animating OPEN (toward expanded) — the direction a content-fit
    /// section settles to a [`Val::Auto`](bevy::ui::Val::Auto) height rather than a fixed `Vh`.
    #[must_use]
    const fn is_expanding(self) -> bool {
        matches!(self, Self::Expanding)
    }

    /// The REST state this animating state settles into once it reaches its target.
    #[must_use]
    const fn settled(self) -> Self {
        match self {
            Self::Expanding | Self::Expanded => Self::Expanded,
            Self::Collapsing | Self::Collapsed => Self::Collapsed,
        }
    }
}

/// The lerp parameter of one accordion row's content height — a clamped `0.0..=1.0`
/// where `0.0` is fully collapsed and `1.0` fully expanded.
///
/// A newtype over `f32` (no-bare-types rule): `0.0` maps to [`ACCORDION_COLLAPSED_VH`]
/// and `1.0` to [`ACCORDION_EXPANDED_VH`]; [`height_vh`](AccordionProgress::height_vh)
/// is the interpolated `Vh` magnitude. The inner is private; advance it through
/// [`advance_toward`](AccordionProgress::advance_toward) and read it through the
/// derived [`Deref`].
#[derive(Component, Deref, Clone, Copy, PartialEq, Debug, Default)]
pub struct AccordionProgress(f32);

impl AccordionProgress {
    /// A progress pinned to a known endpoint (`0.0` collapsed / `1.0` expanded),
    /// clamped into range.
    #[must_use]
    pub const fn new(value: f32) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    /// Advances the progress toward `target` (`0.0`/`1.0`) by `step`, SNAPPING to the
    /// exact target and reporting `settled = true` once within
    /// [`ACCORDION_SETTLE_EPSILON`] — so a float lerp that never lands exactly on its
    /// endpoint (bevy-ui-render-and-test-gotchas) still stops deterministically.
    ///
    /// Returns whether the animation has SETTLED this call (reached the target).
    #[must_use]
    pub fn advance_toward(&mut self, target: f32, step: f32) -> bool {
        let next = if self.0 < target {
            (self.0 + step).min(target)
        } else {
            (self.0 - step).max(target)
        };
        if (next - target).abs() <= ACCORDION_SETTLE_EPSILON {
            self.0 = target;
            true
        } else {
            self.0 = next;
            false
        }
    }

    /// The interpolated content height in viewport-height units (`Vh`): a linear blend
    /// between [`ACCORDION_COLLAPSED_VH`] (at `0.0`) and the DEFAULT expanded target
    /// [`ACCORDION_EXPANDED_VH`] (at `1.0`). A RELATIVE value (C3) — never a fixed pixel.
    ///
    /// The default-target convenience over [`height_vh_to`](AccordionProgress::height_vh_to):
    /// a row with no per-instance [`AccordionExpandedVh`] opens to the shared default.
    #[must_use]
    pub fn height_vh(self) -> f32 {
        self.height_vh_to(ACCORDION_EXPANDED_VH)
    }

    /// The interpolated content height in viewport-height units (`Vh`): a linear blend
    /// between [`ACCORDION_COLLAPSED_VH`] (at `0.0`) and a PER-INSTANCE `expanded` target
    /// (at `1.0`). A RELATIVE value (C3) — never a fixed pixel.
    ///
    /// [`drive_accordions`] reads each content's [`AccordionExpandedVh`] (defaulting to
    /// [`ACCORDION_EXPANDED_VH`] when absent) and lerps toward it through this, so a row
    /// that holds taller content (the GTW-428 stat table) opens to a height that fits it
    /// while every existing caller keeps the shared default (GTW-428 layout fix).
    #[must_use]
    pub fn height_vh_to(self, expanded: f32) -> f32 {
        self.0
            .mul_add(expanded - ACCORDION_COLLAPSED_VH, ACCORDION_COLLAPSED_VH)
    }
}

/// The fully-expanded content height TARGET of one accordion row, in viewport-height units
/// (`Vh`) — the per-instance ceiling the row's height lerps TO when expanded (GTW-428
/// layout fix).
///
/// A newtype over `f32` (no-bare-types rule): [`drive_accordions`] reads it off the
/// [`AccordionContent`] (defaulting to [`ACCORDION_EXPANDED_VH`] when absent) and feeds it
/// to [`height_vh_to`](AccordionProgress::height_vh_to). A row with no per-instance target
/// opens to the shared default ([`spawn_accordion_row`] seeds exactly the default, so every
/// existing caller is unchanged); a caller that hosts taller content (e.g. the GTW-428
/// 2-column per-member stat table) inserts a larger value so its content fits the open
/// height. The inner is private; build it via [`new`](AccordionExpandedVh::new) and read it
/// through the derived [`Deref`]. A RELATIVE value (C3) — never a fixed pixel.
#[derive(Component, Deref, Clone, Copy, PartialEq, Debug)]
pub struct AccordionExpandedVh(f32);

impl AccordionExpandedVh {
    /// Build a per-instance expanded-height target (`Vh`).
    #[must_use]
    pub const fn new(vh: f32) -> Self {
        Self(vh)
    }
}

impl Default for AccordionExpandedVh {
    /// The shared default target ([`ACCORDION_EXPANDED_VH`]) — so a row that does not opt
    /// into a per-instance height behaves exactly as before the GTW-428 generalization.
    fn default() -> Self {
        Self(ACCORDION_EXPANDED_VH)
    }
}

/// Opt-in marker on an [`AccordionContent`] whose REST-OPEN height should FIT ITS CONTENT
/// rather than clip to a fixed [`AccordionExpandedVh`] (the GTW-428 round-2 layout fix).
///
/// A fixed `Vh` expanded ceiling cannot reliably fit a content section whose natural height
/// depends on the rendered font / padding (the GTW-428 per-member stat table holds eight
/// editable attribute fields in its taller column, ~45vh at the editor's text size — a guessed
/// `Vh` clipped the bottom lines at the real window size). When this marker is present,
/// [`drive_accordions`] still LERPS the height up through `Vh` toward the per-instance target
/// (so the open is a visible animation, never a snap), but on SETTLING fully open it sets the
/// content height to [`Val::Auto`](bevy::ui::Val::Auto) so the rest-open section sizes to its
/// EXACT content — every line visible regardless of font metrics, no magnitude to guess. On
/// collapse the lerp returns through `Vh` to the collapsed height as usual. A taller rest-open
/// section simply makes the enclosing scroll list scroll.
///
/// A unit marker (no-bare-types rule); absence preserves the fixed-`Vh` behavior, so every
/// existing caller (the GTW-416 demo, the integration tests, [`spawn_accordion_row`] rows) is
/// unchanged.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct AccordionContentFit;

/// Spawns an [`Accordion`] inside a [`spawn_scroll_list`](super::spawn_scroll_list) and
/// returns the ROW-STACK [`Entity`] the caller adds rows to (via
/// [`spawn_accordion_row`]).
///
/// `colors` carry the scroll-list area/track/thumb fills (forwarded to the list) plus
/// the row header / content fills (carried on the returned stack so
/// [`spawn_accordion_row`] reads them); `marker` is any [`Bundle`] the caller wants on
/// the row-stack root — typically its own identity marker. The row stack is a flex
/// COLUMN parented onto the scroll-list area, so a tall set of expanded rows scrolls +
/// clips within the list frame.
pub fn spawn_accordion(
    commands: &mut Commands,
    colors: AccordionColors,
    marker: impl Bundle,
) -> Entity {
    let area = spawn_scroll_list(commands, colors.scroll_list_colors(), ());
    let stack = commands
        .spawn((Accordion, colors, accordion_stack_node()))
        .id();
    commands.entity(stack).insert(marker);
    commands.entity(area).add_child(stack);
    stack
}

/// Spawns one accordion ROW (a header [`Button`] over a collapsed content section) onto
/// the `accordion` row-stack and returns the [`AccordionContent`] [`Entity`] the caller
/// parents its revealed content (e.g. a stat table) under.
///
/// `header_marker` is any [`Bundle`] for the clickable header (typically a label /
/// identity); `content_marker` is any [`Bundle`] for the content section. The row
/// starts COLLAPSED ([`AccordionAnim::Collapsed`], [`AccordionProgress`]`(0.0)`,
/// content height at [`ACCORDION_COLLAPSED_VH`]); a header press flips it via
/// [`drive_accordions`]. The header carries an [`AccordionTarget`] pointing at the
/// content so the press toggles the right row.
pub fn spawn_accordion_row(
    commands: &mut Commands,
    accordion: Entity,
    colors: AccordionColors,
    header_marker: impl Bundle,
    content_marker: impl Bundle,
) -> Entity {
    let content = commands
        .spawn((
            AccordionContent,
            AccordionAnim::Collapsed,
            AccordionProgress(0.0),
            // Seed the shared-default expanded target so an `spawn_accordion_row` caller (the
            // GTW-416 demo, the integration tests) opens to the 18vh default exactly as before
            // the GTW-428 per-instance generalization.
            AccordionExpandedVh::default(),
            BackgroundColor(colors.content),
            accordion_content_node(),
        ))
        .id();
    commands.entity(content).insert(content_marker);
    let header = commands
        .spawn((
            AccordionHeader,
            AccordionTarget::new(content),
            Button,
            BackgroundColor(colors.header),
            accordion_header_node(),
        ))
        .id();
    commands.entity(header).insert(header_marker);
    let row = commands.spawn((AccordionRow, accordion_row_node())).id();
    commands.entity(row).add_children(&[header, content]);
    commands.entity(accordion).add_child(row);
    content
}

/// Read-write [`Query`] data for one clicked [`AccordionHeader`]: its
/// [`Interaction`](bevy::ui::Interaction) and the [`AccordionTarget`] naming the
/// content to flip.
///
/// Named to keep [`drive_accordions`]'s signature legible (clippy `type_complexity`).
///
/// It deliberately does NOT read the row's [`AccordionAnim`] — that lives on the
/// CONTENT entity, which the `contents` query borrows mutably; reading it here too
/// would make the two queries non-disjoint and panic B0001 (the content `AccordionAnim`
/// is read + flipped through `contents.get_mut` in phase 1).
type HeaderData = (&'static Interaction, &'static AccordionTarget);

/// Read-write [`Query`] data for one [`AccordionContent`] the lerp advances: its mutable
/// animation state + progress + [`Node`] (the height the lerp writes), its optional
/// per-instance [`AccordionExpandedVh`] target, and whether it opts into content-fit
/// ([`AccordionContentFit`]) so a settled-open section adopts a [`Val::Auto`] height.
///
/// Named to keep [`drive_accordions`]'s signature legible (clippy `type_complexity`).
type ContentData = (
    &'static mut AccordionAnim,
    &'static mut AccordionProgress,
    &'static mut Node,
    Option<&'static AccordionExpandedVh>,
    Has<AccordionContentFit>,
);

/// Drives every accordion: flips a header's content toward expanded/collapsed on a
/// press edge, then LERPS each animating content's height toward its target each frame.
///
/// Two phases, in one system so the flip and the per-frame advance never race
/// (bevy-traps rule 3):
///
/// 1. **Toggle** — for each header whose [`Interaction`](bevy::ui::Interaction)
///    `Changed` to [`Pressed`](bevy::ui::Interaction::Pressed) this frame, set its
///    [`AccordionTarget`]'s content [`AccordionAnim`] to
///    [`toggled`](AccordionAnim::toggled) (a rest state begins animating the other way;
///    an in-flight animation reverses). `Changed<Interaction>` + the `== Pressed` test
///    means one flip per click, not one per frame held.
/// 2. **Advance** — for every ANIMATING content, step its [`AccordionProgress`] toward
///    the [`AccordionAnim::target`] by `Time::delta * `[`ACCORDION_LERP_PER_SEC`],
///    write the interpolated [`height_vh`](AccordionProgress::height_vh) onto the
///    content [`Node`]'s [`height`](bevy::ui::Node::height) as a [`Val::Vh`], and once
///    the progress SNAPS to the target settle the [`AccordionAnim`] to its rest state
///    so it stops advancing (no creep, no sleep). A content carrying
///    [`AccordionContentFit`] that SETTLES fully open is given a
///    [`Val::Auto`](bevy::ui::Val::Auto) height instead of the fixed `Vh` target, so its
///    rest-open section sizes to its EXACT content (the GTW-428 round-2 fix — a guessed
///    `Vh` could not fit a font-dependent content height).
///
/// Param-only — no `&mut World` (bevy-traps rule 7). Registered by
/// [`UiPlugin`](crate::UiPlugin) in [`Update`].
pub fn drive_accordions(
    time: Res<Time>,
    headers: Query<HeaderData, (Changed<Interaction>, With<AccordionHeader>)>,
    mut contents: Query<ContentData, With<AccordionContent>>,
) {
    // Phase 1 — apply press toggles to the targeted content's animation state.
    for (interaction, target) in &headers {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if let Ok((mut anim, ..)) = contents.get_mut(target.content()) {
            *anim = anim.toggled();
        }
    }

    // Phase 2 — advance every animating content's height lerp toward its per-instance
    // expanded target (or the shared default when absent), so a taller-content row opens
    // to a height that fits it (GTW-428) while every default-target row is unchanged.
    let step = time.delta_secs() * ACCORDION_LERP_PER_SEC;
    for (mut anim, mut progress, mut node, expanded, content_fit) in &mut contents {
        if !anim.is_animating() {
            continue;
        }
        let expanded_vh = expanded.map_or(ACCORDION_EXPANDED_VH, |target| **target);
        let opening = anim.is_expanding();
        let settled = progress.advance_toward(anim.target(), step);
        // While animating, the height is the interpolated relative `Vh` (a visible lerp). On
        // SETTLING fully OPEN, a content-fit section switches to `Auto` so its rest-open height
        // is its exact content — every line shows regardless of font metrics (GTW-428 round-2).
        node.height = if settled && content_fit && opening {
            Val::Auto
        } else {
            Val::Vh(progress.height_vh_to(expanded_vh))
        };
        if settled {
            *anim = anim.settled();
        }
    }
}

/// The row-stack root [`Node`](bevy::ui::Node): a flex COLUMN that fills the scroll-area
/// width (`100%`) and is `auto`-height so its stacked rows determine its height (and
/// thus the list's overflow). All-relative.
fn accordion_stack_node() -> Node {
    Node {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        width: Val::Percent(100.0),
        ..default()
    }
}

/// One row root [`Node`](bevy::ui::Node): a full-width flex COLUMN (header above
/// content). All-relative.
fn accordion_row_node() -> Node {
    Node {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        width: Val::Percent(100.0),
        ..default()
    }
}

/// The clickable header [`Node`](bevy::ui::Node): full-width, a fixed RELATIVE height
/// ([`ACCORDION_HEADER_VH`]), vertically centring its label. All-relative.
fn accordion_header_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        height: Val::Vh(ACCORDION_HEADER_VH),
        align_items: AlignItems::Center,
        padding: UiRect::all(Val::Vh(ACCORDION_HEADER_PAD_VH)),
        ..default()
    }
}

/// The expandable content [`Node`](bevy::ui::Node): full-width, starting at the
/// COLLAPSED height ([`ACCORDION_COLLAPSED_VH`]) and CLIPPING its overflow so the
/// revealed content is hidden while the height is below its natural size mid-lerp. The
/// height is the animated relative value [`drive_accordions`] writes; all-relative.
fn accordion_content_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        height: Val::Vh(ACCORDION_COLLAPSED_VH),
        overflow: Overflow::clip(),
        ..default()
    }
}

/// The collapsed content height, in viewport-height units (`Vh`) — a RELATIVE unit
/// (C3). Zero so a collapsed row shows only its header; the lerp opens FROM here.
const ACCORDION_COLLAPSED_VH: f32 = 0.0;

/// The fully-expanded content height, in viewport-height units (`Vh`) — a RELATIVE unit
/// (C3). The lerp opens TO here; the content clips to this height (taller revealed
/// content scrolls within the underlying scroll list).
const ACCORDION_EXPANDED_VH: f32 = 18.0;

/// The clickable header height, in viewport-height units (`Vh`) — a RELATIVE unit (C3).
/// Constant across the toggle (only the CONTENT lerps), so the header always reads as a
/// grabbable row.
const ACCORDION_HEADER_VH: f32 = 5.0;

/// The header's inner padding, in viewport-height units (`Vh`) — a RELATIVE unit (C3).
const ACCORDION_HEADER_PAD_VH: f32 = 1.0;

/// The lerp SPEED of the content-height animation, in progress-units (`0.0..=1.0`) per
/// SECOND. At `4.0` a full open/close traverses the `0→1` range in `0.25 s` — fast
/// enough to feel responsive, slow enough to read as a clear animation over several
/// frames (the C1 "lerp, not snap" requirement). Multiplied by `Time::delta` so the
/// animation is frame-rate independent; a bare `f32` rate (framework plumbing fed to
/// the lerp, not a game-domain value).
const ACCORDION_LERP_PER_SEC: f32 = 4.0;

/// The settle tolerance on the lerp progress: once the advancing progress comes within
/// this of its `0.0`/`1.0` target it SNAPS to the exact endpoint and the animation is
/// marked settled. A float lerp never lands on its endpoint exactly
/// (bevy-ui-render-and-test-gotchas), so without this snap the height would creep
/// toward the target forever; this stops it deterministically at the collapsed /
/// expanded height. A bare `f32` epsilon (framework plumbing, not a domain value).
const ACCORDION_SETTLE_EPSILON: f32 = 1.0e-3;

#[cfg(test)]
mod test;
