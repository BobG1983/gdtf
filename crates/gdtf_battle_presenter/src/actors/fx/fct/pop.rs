//! The CONSEQUENCE-POP vocabulary (GTW-572): the named pop struct every consequence
//! family classifies into, its anchor enum, and the [`ConsequenceFct`] palette trait the
//! generic stacked-pop reader ([`read_consequence_fct`](super::stacked_reader::read_consequence_fct))
//! drives.
//!
//! Before GTW-572 each consequence family (bleeding / armor-broken / injury / suppression /
//! DOT / field / on-death) hand-rolled its own reader system — a local per-frame stack
//! counter, a cell/level reconstruction, and a `spawn_floating_text` tail — differing ONLY
//! in the classify half. This module names that classify half: a family implements
//! [`ConsequenceFct`] (one file per family under [`families`](super::families)), and the ONE
//! generic reader does the rest. The OUT-of-scope readers stay out by design (P9): the shot
//! pipeline's [`classify_report`](super::reader::classify_report) is already the shared
//! multi-pop classifier riding the staggered projectile → impact pipeline, and the fall FX
//! reader (`fx/fall.rs`) is a glyph/shake FX, not a stacked pop.

use bevy::prelude::{Color, Message};
use gdtf_battle_sim::prelude::CellLevel;

use super::text::{CombatText, FctEmphasis};

/// Where a consequence pop ANCHORS — the `(cell, level)` it rises over.
///
/// A named domain enum (no bare tuple / flag): a family whose sim message already carries
/// the cell resolves [`Carried`](Self::Carried) with no `World` read; a family whose message
/// names only the ganger resolves [`GangerPosition`](Self::GangerPosition), which the
/// generic reader looks up via `Query<&Position>` — FAIL-CLOSED: an unresolvable ganger
/// DROPS the pop (no pop at a default position, never a panic).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopAnchor {
    /// The message carried its own `(cell, level)` — anchor there directly.
    Carried(CellLevel),
    /// Anchor at this ganger's live [`Position`](gdtf_battle_sim::ganger::Position); if the entity
    /// has none (despawned / minimal harness) the pop is dropped fail-closed.
    GangerPosition(bevy::prelude::Entity),
}

/// One ready-to-spawn consequence pop — the classified string, its RESOLVED swatch, its
/// emphasis weight, and its anchor (GTW-572 C1).
///
/// The ONE named pop struct every consequence family classifies into (P10 — it replaces the
/// per-family `AuxPop` / `InjuryPop` / `SuppressionPop` / `DotTickPop` / `FieldTickPop` /
/// `OnDeathPop` clones). The color is RESOLVED by the family's classify — an injury rides
/// the [`severity_color`](super::palette::severity_color) ramp, every other family maps
/// through [`valence_color`](super::palette::valence_color) — so the reader never re-derives
/// a swatch. The [`Color`](bevy::prelude::Color) is framework plumbing (the swatch fed
/// straight to the primitive), the only bare type the no-bare-types rule permits here.
/// Fields are private (rule 5): a pop is only built through [`ConsequencePop::new`] /
/// [`ConsequencePop::new_bold`], and read through the accessors.
#[derive(Debug, Clone)]
pub struct ConsequencePop {
    /// The combat-text string this pop renders (`"Bleeding"`, `"-4"`, `"SUPPRESSED"`, …).
    text:     CombatText,
    /// The RESOLVED valence swatch the pop is drawn in (severity ramp or valence family).
    color:    Color,
    /// The styling weight — [`FctEmphasis::Bold`] only for the terminal on-death marker.
    emphasis: FctEmphasis,
    /// Where the pop anchors — a carried cell or a ganger's live position (fail-closed).
    anchor:   PopAnchor,
}

impl ConsequencePop {
    /// Build an ordinary (body-weight) consequence pop.
    #[must_use]
    pub const fn new(text: CombatText, color: Color, anchor: PopAnchor) -> Self {
        Self {
            text,
            color,
            emphasis: FctEmphasis::Normal,
            anchor,
        }
    }

    /// Build an EMPHASIZED (bold) consequence pop — the terminal on-death marker family
    /// (the heaviest, most-urgent pop; body weight everywhere else).
    #[must_use]
    pub const fn new_bold(text: CombatText, color: Color, anchor: PopAnchor) -> Self {
        Self {
            text,
            color,
            emphasis: FctEmphasis::Bold,
            anchor,
        }
    }

    /// The pop's combat-text string.
    #[must_use]
    pub const fn text(&self) -> &CombatText {
        &self.text
    }

    /// The pop's resolved swatch — fed straight to
    /// [`spawn_floating_text`](super::text::spawn_floating_text).
    #[must_use]
    pub const fn color(&self) -> Color {
        self.color
    }

    /// The pop's styling weight.
    #[must_use]
    pub const fn emphasis(&self) -> FctEmphasis {
        self.emphasis
    }

    /// The pop's anchor — resolved to a `(cell, level)` by the generic reader.
    #[must_use]
    pub const fn anchor(&self) -> PopAnchor {
        self.anchor
    }
}

/// A CONSEQUENCE FAMILY in the FCT pop palette (GTW-572 C1) — one sim signal, one classify.
///
/// Implemented by a zero-sized family marker in its own file under
/// `families` (one file per family: the marker, the classify impl, and
/// its unit tests — P10/P11). The generic
/// [`read_consequence_fct`](super::stacked_reader::read_consequence_fct) drains
/// [`Self::Signal`], calls [`Self::classify`], resolves the anchor (fail-closed on a
/// [`PopAnchor::GangerPosition`] miss), takes the next slot from the SHARED per-frame
/// [`FctStackCounter`](super::stack::FctStackCounter), and spawns the pop. Registration is
/// one [`add_consequence_fct`](super::stacked_reader::ConsequenceFctAppExt::add_consequence_fct)
/// line (P4 — compile-time generics, no runtime descriptor table).
pub trait ConsequenceFct: Send + Sync + 'static {
    /// The sim message this family drains — the presenter reads it one-way (ADR-0001).
    type Signal: Message;

    /// Classify one drained signal into its pop — the pure, `World`-free mapping every
    /// family unit-tests in its own file. Exactly one pop per signal: every current family
    /// pops on every message (a signal that should NOT pop belongs to no family).
    fn classify(signal: &Self::Signal) -> ConsequencePop;
}
