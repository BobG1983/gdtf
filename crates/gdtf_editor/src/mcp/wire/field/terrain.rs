//! The Terrain form's own fields, one arm per control its field stack draws.

use serde::{Deserialize, Serialize};

use super::draft_name::EditorDraftNameNet;
use crate::mcp::wire::{
    armor::{ArmorHardnessNet, ArmorProtectionNet},
    attachment::DamageTypeNet,
    fire_mode::HitTypeNet,
    list::EditorListIndexNet,
    on_death::OnDeathVariantNet,
    terrain::{
        BlocksPathingNet, FootfallNet, HeightBandNet, LeavesBehindNet, LosBlockingNet,
        MountedWeaponNet, TerrainHpNet, TerrainViewNet, TerrainViewSpriteNet,
    },
    terrain_kind::TerrainKindNet,
    weapon::{ExplodeDamageNet, FieldKeyNet},
};

/// One field of the Terrain draft, carrying the value it is set to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(in crate::mcp) enum TerrainFieldNet {
    /// The draft's kind pick.
    Kind(TerrainKindNet),
    /// The draft's display name.
    DisplayName(EditorDraftNameNet),
    /// The draft's hit points, written to both the cover and the slab field.
    Hp(TerrainHpNet),
    /// The draft's armor protection.
    ArmorProtection(ArmorProtectionNet),
    /// The draft's armor hardness.
    ArmorHardness(ArmorHardnessNet),
    /// The draft's height band, for the kinds that carry one.
    HeightBand(HeightBandNet),
    /// The draft's footfall sound, on a Slab.
    Footfall(FootfallNet),
    /// The draft's mounted weapon, on an Emplacement, set or cleared.
    MountedWeapon(Option<MountedWeaponNet>),
    /// The draft's pathing override, set or cleared.
    BlocksPathing(Option<BlocksPathingNet>),
    /// The draft's line-of-sight override, set or cleared.
    BlocksLos(Option<LosBlockingNet>),
    /// What the draft's piece leaves standing when it is destroyed.
    LeavesBehind(LeavesBehindNet),
    /// The sprite one owed view of the draft is drawn with.
    View {
        /// Which view the row names.
        view:   TerrainViewNet,
        /// The sprite def key that draws it.
        sprite: TerrainViewSpriteNet,
    },
    /// Which variant the on-death effect at one index is on.
    OnDeathVariant {
        /// Which effect of the list.
        index:   EditorListIndexNet,
        /// The variant its combo is set to.
        variant: OnDeathVariantNet,
    },
    /// The explode effect's hit geometry, at one index.
    OnDeathHitType {
        /// Which effect of the list.
        index:    EditorListIndexNet,
        /// The geometry its row draws.
        hit_type: HitTypeNet,
    },
    /// The explode effect's blast damage, at one index.
    OnDeathDamage {
        /// Which effect of the list.
        index:  EditorListIndexNet,
        /// The flat damage each cell takes.
        damage: ExplodeDamageNet,
    },
    /// The explode effect's damage channel, at one index.
    OnDeathDamageType {
        /// Which effect of the list.
        index:       EditorListIndexNet,
        /// The channel the blast deals on.
        damage_type: DamageTypeNet,
    },
    /// The leave-field effect's field key, at one index.
    OnDeathField {
        /// Which effect of the list.
        index: EditorListIndexNet,
        /// The catalog key of the field left behind.
        field: FieldKeyNet,
    },
}
