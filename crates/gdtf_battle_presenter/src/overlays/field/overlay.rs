//! Field-coverage tint tiles from the sim field registry.

use bevy::{camera::visibility::RenderLayers, prelude::*};
use gdtf_battle_sim::{
    effects::fields::FieldRegistry,
    prelude::{CellLevel, Level},
    weapon::DamageType,
};

use crate::{
    ActiveLevel, CELL_PX, Layer, WORLD_RENDER_LAYER, cell_to_world_layered,
    overlays::pool::draw_pool,
};

/// Marker on a field-coverage cell sprite.
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct FieldCellSprite;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FieldDraw {
    cell:        CellLevel,
    damage_type: DamageType,
}

const FIELD_TINT_ALPHA: f32 = 0.42;

#[must_use]
const fn field_tint(damage_type: DamageType) -> Color {
    let (red, green, blue) = match damage_type {
        DamageType::Chem => (0.35, 0.80, 0.20),
        DamageType::Shock | DamageType::Las => (0.25, 0.70, 0.95),
        DamageType::Plasma | DamageType::Blast => (0.95, 0.35, 0.10),
        DamageType::Kinetic | DamageType::Rend => (0.90, 0.65, 0.15),
    };
    Color::srgba(red, green, blue, FIELD_TINT_ALPHA)
}

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
    draws.sort_by_key(|draw| (draw.cell.z, draw.cell.y, draw.cell.x));
    draws
}

/// Draw field tints for cells on the active storey.
pub fn draw_field_overlay(
    mut commands: Commands,
    fields: Res<FieldRegistry>,
    active: Res<ActiveLevel>,
    mut sprites: FieldSpriteQuery,
) {
    let active_level: Level = **active;
    let draws = field_draws(&fields, active_level);

    let world_at = |cell: CellLevel| cell_to_world_layered(cell.cell(), active_level, Layer::Field);
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
        effects::fields::{FieldDamage, FieldDef, FieldDuration, FieldRegistry, ImmuneArmorTypes},
        prelude::{Cell, CellLevel, Level},
        weapon::DamageType,
    };

    use super::{FIELD_TINT_ALPHA, field_draws, field_tint};

    fn field_def(damage_type: DamageType) -> FieldDef {
        FieldDef::new(
            FieldDamage::new(3),
            damage_type,
            ImmuneArmorTypes::new([]),
            FieldDuration::Permanent,
        )
    }

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

        let l2 = field_draws(&registry, Level::new(2));
        assert!(
            l2.is_empty(),
            "a storey with no field must draw nothing, got {l2:?}"
        );
    }

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
