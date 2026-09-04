use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    aim::{Shooter, stability_for},
    cover::CoverLedger,
    effects::attachments::WeaponBraceBonus,
    ganger::{Aiming, Facing, Suppressed},
    prelude::{Position, Stance},
    slab::BraceStairCells,
    stability::{StabilityTerms, terrain_brace::terrain_braces},
    surface::SurfaceGrid,
    tuning::CombatTuning,
    weapon::{MeleeWeapon, Stable, Weapon, Wields},
};
use gdtf_ui::{FillFraction, ProgressBarFill, set_progress_bar};

use crate::states::running::game::battlescape::status_panel::stability_readout::components::{
    StabilityBar, Steadiness,
};

type ShooterView<'a> = (
    &'a Stance,
    &'a Aiming,
    &'a Position,
    &'a Facing,
    Option<&'a Suppressed>,
);

type WeaponStabilityRead<'a> = (&'a Stable, Option<&'a WeaponBraceBonus>);

#[derive(SystemParam)]
pub(in crate::states::running::game::battlescape::status_panel) struct ShooterReadQueries<'w, 's> {
    shooters: Query<'w, 's, ShooterView<'static>>,
    wields:   Query<'w, 's, &'static Wields>,
    weapons:  Query<'w, 's, WeaponStabilityRead<'static>, With<Weapon>>,
    melee:    Query<'w, 's, (), With<MeleeWeapon>>,
}

#[derive(SystemParam)]
pub(in crate::states::running::game::battlescape::status_panel) struct StabilityBarWriter<'w, 's> {
    bars:     Query<'w, 's, Entity, With<StabilityBar>>,
    children: Query<'w, 's, &'static Children>,
    fills:    Query<'w, 's, &'static mut Node, With<ProgressBarFill>>,
}

pub(in crate::states::running::game::battlescape::status_panel) fn update_stability_readout(
    selected: Res<SelectedShooter>,
    cover: Option<Res<CoverLedger>>,
    tuning: Option<Res<CombatTuning>>,
    brace_cells: Option<Res<BraceStairCells>>,
    surface: Option<Res<SurfaceGrid>>,
    reads: ShooterReadQueries,
    mut writer: StabilityBarWriter,
) {
    let Ok(bar) = writer.bars.single() else {
        return;
    };

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
    let weapon = reads
        .wields
        .get(entity)
        .ok()?
        .ranged_weapon(|e| reads.melee.get(e).is_ok())?;
    let (stable, brace_bonus) = reads.weapons.get(weapon).ok()?;
    let stable = *stable;
    let brace_bonus = brace_bonus.copied().unwrap_or_else(WeaponBraceBonus::none);
    let cover = cover?;
    let tuning = tuning?;
    let brace_cells = brace_cells?;
    let surface = surface?;

    let terrain_braced = terrain_braces(*position, **stance, brace_cells, surface);

    let shooter = Shooter {
        stance,
        aiming,
        position,
        facing,
        suppressed,
    };
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
