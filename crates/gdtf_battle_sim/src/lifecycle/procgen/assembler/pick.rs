use super::super::{
    anchor::Anchor,
    error::PackingError,
    geometry::{Footprint, MinPlayerSide, RegionRect},
    packer::MaxRectsPacker,
};
use crate::level::{Prefab, PrefabKey, PrefabRegistry, SpawnRole, ThemeUuid};

fn candidates(registry: &PrefabRegistry, theme: ThemeUuid, role: SpawnRole) -> Vec<Prefab> {
    let mut out: Vec<Prefab> = registry
        .keys()
        .filter(|k| k.theme == theme && k.role == role)
        .flat_map(|k: &PrefabKey| registry.prefabs_for(k).iter().cloned())
        .collect();
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
    Err(last_no_fit
        .or(last_too_small)
        .unwrap_or(PackingError::NoPrefabForRole {
            theme,
            role: SpawnRole::Player,
        }))
}

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
