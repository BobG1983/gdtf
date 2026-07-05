//! The fire-mode control's per-mode TU-cost sub-lines: the cost-input reads bundled
//! into one [`SystemParam`] and the [`sync_mode_tu_cost_lines`] writer that keeps each
//! offered segment's "{n} TU" line equal to the exact sim charge (GTW-303). Split out
//! of the monolithic `mode_panel.rs` (GTW-583); the control rationale lives on the
//! parent `mode_panel` module.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    Aiming, FireMode, MeleeWeapon, TuMax, WieldedBy, Wields, mode_tu_cost, tuning::CombatTuning,
};
use gdtf_ui::{
    Segment, SegmentColors, SegmentIndex, SegmentSubLabel, SegmentSubText, set_segment_sub_line,
};

use super::order::MODE_ORDER;
use crate::states::running::game::battlescape::action_bar::components::ModeControl;

/// Read-only [`Query`] data the cost-line system reads off the SELECTED GANGER to compute
/// each offered mode's TU charge: its [`TuMax`] (the round-start ceiling the charge is a
/// percentage of) and its [`Aiming`] flag (selects the aim premium). The third input, the
/// [`FireMode`] selector (the offered specs), lives on the related WEAPON entity since
/// GTW-323 slice 3 — read separately ([`CostWeapon`]) through the ganger's [`Wields`]
/// relationship, NOT off the ganger.
///
/// Named to keep [`sync_mode_tu_cost_lines`]'s signature legible (clippy `type_complexity`).
type CostShooter = (&'static TuMax, &'static Aiming);

/// Read-only [`Query`] data the cost-line system reads off a WEAPON entity
/// (`With<`[`WieldedBy`]`>`): its [`FireMode`] selector (the offered specs), resolved
/// through the selected ganger's [`Wields`] relationship since GTW-323 slice 3.
type CostWeapon = &'static FireMode;

/// Read-only [`Query`] FILTER selecting each Mode [`Segment`] (its [`SegmentIndex`] +
/// [`Children`]), the shape [`set_segment_sub_line`](gdtf_ui::set_segment_sub_line) reads.
///
/// Aliased so [`sync_mode_tu_cost_lines`]'s `set_segment_sub_line` argument query stays
/// legible (clippy `type_complexity`).
type ModeSegment = (&'static SegmentIndex, &'static Children);

/// The cost INPUTS + recompute-trigger reads [`sync_mode_tu_cost_lines`] needs, grouped into
/// one [`SystemParam`] so the system stays under clippy's argument-count gate (the
/// [`set_segment_sub_line`](gdtf_ui::set_segment_sub_line) call already fixes four params).
///
/// Bundling the selection + tuning + per-shooter reads + the two change detectors keeps the
/// firing-cost derivation's read surface in one named value (no bare framework tuple) without
/// taking exclusive `&mut World`.
#[derive(SystemParam)]
pub(in crate::states::running::game::battlescape) struct ModeCostInputs<'w, 's> {
    /// The current selection — the shooter whose modes' costs are displayed.
    selected:       Res<'w, SelectedShooter>,
    /// The live combat tuning the per-shot charge reads (the aim premium factor).
    tuning:         Res<'w, CombatTuning>,
    /// The selected GANGER's cost inputs ([`TuMax`] / [`Aiming`]).
    shooters:       Query<'w, 's, CostShooter>,
    /// The selected ganger's [`Wields`] relationship (resolves the weapon entity).
    wields:         Query<'w, 's, &'static Wields>,
    /// The wielded WEAPON entity's [`FireMode`] selector ([`CostWeapon`]) — the offered specs.
    weapons:        Query<'w, 's, CostWeapon, With<WieldedBy>>,
    /// The [`MeleeWeapon`] marker probe (GTW-505 C5) — so the RANGED weapon's modes resolve
    /// excluding the melee weapon the ganger also wields.
    melee:          Query<'w, 's, (), With<MeleeWeapon>>,
    /// Detects an Aim flip (a [`Changed<Aiming>`](Changed) on any ganger) → recompute.
    aim_changed:    Query<'w, 's, (), Changed<Aiming>>,
    /// Detects the [`ModeControl`] freshly spawned (the GTW-255 auto-select ordering trap).
    added_controls: Query<'w, 's, (), Added<ModeControl>>,
}

/// MUTATES each OFFERED Mode segment's sub-line to its aim-adjusted per-shot TU cost
/// ("{n} TU"), and CLEARS the sub-line of a non-offered / hidden segment (GTW-303).
///
/// Each segment of the firemode control shows its mode name (the primary
/// [`SegmentText`](gdtf_ui::SegmentText) label, top) over its TU cost (a quieter
/// [`SegmentSubText`](gdtf_ui::SegmentSubText) sub-line, bottom — slice 1's
/// [`set_segment_sub_line`](gdtf_ui::set_segment_sub_line)). The displayed cost is the EXACT
/// value the sim charges: [`mode_tu_cost`](gdtf_battle_sim::mode_tu_cost) of the selected
/// shooter's [`FireModeSpec`](gdtf_battle_sim::FireModeSpec) / [`TuMax`] / [`Aiming`] under the live
/// [`CombatTuning`](gdtf_battle_sim::tuning::CombatTuning) — reusing that one function so the
/// display can never diverge from the charge (no presenter-side re-derivation). When Aim is
/// ON the cost includes the aim ×premium; when OFF it reverts to the hip-fire base — the
/// `Changed<Aiming>` trigger re-derives both directions IN PLACE (the sub-line text mutates,
/// the node id stays stable, [[ui-mutate-not-respawn]]).
///
/// A mode is shown only when the selected weapon offers it ([`FireMode::iter`]); a
/// non-offered mode (its segment collapsed by [`rebuild_mode_segments`](super::visibility::rebuild_mode_segments)) gets its sub-line
/// CLEARED so a stale cost never lingers on a hidden segment. No selection, an unarmed
/// selection (no [`FireMode`]), or a selection missing [`TuMax`]/[`Aiming`] clears EVERY
/// segment's sub-line (the panel is hidden in that state anyway — GTW-273).
///
/// It re-derives on the same signals [`rebuild_mode_segments`](super::visibility::rebuild_mode_segments) / [`sync_aim_switch_state`]
/// key off: a [`SelectedShooter`](gdtf_battle_input::SelectedShooter) change, a
/// [`Changed<Aiming>`](Changed) on any ganger (Aim flips on the selected shooter), or the
/// [`ModeControl`] freshly spawned ([`Added<ModeControl>`](Added)) — the GTW-255 auto-select
/// ordering trap, where the selection is filled several frames before the control exists.
/// Otherwise it early-returns (change-detection hygiene). Runs `.after(UiSystems::ApplyTheme)`
/// alongside [`rebuild_mode_segments`](super::visibility::rebuild_mode_segments) so the offered set is settled before the cost lines are
/// written.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] (the sub-line spawn/despawn go through it),
/// the [`ModeCostInputs`] bundle (selection + tuning + per-shooter reads + the two recompute
/// detectors), a marker-only [`ModeControl`] root query (which controls to write), and the
/// three [`set_segment_sub_line`](gdtf_ui::set_segment_sub_line) argument queries (the control
/// `(Children, SegmentColors)` read, the segments, and the writable sub-text query) — no
/// `&mut World`.
#[allow(
    clippy::type_complexity,
    reason = "param tuple aliased where possible; the set_segment_sub_line call signature \
    fixes the controls / segments / sub-texts query shapes"
)]
pub(in crate::states::running::game::battlescape) fn sync_mode_tu_cost_lines(
    mut commands: Commands,
    inputs: ModeCostInputs,
    mode_controls: Query<Entity, With<ModeControl>>,
    controls: Query<(&Children, &SegmentColors)>,
    segments: Query<ModeSegment, With<Segment>>,
    mut sub_texts: Query<&mut Text, With<SegmentSubText>>,
) {
    // Re-derive on a selection change, an Aim flip (Changed<Aiming>), OR when the Mode control
    // was JUST spawned (the GTW-255 auto-select ordering trap — see the doc comment). Otherwise
    // early-return (change-detection hygiene).
    let control_just_spawned = inputs.added_controls.iter().next().is_some();
    let aim_flipped = inputs.aim_changed.iter().next().is_some();
    if !inputs.selected.is_changed() && !aim_flipped && !control_just_spawned {
        return;
    }

    // The selected ganger's cost inputs (TuMax/Aiming) + its wielded RANGED weapon's FireMode
    // selector (resolved `ganger → Wields → the ranged weapon entity`, GTW-323 slice 3).
    // GTW-505 C5: resolve through `Wields::ranged_weapon` (excluding the `MeleeWeapon`-marked
    // entity) — NOT `Wields::weapon` (the spawn-order fragile first entity) — so the cost
    // lines display the GUN's modes, never the melee weapon's. Absent / unarmed (no ganger
    // inputs, no Wields, or no ranged FireMode) → every sub-line is cleared below.
    let ganger = **inputs.selected;
    let shooter_inputs = ganger.and_then(|shooter| inputs.shooters.get(shooter).ok());
    let weapon_mode = ganger
        .and_then(|shooter| inputs.wields.get(shooter).ok())
        .and_then(|w| w.ranged_weapon(|entity| inputs.melee.get(entity).is_ok()))
        .and_then(|weapon| inputs.weapons.get(weapon).ok());

    for control in &mode_controls {
        for (index, mode) in MODE_ORDER.iter().enumerate() {
            // The offered spec for this mode (off the weapon) + the ganger's TuMax/Aiming →
            // its cost line; a non-offered mode / no armed selection → clear the sub-line
            // (None). The cost is the EXACT sim charge (`mode_tu_cost`), so the display
            // equals the debit.
            let label = shooter_inputs
                .zip(weapon_mode)
                .and_then(|((tu_max, aiming), weapon)| {
                    let spec = weapon.iter().find(|s| s.kind == *mode)?;
                    let cost = mode_tu_cost(spec, tu_max, aiming, &inputs.tuning);
                    Some(SegmentSubLabel::new(format!("{} TU", *cost)))
                });
            set_segment_sub_line(
                &mut commands,
                control,
                index,
                label.as_ref(),
                &controls,
                &segments,
                &mut sub_texts,
            );
        }
    }
}
