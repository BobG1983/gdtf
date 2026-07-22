//! The tick-quiet write helper (GTW-627 C4): the ONE place owning the fog/resolver write
//! discipline, so it cannot be re-derived per call site.
//!
//! Two writers share it — the terrain arm of [`present_fog`](super::fog::present_fog) and
//! the ganger-visibility resolver
//! ([`resolve_ganger_visibility`](super::ganger::resolve_ganger_visibility)) — and both
//! run EVERY frame over their full entity sets, so an unconditional write would re-mark
//! everything changed each tick (re-triggering visibility propagation, and re-uploading
//! each material uniform — the exact re-dirty class the GTW-568 pooled-overlay helper
//! documents in `overlays/pool/draw.rs`). The discipline:
//!
//! * **Visibility flips go through [`set_if_neq`](DetectChangesMut::set_if_neq)** — an
//!   already-correct entity's change ticks stay untouched. The projection takes the
//!   [`Mut`] WRAPPER (never a bare `&mut Visibility`): merely deref-projecting through
//!   [`Mut`] would itself mark the component changed, defeating the discipline (the GTW-568
//!   lesson).
//! * **Material knobs compare-before-mutate** — read through the immutable
//!   [`Assets::get`] path (no resource-tick poke through [`ResMut`]'s `Deref`), and take
//!   [`Assets::get_mut`] — which marks the store changed and queues the
//!   `AssetEvent::Modified` that re-uploads the uniform next frame — ONLY when a knob
//!   actually differs.

use bevy::{
    asset::{AssetId, Assets},
    prelude::{DetectChangesMut, Mut, ResMut, Visibility},
};

use super::fog::{Brightness, Saturation, TerrainFogMaterial};

/// Flip `visibility` to `target` tick-quietly: [`set_if_neq`](DetectChangesMut::set_if_neq)
/// through the [`Mut`] wrapper, so an entity already at `target` is NOT re-marked changed.
pub(super) fn set_visibility_quiet(visibility: &mut Mut<Visibility>, target: Visibility) {
    visibility.set_if_neq(target);
}

/// Drive one terrain tile's fog material knobs (`saturation` + `brightness`)
/// tick-quietly: compare through [`Assets::get`] first, and take the change-marking
/// [`Assets::get_mut`] only when a knob actually differs.
///
/// Takes the [`ResMut`] WRAPPER (not `&mut Assets<_>`) for the same reason the visibility
/// projection takes [`Mut`]: dereferencing the wrapper mutably at the call site would mark
/// the store's resource tick BEFORE any compare. The knob compare is bitwise
/// (`f32::to_bits`) — the fog writes only the named constant magnitudes, so bit equality
/// is exact and dodges the float-compare pitfalls. A tile whose material is not in the
/// store (still loading) is skipped, exactly as the earlier writer skipped a failed
/// `get_mut`.
pub(super) fn set_fog_knobs_quiet(
    materials: &mut ResMut<Assets<TerrainFogMaterial>>,
    id: AssetId<TerrainFogMaterial>,
    saturation: Saturation,
    brightness: Brightness,
) {
    // Immutable read (Assets::get via ResMut's Deref): no resource-tick poke, no asset
    // event — an already-correct tile costs a compare and nothing else.
    let differs = materials.get(id).is_some_and(|material| {
        material.saturation.to_bits() != saturation.to_bits()
            || material.brightness.to_bits() != brightness.to_bits()
    });
    if !differs {
        return;
    }
    // A knob really moved: NOW take the tracked mutable path (marks the store changed +
    // queues the AssetEvent::Modified that re-uploads this tile's uniform next frame).
    if let Some(mut material) = materials.get_mut(id) {
        material.saturation = *saturation;
        material.brightness = brightness;
    }
}
