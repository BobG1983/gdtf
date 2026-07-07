//! The pure storey-treatment classifier (GTW-594 C1) and its vocabulary: the
//! [`IsolateView`] toggle, the composed [`StoreyViewMode`] input, and the
//! [`StoreyTreatment`] verdict.

use bevy::prelude::{Deref, Resource};
use gdtf_battle_sim::prelude::Level;

use super::super::active_level::{ActiveLevel, ViewMode};

/// How many storeys BELOW the active view storey a context storey sits (`>= 1` when
/// reported by the classifier), or — as [`IsolateView::On`]'s payload — how many onion
/// storeys below the active stay visible in the Isolate view.
///
/// A named view-domain newtype (no-bare-types) over the storey distance; PRIVATE inner,
/// derived [`Deref`] to the wrapped `u8`, minted only through [`ContextDepth::new`].
/// The classifier reports the TRUE distance; any brightness/tint RAMP a treatment table
/// derives from it CLAMPS at two tiers (GTW-594 C3 — depth `1` is tier one, depth `>= 2`
/// is tier two; tables never ramp deeper).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContextDepth(u8);

impl ContextDepth {
    /// Build a storey-distance depth (in storeys below the active view storey).
    #[must_use]
    pub const fn new(storeys: u8) -> Self {
        Self(storeys)
    }
}

/// The ISOLATE view toggle (GTW-594 C2/C3) — an ORTHOGONAL presenter-owned [`Resource`]
/// beside the two-state [`ViewMode`], never a third `ViewMode` variant (the test-pinned
/// two-state [`ViewMode::toggled`] contract is preserved).
///
/// While [`On`](Self::On), the drawn band's FLOOR becomes the active storey minus the
/// wrapped onion depth (band floor = active for depth `0`): the active storey plus at most
/// `depth` context storeys below draw, everything else — including every storey the
/// two-state mode would have drawn — is [`Hidden`](StoreyTreatment::Hidden).
///
/// **Precedence (GTW-594 C3, the ruled rule): Isolate WINS.** While `On`, the two-state
/// [`ViewMode`] (`DownToActive` × `FullView`) is preempted entirely — toggling it changes
/// nothing until Isolate turns [`Off`](Self::Off). Pinned by the classifier's unit tests.
///
/// The battlescape's [`TopDownRendererPlugin`](crate::TopDownRendererPlugin)
/// `init_resource`s the [`Default`] ([`Off`](Self::Off)) — NO battlescape visual change on
/// day one (C3); the editor seeds its own `Editing`-scoped default (`On`, one onion storey
/// below — C2).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IsolateView {
    /// Isolate off — the two-state [`ViewMode`] chooses the band (the pre-GTW-594
    /// behaviour, unchanged).
    #[default]
    Off,
    /// Isolate on — draw the active storey plus at most this many onion storeys below;
    /// hide everything else. Wins over the two-state [`ViewMode`] (C3 precedence).
    On(ContextDepth),
}

/// The composed view-mode input to [`storey_treatment`] (GTW-594 C1): the two-state
/// [`ViewMode`] plus the orthogonal [`IsolateView`] toggle, bundled so every consumer
/// hands the classifier the SAME pair and the Isolate-wins precedence (C3) is decided in
/// exactly one place — inside the classifier, never at a call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StoreyViewMode {
    /// The two-state band-ceiling mode (GTW-521) — consulted only while `isolate` is
    /// [`IsolateView::Off`].
    view:    ViewMode,
    /// The orthogonal Isolate toggle (GTW-594) — wins over `view` while on.
    isolate: IsolateView,
}

impl StoreyViewMode {
    /// Compose the two presenter view resources into the classifier's one mode input.
    #[must_use]
    pub const fn new(view: ViewMode, isolate: IsolateView) -> Self {
        Self { view, isolate }
    }
}

/// The treatment a storey classifies to for one frame (GTW-594 C1) — the named verdict
/// each presenter maps to pixels through its OWN treatment table.
///
/// The testable law (unit-tested here, the A1 invariant): under EVERY mode exactly one
/// storey classifies [`Active`](Self::Active) per frame, and only [`Active`](Self::Active)
/// may render full-bright art.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StoreyTreatment {
    /// Not drawn at all this frame.
    Hidden,
    /// THE active view storey — the one storey allowed to render full-bright art.
    Active,
    /// A drawn NON-active context storey, at the wrapped storey distance from the active.
    ///
    /// Canonically a storey BELOW the active (the UFO:EU multi-level context). The one
    /// deliberate exception: in the legacy [`ViewMode::FullView`] whole-stack view
    /// (Isolate off) storeys ABOVE the active also draw, and they classify HERE at their
    /// storey distance — the flat context treatment they already received under GTW-521,
    /// preserved bit-for-bit. A DISTINCT above-active treatment is explicitly out of
    /// scope (GTW-594 C4; the canon above-active hard cut holds in every other mode —
    /// GTW-596 owns any change).
    ContextBelow(ContextDepth),
}

/// The ONE pure shared storey classification (GTW-594 C1): what treatment `storey` gets
/// under `mode`, given the `active` view storey.
///
/// Promoted from the three pre-GTW-594 "which storeys draw" choke points — the terrain
/// draw band (`drawn_band`), the ganger band fact ([`ActiveLevel::draws_storey`]), and the
/// editor preview's `drawn_storeys` — which now all derive from this classifier, so the
/// surfaces can never drift. Pure (no Bevy borrows) and total over every `(storey,
/// active, mode)`:
///
/// - `storey == active` → [`StoreyTreatment::Active`] — ALWAYS, in every mode (the
///   exactly-one-Active law).
/// - [`IsolateView::On`] (WINS over the two-state mode — the C3 precedence): a storey
///   within the onion band strictly below the active (`active - depth ..= active - 1`) is
///   [`StoreyTreatment::ContextBelow`] at its true distance; every other storey is
///   [`StoreyTreatment::Hidden`].
/// - [`IsolateView::Off`] + [`ViewMode::DownToActive`] (the default): below the active is
///   [`StoreyTreatment::ContextBelow`] at its distance, above is
///   [`StoreyTreatment::Hidden`] (the GTW-519/520 band, unchanged).
/// - [`IsolateView::Off`] + [`ViewMode::FullView`]: every non-active storey is
///   [`StoreyTreatment::ContextBelow`] at its distance — above-active included (the
///   GTW-521 whole-stack view, see [`StoreyTreatment::ContextBelow`]).
///
/// Callers iterate only the storeys their own volume holds (the presenter walks
/// `0..MAX_LEVELS`, the editor its prefab's `0..levels`), so the classifier needs no
/// ceiling input.
#[must_use]
pub fn storey_treatment(
    storey: Level,
    active: ActiveLevel,
    mode: StoreyViewMode,
) -> StoreyTreatment {
    // Level Derefs to its u8 storey index; ActiveLevel Derefs to Level.
    let storey_ix = *storey;
    let active_ix = **active;
    if storey_ix == active_ix {
        return StoreyTreatment::Active;
    }
    match mode.isolate {
        // Isolate WINS over the two-state mode (C3): only the onion band below survives.
        IsolateView::On(onion) => match active_ix.checked_sub(storey_ix) {
            Some(depth) if depth <= *onion => StoreyTreatment::ContextBelow(ContextDepth(depth)),
            _ => StoreyTreatment::Hidden,
        },
        IsolateView::Off => match mode.view {
            // The default band (GTW-519/520): everything below draws as context, above
            // is the canon hard cut.
            ViewMode::DownToActive => match active_ix.checked_sub(storey_ix) {
                Some(depth) => StoreyTreatment::ContextBelow(ContextDepth(depth)),
                None => StoreyTreatment::Hidden,
            },
            // The whole-stack view (GTW-521): every non-active storey is context at its
            // distance, above-active included (the documented legacy exception).
            ViewMode::FullView => {
                StoreyTreatment::ContextBelow(ContextDepth(storey_ix.abs_diff(active_ix)))
            }
        },
    }
}
