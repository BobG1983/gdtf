//! The assembler's prefab PICKERS — the deterministic `(theme, role)` candidate list and the
//! player / fitting-fragment selectors both assemble steps scan (GTW-424; NO RNG draw — prefab
//! choice within a bucket is a deterministic largest-first scan).

use super::super::{
    anchor::Anchor,
    error::PackingError,
    geometry::{Footprint, MinPlayerSide, RegionRect},
    packer::MaxRectsPacker,
};
use crate::level::{Prefab, PrefabKey, PrefabRegistry, SpawnRole, ThemeUuid};

/// Every `(theme, role)` prefab the registry holds, in a DETERMINISTIC order (sorted by
/// footprint area DESC then by name, so the iteration order does not depend on the
/// registry's unordered `HashMap`) — the candidate list both pickers scan.
///
/// Prefabs are level FRAGMENTS smaller than the board: their registry `size` key is the
/// fragment footprint, NOT the board. So the assembler enumerates ACROSS sizes for a
/// `(theme, role)` and picks a FITTING one — it never assumes a fragment fills the board.
///
/// GTW-492: keyed on the stable [`ThemeUuid`] (the [`PrefabKey::theme`] field) and the
/// [`PrefabKey::role`] field. An absent `theme` (no key matches) yields an EMPTY list — the
/// pickers turn that into a fail-closed [`PackingError::NoPrefabForRole`].
fn candidates(registry: &PrefabRegistry, theme: ThemeUuid, role: SpawnRole) -> Vec<Prefab> {
    let mut out: Vec<Prefab> = registry
        .keys()
        .filter(|k| k.theme == theme && k.role == role)
        .flat_map(|k: &PrefabKey| registry.prefabs_for(k).iter().cloned())
        .collect();
    // Deterministic order: largest fragment first (prefer the densest deployment zone that
    // fits), ties broken by name so the order is total and seed-independent.
    out.sort_by(|a, b| {
        let area = |p: &Prefab| {
            let f = Footprint::of(p.spec().size);
            i64::from(f.width()) * i64::from(f.height())
        };
        area(b)
            .cmp(&area(a))
            .then_with(|| (**a.name()).cmp(&**b.name()))
    });
    out
}

/// Pick the PLAYER-spawn prefab: the first candidate (largest-first, deterministic) that
/// clears the OQ-5 minimum side AND fits — with its margin — flush at the player anchor.
///
/// OQ-5's cap is realised here: a fragment too large to leave room for the opposite enemy
/// region is simply not chosen (it fails the packer fit). Fails closed with
/// [`PackingError::NoPrefabForRole`] if no player prefab exists at all, or
/// [`PackingError::PlayerFootprintTooSmall`] if EVERY candidate is below the minimum side,
/// or [`PackingError::FootprintDoesNotFit`] if every (large-enough) candidate is too large
/// to fit the board with its margin.
pub(super) fn pick_player_prefab(
    registry: &PrefabRegistry,
    theme: ThemeUuid,
    packer: &MaxRectsPacker,
    board: RegionRect,
    anchor: Anchor,
    min_player_side: MinPlayerSide,
) -> Result<Prefab, PackingError> {
    let candidates = candidates(registry, theme, SpawnRole::Player);
    if candidates.is_empty() {
        return Err(PackingError::NoPrefabForRole {
            theme,
            role: SpawnRole::Player,
        });
    }

    // Track the best diagnostic error: a too-small one only matters if NO candidate
    // clears the minimum; a does-not-fit one if none of the big-enough ones fit.
    let mut last_too_small: Option<PackingError> = None;
    let mut last_no_fit: Option<PackingError> = None;
    for prefab in candidates {
        let footprint = Footprint::of(prefab.spec().size);
        if footprint.min_side() < *min_player_side.cells() {
            last_too_small = Some(PackingError::PlayerFootprintTooSmall {
                footprint,
                min_side: min_player_side,
            });
            continue;
        }
        let region = board.place_at_anchor(anchor, footprint);
        if *packer.fits(region) {
            return Ok(prefab);
        }
        last_no_fit = Some(PackingError::FootprintDoesNotFit {
            anchor,
            footprint,
            region: board,
        });
    }
    // Prefer the "does not fit" diagnostic (a big-enough candidate existed but did not
    // fit) over "too small" (no candidate even reached the minimum).
    Err(last_no_fit
        .or(last_too_small)
        .unwrap_or(PackingError::NoPrefabForRole {
            theme,
            role: SpawnRole::Player,
        }))
}

/// Pick a prefab of `role` that FITS — with its margin — flush at `anchor` against the
/// current free space (the enemy-spawn picker, C2). Largest fitting fragment first.
///
/// Fails closed with [`PackingError::NoPrefabForRole`] if none exists, or
/// [`PackingError::FootprintDoesNotFit`] if every candidate is too large for the remaining
/// space at the opposite anchor.
pub(super) fn pick_fitting_prefab(
    registry: &PrefabRegistry,
    theme: ThemeUuid,
    role: SpawnRole,
    packer: &MaxRectsPacker,
    board: RegionRect,
    anchor: Anchor,
) -> Result<Prefab, PackingError> {
    let candidates = candidates(registry, theme, role);
    if candidates.is_empty() {
        return Err(PackingError::NoPrefabForRole { theme, role });
    }
    let mut last_no_fit: Option<PackingError> = None;
    for prefab in candidates {
        let footprint = Footprint::of(prefab.spec().size);
        let region = board.place_at_anchor(anchor, footprint);
        if *packer.fits(region) {
            return Ok(prefab);
        }
        last_no_fit = Some(PackingError::FootprintDoesNotFit {
            anchor,
            footprint,
            region: board,
        });
    }
    Err(last_no_fit.unwrap_or(PackingError::NoPrefabForRole { theme, role }))
}
