//! The shared pooled-draw walk (GTW-568): the ONE take-first-N / lazily-grow /
//! hide-surplus loop every pooled overlay draw reuses.

use bevy::prelude::{DetectChangesMut, Mut, Visibility};

/// Walk a pooled-entity query against this frame's draw list — the shared pooled-overlay
/// draw loop (GTW-568), replacing the hand-rolled copies in the reachable / field /
/// path-preview / vertical-link draws and their single-entity (fire-target tile + cost
/// label, path target label) variants.
///
/// The pooled-overlay convention (why every overlay draws this way): pooled entities are
/// MUTATED in place, never respawned — and NEVER despawned. Each draw item reuses the
/// next pooled entity in `pooled`'s iteration order (`show` does the per-overlay writes:
/// transform / tint / atlas index / label text); when the pool is exhausted, `grow`
/// lazily spawns a new pooled entity for the draw — and MAY DECLINE (the vertical-link
/// draw skips the spawn while its terrain sheet is still loading; the pool simply stays
/// smaller that frame). Every surplus pooled entity is then hidden. A SINGLE-entity
/// affordance is the same walk over a 0/1-length draw list: [`Some`] shows/moves the one
/// pooled entity, [`None`] hides it via the surplus sweep.
///
/// # Visibility flips are owned HERE (`set_if_neq`)
///
/// Both the show-flip to [`Visible`](Visibility::Visible) and the surplus-hide to
/// [`Hidden`](Visibility::Hidden) go through
/// [`set_if_neq`](DetectChangesMut::set_if_neq), so an already-correct pooled entity is
/// NOT re-dirtied. The hand-rolled copies wrote `*visibility = Hidden` through the
/// [`Mut`] deref unconditionally, re-marking every already-hidden pooled sprite changed
/// EVERY frame and re-triggering visibility propagation each tick. `hide` PROJECTS the
/// caller's heterogeneous query item onto its visibility column; it returns the
/// [`Mut`]`<Visibility>` WRAPPER (not a bare `&mut Visibility`) because merely
/// deref-projecting through [`Mut`] would itself mark the component changed, defeating
/// the fix. Non-visibility writes stay in the caller's `show` closure (a shown entity's
/// transform / tint writes are the caller's business, unchanged by this helper).
///
/// # Draw-order determinism stays the caller's job
///
/// The walk consumes `draws` exactly as given; each caller keeps building its own
/// ordered draw list (e.g. the field overlay sorts its `HashMap`-sourced draws by
/// `(z, y, x)` before calling in).
///
/// # The highlight overlay is EXCLUDED on purpose
///
/// `overlays/highlight` stays bespoke: its draw is MESSAGE-driven with RETENTION
/// semantics — it acts on the last [`HighlightRequest`](crate::HighlightRequest) and
/// KEEPS the reticle where it is when no message arrives this frame. A hide-on-empty
/// pooled walk would wrongly hide the retained reticle on every messageless frame. Do
/// not migrate it here, and do not grow this helper mode flags for it — a caller that
/// needs a flag splits back to bespoke.
///
/// Closure-generic on purpose (NOT a `SystemParam`-/`Bundle`-generic type): the callers
/// pool four different component tuples, and thin `FnMut` callbacks keep the helper oblivious
/// to what a pooled item is beyond its visibility projection (framework plumbing, the
/// no-bare-types rule-4 carve-out).
pub fn draw_pool<'v, Item, Draw>(
    mut pooled: impl Iterator<Item = Item>,
    draws: impl IntoIterator<Item = Draw>,
    mut show: impl FnMut(Draw, &mut Item),
    mut grow: impl FnMut(Draw),
    mut hide: impl for<'a> FnMut(&'a mut Item) -> &'a mut Mut<'v, Visibility>,
) {
    for draw in draws {
        if let Some(mut item) = pooled.next() {
            // Reuse the next pooled entity in iteration order: the caller writes its
            // per-overlay state, the helper owns the show-flip (set_if_neq — no re-dirty
            // when the entity is already Visible).
            show(draw, &mut item);
            hide(&mut item).set_if_neq(Visibility::Visible);
        } else {
            // Pool exhausted: lazy growth via the caller's opaque spawn (which may
            // decline — the pooled set then simply stays smaller this frame).
            grow(draw);
        }
    }
    // Hide every surplus pooled entity the current draw list no longer needs — never
    // despawn (set_if_neq: an already-hidden sprite's change ticks stay untouched).
    for mut item in pooled {
        hide(&mut item).set_if_neq(Visibility::Hidden);
    }
}
