//! Repaints the status-panel **stability readout** bar from the selected shooter (GTW-345).
//!
//! [`update_stability_readout`] reads [`Res<SelectedShooter>`](gdtf_battle_input::SelectedShooter),
//! resolves it to the selected [`Entity`], assembles the sim [`Shooter`] borrow-view from the
//! ganger's [`Stance`] / [`Aiming`] / [`Position`] / [`Facing`] (plus its GTW-526 optional
//! [`Suppressed`] state, so the preview widens under suppression), resolves the wielded RANGED
//! weapon's [`Stable`] tag (`ganger → Wields → the ranged weapon entity → Stable`, the SAME
//! ranged-filtered resolution the sim fire path uses — GTW-505 C5), reads the model
//! [`CoverLedger`] + [`CombatTuning`] +
//! [`BraceStairCells`] + [`SurfaceGrid`], calls [`terrain_braces`] then [`stability_for`] —
//! re-deriving NONE of the §1a math. The returned [`ConeMult`] (the steadiness read) becomes a
//! [`Steadiness`] and then a [`FillFraction`](gdtf_ui::FillFraction) at the bar boundary,
//! fuller = steadier.
//!
//! Fail-closed (AC4): no selection, a selected entity lacking the shooter or weapon
//! components, or any of [`CoverLedger`] / [`CombatTuning`] / [`BraceStairCells`] /
//! [`SurfaceGrid`] absent (a harness without them) → the bar shows the EMPTY state (zeroed),
//! never stale data, never a panic.
//!
//! GTW-392: [`terrain_braces`] is called with the same [`BraceStairCells`] +
//! [`SurfaceGrid`] the fire path reads — no stub-false shortcut. The brace slab bonus is
//! therefore visible in the HUD bar the moment it applies, matching the fire path exactly.
//!
//! It runs in `Update` gated `run_if(resource_exists::<BattleInProgress>)`,
//! `.after(InputSystems::Gather)` — mirroring
//! [`update_status_panel`](super::super::update::update_status_panel) so it observes the same
//! update's auto-select write to `SelectedShooter`.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    Aiming, BraceStairCells, CoverLedger, Facing, MeleeWeapon, Position, Shooter, Stable, Stance,
    Suppressed, Weapon, Wields,
    stability::{StabilityTerms, terrain_brace::terrain_braces},
    stability_for,
    surface::SurfaceGrid,
    tuning::CombatTuning,
    weapon::WeaponBraceBonus,
};
use gdtf_ui::{FillFraction, ProgressBarFill, set_progress_bar};

use crate::states::running::game::battlescape::status_panel::stability_readout::components::{
    StabilityBar, Steadiness,
};

/// The selected shooter's stability-relevant ganger components, read in ONE query tuple
/// (the `StatBlockData` wide-read precedent — keeps the read off clippy `type_complexity`).
///
/// These are exactly the borrows the sim [`Shooter`] view is assembled from — the four
/// core ganger components plus the GTW-526 optional [`Suppressed`] state, so the HUD
/// preview widens the readout under suppression exactly as the fire path does.
type ShooterView<'a> = (
    &'a Stance,
    &'a Aiming,
    &'a Position,
    &'a Facing,
    Option<&'a Suppressed>,
);

/// The wielded RANGED weapon's stability-relevant components, read in ONE query tuple (the
/// `ShooterView` precedent — keeps the `weapons` query off clippy `type_complexity`): the
/// `stable` tag plus the GTW-549 optional per-item [`WeaponBraceBonus`] attachment, so the HUD
/// preview folds the graduated brace bonus exactly as the fire path does (the `Option` =
/// absent when no brace attachment is fitted). SUPERSEDES the GTW-542 sight-stability read — a
/// sight now boosts AIM (Accuracy), not stability.
type WeaponStabilityRead<'a> = (&'a Stable, Option<&'a WeaponBraceBonus>);

/// The read-only queries the stability resolution touches, bundled as one [`SystemParam`]
/// (the `StatBlockWidgets` / `RouteGrids` bundle precedent) so the system stays under clippy's
/// argument-count gate.
///
/// The shooter's [`Shooter`]-view components, its [`Wields`] relationship, the wielded
/// RANGED weapon's [`Stable`] tag, and the [`MeleeWeapon`] marker probe — every query disjoint
/// (distinct component types), so they coexist with no `B0001` conflict.
#[derive(SystemParam)]
pub(in crate::states::running::game::battlescape::status_panel) struct ShooterReadQueries<'w, 's> {
    /// The selected ganger's [`Shooter`]-view components (`Stance`/`Aiming`/`Position`/`Facing`).
    shooters: Query<'w, 's, ShooterView<'static>>,
    /// The ganger → weapon relationship (`ganger → Wields → the ranged weapon entity`).
    wields:   Query<'w, 's, &'static Wields>,
    /// The wielded RANGED weapon's `stable` tag plus its optional GTW-549 per-item
    /// [`WeaponBraceBonus`] attachment, read off the weapon entity — so the HUD preview folds
    /// the graduated brace bonus into the readout exactly as the fire path does (the `Option` =
    /// absent when no brace attachment is fitted).
    weapons:  Query<'w, 's, WeaponStabilityRead<'static>, With<Weapon>>,
    /// The [`MeleeWeapon`] marker probe (GTW-505 C5) — so the RANGED weapon resolves excluding
    /// the melee weapon the ganger also wields, rather than relying on relate order.
    melee:    Query<'w, 's, (), With<MeleeWeapon>>,
}

/// The bar-mutate queries the readout writes through, bundled as one [`SystemParam`] so the
/// system stays under clippy's argument-count gate.
///
/// The single [`StabilityBar`] track, its `Children` (to find the fill child), and the
/// [`ProgressBarFill`] width writer the `gdtf_ui` [`set_progress_bar`] helper mutates.
#[derive(SystemParam)]
pub(in crate::states::running::game::battlescape::status_panel) struct StabilityBarWriter<'w, 's> {
    /// The status panel's single stability bar track.
    bars:     Query<'w, 's, Entity, With<StabilityBar>>,
    /// The bar track's children (its `ProgressBarFill` child).
    children: Query<'w, 's, &'static Children>,
    /// The `ProgressBarFill` width writer the `set_progress_bar` helper mutates.
    fills:    Query<'w, 's, &'static mut Node, With<ProgressBarFill>>,
}

/// Repaints the stability readout bar from the current [`SelectedShooter`].
///
/// Assembles the [`Shooter`] borrow-view, resolves the wielded [`Stable`], reads the model
/// [`CoverLedger`] + [`CombatTuning`] + [`BraceStairCells`] + [`SurfaceGrid`], calls
/// [`terrain_braces`] then [`stability_for`] for the `(ConeMult, _)` pair, and mutates the
/// [`StabilityBar`] fill to the [`Steadiness`] derived from the [`ConeMult`].
/// Any missing piece (AC4) drives the EMPTY state. Param-only (`bevy-traps.md` #7): no
/// `&mut World`.
pub(in crate::states::running::game::battlescape::status_panel) fn update_stability_readout(
    selected: Res<SelectedShooter>,
    cover: Option<Res<CoverLedger>>,
    tuning: Option<Res<CombatTuning>>,
    brace_cells: Option<Res<BraceStairCells>>,
    surface: Option<Res<SurfaceGrid>>,
    reads: ShooterReadQueries,
    mut writer: StabilityBarWriter,
) {
    // The status panel's single stability bar (exactly one in a live battle).
    let Ok(bar) = writer.bars.single() else {
        return;
    };

    // Fail-closed: resolve every input, defaulting to the EMPTY readout the moment any one
    // is missing (no selection / not a shooter / unarmed / no cover ledger / no tuning /
    // no brace cells / no surface grid — GTW-392 adds the last two).
    let fraction = resolve_steadiness(
        *selected,
        cover.as_deref(),
        tuning.as_deref(),
        brace_cells.as_deref(),
        surface.as_deref(),
        &reads,
    )
    .map_or(FillFraction::new(0.0), Steadiness::fill_fraction);

    set_progress_bar(bar, fraction, &writer.children, &mut writer.fills);
}

/// Resolves the selected shooter's [`Steadiness`], or `None` when any input is missing.
///
/// The single fail-closed read path (AC4): returns the [`Steadiness`] derived from
/// [`stability_for`]'s [`ConeMult`] only when there is a selection, it is a shooter
/// (`Stance`/`Aiming`/`Position`/`Facing`), it wields a weapon carrying a [`Stable`] tag, and
/// the model resources ([`CoverLedger`], [`CombatTuning`], [`BraceStairCells`],
/// [`SurfaceGrid`]) are all present; any miss returns `None` so the caller paints the empty
/// bar. Re-derives NONE of the §1a math — it only assembles the view + resolves `Stable` +
/// reads the resources + calls [`terrain_braces`] + [`stability_for`].
///
/// GTW-392: calls [`terrain_braces`] with the same [`BraceStairCells`] + [`SurfaceGrid`] the
/// fire path reads — no stub-false shortcut. Keeping the two call sites in sync means the
/// HUD reflects exactly what the fire path will compute.
fn resolve_steadiness(
    selected: SelectedShooter,
    cover: Option<&CoverLedger>,
    tuning: Option<&CombatTuning>,
    brace_cells: Option<&BraceStairCells>,
    surface: Option<&SurfaceGrid>,
    reads: &ShooterReadQueries,
) -> Option<Steadiness> {
    let entity = (*selected)?;
    let (stance, aiming, position, facing, suppressed) = reads.shooters.get(entity).ok()?;
    // Resolve `ganger → Wields → the RANGED weapon entity → Stable` (GTW-323, the fire path's
    // resolution). GTW-505 C5: a ganger wields BOTH a ranged AND a melee weapon, so resolve
    // through `Wields::ranged_weapon` (excluding the `MeleeWeapon`-marked entity) — NOT
    // `Wields::weapon` (the spawn-order fragile first entity) — so the steadiness reads the
    // GUN's `Stable` tag. An unarmed ganger / a ranged weapon with no `Stable` tag fails closed.
    let weapon = reads
        .wields
        .get(entity)
        .ok()?
        .ranged_weapon(|e| reads.melee.get(e).is_ok())?;
    let (stable, brace_bonus) = reads.weapons.get(weapon).ok()?;
    let stable = *stable;
    // GTW-549: resolve the additive per-item brace term off the weapon's optional
    // WeaponBraceBonus attachment (the SAME term the fire path reads) so a braced weapon
    // previews a tighter steadiness; a weapon with no brace resolves the zero identity
    // (byte-identical readout).
    let brace_bonus = brace_bonus.copied().unwrap_or_else(WeaponBraceBonus::none);
    let cover = cover?;
    let tuning = tuning?;
    let brace_cells = brace_cells?;
    let surface = surface?;

    // GTW-392: compute the terrain-brace result from live grids — same call as the fire path.
    // `*stance` dereferences Stance → StanceKind (one Deref step).
    let terrain_braced = terrain_braces(*position, **stance, brace_cells, surface);

    // Assemble the transient `Shooter` borrow-view (the `ShooterSnapshot::shooter_view`
    // shape) and call the authoritative composer; take the `ConeMult` (the steadiness read).
    let shooter = Shooter {
        stance,
        aiming,
        position,
        facing,
        // GTW-526: thread the shooter's Suppressed state so the HUD preview widens the
        // readout under suppression, matching the fire path exactly.
        suppressed,
    };
    // GTW-543: the readout previews the shooter's OWN carried weapon (it resolves the ranged
    // weapon, not the emplacement mount), so the emplacement term stays at its zero-identity
    // DEFAULT (GTW-573 C7 — the StabilityTerms struct-update spells only the engaged terms) —
    // the preview is byte-identical to before the seam. The AUTHORITATIVE mounted-shot
    // steadiness is applied by the sim `fire()` path; a mounted-weapon HUD preview is out of
    // this slice's scope.
    let (cone_mult, _recoil_growth) = stability_for(
        &shooter,
        StabilityTerms {
            stable,
            terrain_braced,
            brace_bonus,
            ..StabilityTerms::default()
        },
        cover,
        tuning,
    );
    Some(Steadiness::from_cone_mult(cone_mult))
}
