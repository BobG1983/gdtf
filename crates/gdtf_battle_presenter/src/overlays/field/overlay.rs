//! The area-damage-field overlay (GTW-545, child GTW-41f): the persistent per-cell VIEW of the
//! sim's live [`FieldRegistry`](gdtf_battle_sim::FieldRegistry) — one translucent tile per fielded
//! cell so a seeded field (a toxic-waste pool, an electrified floor, a patch of burning ground) is
//! VISIBLE on the battlefield, tinted by the field's [`DamageType`](gdtf_battle_sim::DamageType)
//! flavour.
//!
//! # One-way sim read (ADR-0001)
//!
//! Unlike the presenter-owned `ReachableCells` read-seam (which the INPUT
//! crate populates), this overlay reads the AUTHORITATIVE sim
//! [`FieldRegistry`](gdtf_battle_sim::FieldRegistry) resource DIRECTLY (a battle-lifetime
//! [`Resource`](bevy::prelude::Resource) `setup_battle` inserts), the `input → presenter → sim`
//! direction: the presenter READS the sim's field placements and DRAWS them; it never writes the
//! sim. This is the SAME shape as the terrain draw reading the cover ledger — the field is sim
//! truth, the tint is the view.
//!
//! # Active-storey hard-cut
//!
//! [`draw_field_overlay`] reads [`ActiveLevel`](crate::ActiveLevel) live every frame and shows
//! ONLY the fields whose storey equals the active level — a `PageUp` to L1 shows the L1 fields with
//! no extra wiring (the `draw_reachable_overlay` precedent).
//!
//! # Mutate, never respawn
//!
//! The system maintains a POOL of [`FieldCellSprite`]-marked sprites: it reuses an existing entity
//! for each field cell to draw (moving its [`Transform`], setting its tint, showing it) and HIDES
//! surplus pooled entities it no longer needs — it never despawn-then-respawns the set each frame
//! (the UI-mutate-not-respawn convention). The walk itself is the shared
//! [`draw_pool`](crate::overlays::pool::draw_pool) helper (GTW-568), which owns the `set_if_neq`
//! visibility flips.

use bevy::{camera::visibility::RenderLayers, prelude::*};
use gdtf_battle_sim::{CellLevel, DamageType, FieldRegistry, Level};

use crate::{
    ActiveLevel, CELL_PX, Layer, WORLD_RENDER_LAYER, cell_to_world_layered,
    overlays::pool::draw_pool,
};

/// Marker for a pooled area-damage-field cell [`Sprite`].
///
/// Plumbing around the framework sprite (the no-bare-types framework carve-out, the same
/// justification the `ReachableCellSprite` marker uses):
/// [`draw_field_overlay`] queries `With<FieldCellSprite>` to find and MUTATE the pooled sprites in
/// place rather than despawn-respawning them each frame.
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct FieldCellSprite;

/// The translucent overlay tint of a field cell — the per-cell + its resolved [`DamageType`]
/// swatch to draw.
///
/// A named pair (no-bare-types: the `(CellLevel, DamageType)` pair is a domain value — the fielded
/// cell annotated with the damage-type flavour that decides its hazard tint). [`field_draws`]
/// builds the list of these for the active storey and [`draw_field_overlay`] draws each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FieldDraw {
    /// The `(cell, level)` carrying the live field.
    cell:        CellLevel,
    /// The damage-type flavour of the field at `cell` — decides the [`field_tint`] swatch.
    damage_type: DamageType,
}

/// The base overlay alpha every field tint is drawn at — translucent so the fielded floor reads as
/// a "danger zone here" wash while the terrain, actors, and route feedback stay legible over it.
///
/// Framework plumbing — a literal alpha handed into a [`Color`], not a domain quantity (the
/// `CELL_PX`-class const carve-out).
const FIELD_TINT_ALPHA: f32 = 0.42;

/// The hazard tint for a field's [`DamageType`] flavour — the presenter-owned damage-type → swatch
/// mapping the overlay wash uses.
///
/// Presenter-owned (it picks the literal sRGB swatch the overlay draws), mirroring the FCT palette
/// rationale: the swatch reads the ENVIRONMENTAL hazard family per wheel node — a caustic green for
/// [`Chem`](DamageType::Chem) (toxic pool), an electric blue for [`Shock`](DamageType::Shock)
/// (live wire), a fiery orange-red for [`Plasma`](DamageType::Plasma) (burning ground), and a
/// neutral hazard amber for the remaining nodes — each at [`FIELD_TINT_ALPHA`] so the floor reads
/// as a translucent danger wash. Total over the seven wheel nodes, so a field of any authored
/// [`DamageType`] resolves to a swatch.
#[must_use]
const fn field_tint(damage_type: DamageType) -> Color {
    let (red, green, blue) = match damage_type {
        // Toxin / acid / gas — a caustic sickly green (the toxic-waste-pool family).
        DamageType::Chem => (0.35, 0.80, 0.20),
        // Arc / EMP — an electric cyan-blue (the electrified-floor / live-wire family).
        DamageType::Shock | DamageType::Las => (0.25, 0.70, 0.95),
        // Superheated / concussion — a fiery orange-red (the burning-ground / blast family).
        DamageType::Plasma | DamageType::Blast => (0.95, 0.35, 0.10),
        // Slugs / edges — a neutral hazard amber for the remaining wheel nodes.
        DamageType::Kinetic | DamageType::Rend => (0.90, 0.65, 0.15),
    };
    Color::srgba(red, green, blue, FIELD_TINT_ALPHA)
}

/// The pooled field-cell sprite query — mutated in place over the [`FieldCellSprite`] pool. A
/// `type` alias so the system signature stays under the `type_complexity` lint. Framework plumbing
/// (a query alias), exempt from no-bare-types.
type FieldSpriteQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Transform,
        &'static mut Sprite,
        &'static mut Visibility,
    ),
    With<FieldCellSprite>,
>;

/// Resolve the field cells to draw for the current active storey — the PURE draw-decision helper
/// that clips the sim's live [`FieldRegistry`] to the active level.
///
/// Returns one [`FieldDraw`] per placed field whose storey index equals `active_level`, each
/// carrying the field's [`DamageType`] flavour (read off its
/// [`FieldDef`](gdtf_battle_sim::FieldDef)). Sorted by `(z, y, x)` so the pooled-sprite assignment
/// is deterministic frame to frame (the [`FieldRegistry`] iterates a `HashMap` in unspecified
/// order). Called by [`draw_field_overlay`] and directly tested by the sibling `test` module
/// without an [`App`].
fn field_draws(fields: &FieldRegistry, active_level: Level) -> Vec<FieldDraw> {
    let active_z = i32::from(*active_level);
    let mut draws: Vec<FieldDraw> = fields
        .iter()
        .filter(|(cell, _placed)| cell.z == active_z)
        .map(|(cell, placed)| FieldDraw {
            cell:        *cell,
            damage_type: placed.def().damage_type,
        })
        .collect();
    // Deterministic order (HashMap iteration is unspecified) so a stable field set assigns the same
    // pooled sprite to the same cell each frame.
    draws.sort_by_key(|draw| (draw.cell.z, draw.cell.y, draw.cell.x));
    draws
}

/// `Update` ([`PresenterSystems::Draw`](crate::PresenterSystems)): draw the area-damage-field
/// overlay — one cell-keyed translucent [`Sprite`] per live field on the active storey (GTW-545).
///
/// Reads the AUTHORITATIVE sim [`FieldRegistry`] (a battle-lifetime resource `setup_battle` seeds
/// and the GTW-547 spawn API grows) and the [`ActiveLevel`] (the active-storey hard-cut), then
/// maintains a POOL of [`FieldCellSprite`] sprites:
///
/// - for each field cell ON the active storey, it takes (or lazily spawns) a pooled sprite, moves
///   it to [`cell_to_world_layered`] at the [`Layer::Field`] band, sets its tint to the field's
///   [`field_tint`] (per its [`DamageType`]), and shows it;
/// - every surplus pooled sprite is [`Visibility::Hidden`] — NEVER despawned (the
///   UI-mutate-not-respawn convention).
///
/// The overlay follows `PageUp` with NO extra wiring: it reads `Res<ActiveLevel>` live every
/// frame, so after `PageUp` raises `ActiveLevel` to L1 the next Draw renders only L1 fields.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the lazy pool growth, the [`FieldRegistry`] /
/// [`ActiveLevel`] reads, and the [`FieldSpriteQuery`] for the pooled sprites. Battle-gated in
/// [`TopDownRendererPlugin`](crate::TopDownRendererPlugin) by `resource_exists::<FieldRegistry>`
/// (the sim's live-field witness), so it stays inert when no battle has seeded a field registry.
/// Pure VIEW: it only READS the sim registry and DRAWS; it never writes the sim.
pub fn draw_field_overlay(
    mut commands: Commands,
    fields: Res<FieldRegistry>,
    active: Res<ActiveLevel>,
    mut sprites: FieldSpriteQuery,
) {
    let active_level: Level = **active;
    let draws = field_draws(&fields, active_level);

    // The world position of a field-cell sprite (shared by the reuse + grow paths).
    let world_at = |cell: CellLevel| cell_to_world_layered(cell.cell(), active_level, Layer::Field);
    // The shared pooled-draw walk (GTW-568): reuse the pooled sprites in iteration order
    // (move + tint), lazily spawn past the pool, hide the surplus — the helper owns the
    // set_if_neq visibility flips (mutate, not respawn).
    draw_pool(
        sprites.iter_mut(),
        draws,
        |draw, (transform, sprite, _)| {
            transform.translation = world_at(draw.cell);
            sprite.color = field_tint(draw.damage_type);
        },
        |draw| {
            spawn_field_sprite(
                &mut commands,
                world_at(draw.cell),
                field_tint(draw.damage_type),
            );
        },
        |(_, _, visibility)| visibility,
    );
}

/// Lazily spawn ONE pooled field-cell sprite at `world` with the hazard `tint`.
///
/// A one-cell translucent [`Sprite`] on the world render layer, shown from spawn. Pooled (kept +
/// reused / hidden, never despawned), so this runs only when the field set grows past the current
/// pool size.
fn spawn_field_sprite(commands: &mut Commands, world: Vec3, tint: Color) {
    commands.spawn((
        FieldCellSprite,
        Sprite {
            color: tint,
            custom_size: Some(Vec2::splat(CELL_PX)),
            ..default()
        },
        Transform::from_translation(world),
        Visibility::Visible,
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));
}

#[cfg(test)]
mod test {
    use gdtf_battle_sim::{
        Cell, CellLevel, DamageType, FieldDamage, FieldDef, FieldDuration, FieldRegistry,
        ImmuneArmorTypes, Level,
    };

    use super::{FIELD_TINT_ALPHA, field_draws, field_tint};

    /// A `FieldDef` of `damage_type` (the only field the overlay reads) — a permanent zone with an
    /// arbitrary drain and no immunity (the draw path never reads those).
    fn field_def(damage_type: DamageType) -> FieldDef {
        FieldDef::new(
            FieldDamage::new(3),
            damage_type,
            ImmuneArmorTypes::new([]),
            FieldDuration::Permanent,
        )
    }

    /// A registry with a `Chem` field on L0 and a `Shock` field on L1 — the cross-storey fixture
    /// the hard-cut test slices.
    fn two_storey_registry() -> FieldRegistry {
        let mut registry = FieldRegistry::new();
        registry.spawn(
            CellLevel::new(Cell::new(3, 4), Level::new(0)),
            field_def(DamageType::Chem),
        );
        registry.spawn(
            CellLevel::new(Cell::new(5, 6), Level::new(1)),
            field_def(DamageType::Shock),
        );
        registry
    }

    /// The active-storey hard-cut: `field_draws` returns ONLY the fields on the active level — the
    /// L0 draw omits the L1 field and vice versa.
    #[test]
    fn field_draws_hard_cuts_to_the_active_storey() {
        let registry = two_storey_registry();

        let l0 = field_draws(&registry, Level::new(0));
        assert_eq!(
            l0.len(),
            1,
            "L0 must draw exactly the one L0 field, got {l0:?}"
        );
        assert_eq!(l0[0].cell.z, 0, "the L0 draw must be the L0 field");
        assert_eq!(
            l0[0].damage_type,
            DamageType::Chem,
            "the L0 field carries its authored Chem flavour",
        );

        let l1 = field_draws(&registry, Level::new(1));
        assert_eq!(
            l1.len(),
            1,
            "L1 must draw exactly the one L1 field, got {l1:?}"
        );
        assert_eq!(l1[0].cell.z, 1, "the L1 draw must be the L1 field");
        assert_eq!(
            l1[0].damage_type,
            DamageType::Shock,
            "the L1 field carries its authored Shock flavour",
        );

        // A storey with no field draws nothing.
        let l2 = field_draws(&registry, Level::new(2));
        assert!(
            l2.is_empty(),
            "a storey with no field must draw nothing, got {l2:?}"
        );
    }

    /// The overlay tint is a translucent per-damage-type wash: distinct hazard families read
    /// distinctly (Chem green vs Shock blue vs Plasma fire), every tint at the base overlay alpha.
    #[test]
    fn field_tint_discriminates_hazard_families_at_the_overlay_alpha() {
        let chem = field_tint(DamageType::Chem).to_srgba();
        let shock = field_tint(DamageType::Shock).to_srgba();
        let plasma = field_tint(DamageType::Plasma).to_srgba();

        assert!(
            (chem.alpha - FIELD_TINT_ALPHA).abs() < 0.001,
            "every field tint is drawn at the translucent overlay alpha",
        );
        assert_ne!(
            (chem.red, chem.green, chem.blue),
            (shock.red, shock.green, shock.blue),
            "the Chem toxic green must read distinctly from the Shock electric blue",
        );
        assert_ne!(
            (chem.red, chem.green, chem.blue),
            (plasma.red, plasma.green, plasma.blue),
            "the Chem toxic green must read distinctly from the Plasma fire",
        );
    }
}
