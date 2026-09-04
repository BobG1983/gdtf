//! Wire mirrors of the editor's own lifecycle state, mode tabs, and write outcomes.

mod armor;
mod attachment;
mod camera;
mod cell;
mod content_family;
mod delete;
mod draft;
mod facing;
mod family;
mod field;
mod fire_mode;
mod gang;
mod grid;
mod injury;
mod injury_sub_tab;
mod key;
mod last_save;
mod list;
mod melee_weapon;
mod mode;
mod on_death;
mod outcome;
mod painted;
mod pairing;
mod phase;
mod placement;
mod prefab;
mod prefab_refusal;
mod refusal;
mod save_fault;
mod sprite;
mod stat_target;
mod terrain;
mod terrain_kind;
#[cfg(test)]
mod test;
mod toggle;
mod validation;
mod view;
mod wait;
mod weapon;
mod weighting;

pub(in crate::mcp) use armor::{
    ArmorFloorNet, ArmorHardnessNet, ArmorIntegrityNet, ArmorProtectionNet, ArmorTypeNet,
    BodyPartNet,
};
pub(in crate::mcp) use attachment::{
    AttachmentEffectNet, AttachmentSlotNet, DamageTypeNet, FatalBiasNet, WeaponDamageNet,
    WeaponPunchNet, WeaponShredNet,
};
pub(in crate::mcp) use camera::{EditorPanNet, EditorZoomNet};
pub(in crate::mcp) use cell::{EditorCellXNet, EditorCellYNet, EditorLevelNet};
pub(in crate::mcp) use content_family::ContentFamilyNet;
pub(in crate::mcp) use delete::{DeleteCancelNet, DeleteFamilyNet, DeleteKeyNet, DeleteOutcomeNet};
pub(in crate::mcp) use draft::{EditorDraftOutcomeNet, EditorDraftRonNet};
#[cfg(test)]
pub(in crate::mcp) use facing::TerrainCornerNet;
pub(in crate::mcp) use facing::TerrainFacingNet;
pub(in crate::mcp) use family::{EditorFamilyEntryNet, EditorFamilyLabelNet, EditorFamilyRowNet};
#[cfg(test)]
pub(in crate::mcp) use field::FieldTurnsNet;
pub(in crate::mcp) use field::{
    ArmorFieldNet, AttachmentFieldNet, EditorDraftNameNet, EditorFieldNet, FieldDamageNet,
    FieldDurationNet, FieldFormFieldNet, GangFieldNet, InjuryFieldNet, MeleeWeaponFieldNet,
    SpriteFieldNet, TerrainFieldNet, ThemeFieldNet, WeaponFieldNet, WeightingFieldNet,
};
pub(in crate::mcp) use fire_mode::{FireModeSpecNet, HitTypeNet};
pub(in crate::mcp) use gang::{GangAttributeNet, GangAttributeValueNet};
pub(in crate::mcp) use grid::{
    EditorGridHeightNet, EditorGridLevelsNet, EditorGridSizeNet, EditorGridWidthNet,
};
#[cfg(test)]
pub(in crate::mcp) use injury::{BleedAmountNet, MovementCostFactorNet, StatDeltaNet};
pub(in crate::mcp) use injury::{
    InjuryCategoryNet, InjuryEffectNet, InjuryKeyNet, InjurySeverityNet, InjuryTextNet,
};
pub(in crate::mcp) use injury_sub_tab::InjurySubTabNet;
#[cfg(test)]
pub(in crate::mcp) use key::SavedPathNet;
pub(in crate::mcp) use key::{EditorContentNameNet, EditorKeyNet, TerrainKeyNet, ThemeKeyNet};
pub(in crate::mcp) use last_save::{EditorLastSaveRowNet, LastSaveOutcomeNet};
pub(in crate::mcp) use list::{
    EditorListIndexNet, EditorListMemberNet, EditorListNet, EditorListOpNet,
};
pub(in crate::mcp) use melee_weapon::{
    AttachmentKeyNet, FightModeSpecNet, HandednessNet, ReachNet, ShoveNet, WeaponSlotNet,
};
#[cfg(test)]
pub(in crate::mcp) use melee_weapon::{FightModeKindNet, SlotCapacityNet, StrikesNet, TuCostNet};
pub(in crate::mcp) use mode::EditorModeNet;
pub(in crate::mcp) use on_death::{OnDeathEffectNet, OnDeathVariantNet};
pub(in crate::mcp) use outcome::{
    EditorLoadOutcomeNet, EditorLoadPrefabOutcomeNet, EditorNewOutcomeNet, EditorSaveOutcomeNet,
};
pub(in crate::mcp) use painted::{PaintedMapNet, PaintedRowNet};
pub(in crate::mcp) use pairing::PairingOutcomeNet;
pub(in crate::mcp) use phase::EditorPhaseNet;
pub(in crate::mcp) use placement::PlacementVerdictNet;
pub(in crate::mcp) use prefab::{PrefabPlacementCountNet, SpawnRoleNet};
pub(in crate::mcp) use prefab_refusal::{PaintRefusalNet, SelectTileRefusalNet};
pub(in crate::mcp) use refusal::EditorRefusalNet;
pub(in crate::mcp) use save_fault::EditorSaveFaultNet;
pub(in crate::mcp) use sprite::{
    SpriteAnimatedNet, SpriteFacingNet, SpriteFpsNet, SpriteKeyNet, SpritePxNet, SpriteSourceNet,
};
#[cfg(test)]
pub(in crate::mcp) use stat_target::StatTargetNet;
pub(in crate::mcp) use terrain::{
    BlocksPathingNet, FootfallNet, HeightBandNet, LeavesBehindNet, LosBlockingNet,
    MountedWeaponNet, TerrainHpNet, TerrainTagNet, TerrainViewNet, TerrainViewSpriteNet,
};
pub(in crate::mcp) use terrain_kind::TerrainKindNet;
pub(in crate::mcp) use toggle::TerrainToggleNet;
pub(in crate::mcp) use validation::{
    ChecksCompleteNet, ValidationFindingNet, ValidationPublishedNet,
};
pub(in crate::mcp) use view::{EditorIsolateViewNet, EditorViewModeNet, EditorViewNet};
pub(in crate::mcp) use wait::EditorWaitConditionNet;
pub(in crate::mcp) use weapon::{
    AccuracyNet, BaseSpreadNet, DotDamageNet, DotEnabledNet, DotTurnsNet, ExplodeDamageNet,
    FieldKeyNet, KickbackNet, MagazineSizeNet, ReloadTuNet, StableNet, TrajectoryStyleNet,
};
pub(in crate::mcp) use weighting::{
    DamageContextNet, InjuryWeightNet, WeightingBucketNet, WeightingRowNet, WeightingTableNet,
};
