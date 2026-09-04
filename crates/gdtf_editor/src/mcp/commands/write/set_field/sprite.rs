//! The Sprite form's own field arms, written through the draft its panels write.

use crate::{
    mcp::{
        commands::write::form_fault::{ANIMATION_OFF, FormWriteFault},
        wire::{
            EditorDraftNameNet, EditorListIndexNet, SpriteAnimatedNet, SpriteFacingNet,
            SpriteFieldNet, SpriteFpsNet, SpritePxNet, SpriteSourceNet,
        },
    },
    sprite_form::SpriteDraft,
};

// Every fps, frame and frame-source control is drawn only while animation is on.
const fn behind_the_gate(draft: &SpriteDraft) -> Result<(), FormWriteFault> {
    if draft.is_animated() {
        Ok(())
    } else {
        Err(FormWriteFault::Gated(ANIMATION_OFF))
    }
}

// The source stored at one frame, or the fault a past-the-end index answers.
fn frame_at(draft: &SpriteDraft, index: usize) -> Result<SpriteSourceNet, FormWriteFault> {
    let frame = draft
        .def()
        .animation
        .as_ref()
        .and_then(|animation| animation.frames.get(index));
    match frame {
        Some(source) => Ok(SpriteSourceNet::from_source(source)),
        None => Err(FormWriteFault::bad(format!(
            "frame {index} is past the end of the animation's frame list"
        ))),
    }
}

// The facing the draft keyed the override under, and the source it holds, after the write.
fn stored_override(
    draft: &SpriteDraft,
    facing: SpriteFacingNet,
) -> (SpriteFacingNet, Option<SpriteSourceNet>) {
    match draft.facing_override(facing.to_facing()) {
        Some(source) => (
            SpriteFacingNet::from_facing(facing.to_facing()),
            Some(SpriteSourceNet::from_source(source)),
        ),
        None => (facing, None),
    }
}

fn write_fps(draft: &mut SpriteDraft, fps: SpriteFpsNet) -> Result<SpriteFieldNet, FormWriteFault> {
    behind_the_gate(draft)?;
    if !SpriteDraft::FPS_RANGE.contains(&*fps) {
        return Err(FormWriteFault::bad(format!(
            "{} is outside {:?}, the range the animation's FPS input offers",
            *fps,
            SpriteDraft::FPS_RANGE
        )));
    }
    draft.set_fps(fps.to_fps());
    let stored = draft
        .def()
        .animation
        .as_ref()
        .map_or(fps, |animation| SpriteFpsNet::from_fps(animation.fps));
    Ok(SpriteFieldNet::Fps(stored))
}

fn write_frame(
    draft: &mut SpriteDraft,
    index: usize,
    source: &SpriteSourceNet,
) -> Result<SpriteFieldNet, FormWriteFault> {
    behind_the_gate(draft)?;
    frame_at(draft, index)?;
    draft.set_frame(index, source.to_source());
    Ok(SpriteFieldNet::Frame {
        index:  EditorListIndexNet::new(index),
        source: frame_at(draft, index)?,
    })
}

/// Write one Sprite field, answering the field as the draft stores it.
pub(super) fn write(
    draft: &mut SpriteDraft,
    field: SpriteFieldNet,
) -> Result<SpriteFieldNet, FormWriteFault> {
    match field {
        SpriteFieldNet::Name(name) => {
            draft.set_name((*name).clone());
            Ok(SpriteFieldNet::Name(EditorDraftNameNet::new(draft.name())))
        }
        SpriteFieldNet::BaseSource(source) => {
            draft.set_base_source(source.to_source());
            Ok(SpriteFieldNet::BaseSource(SpriteSourceNet::from_source(
                &draft.def().source,
            )))
        }
        SpriteFieldNet::AnchorX(x) => {
            draft.set_anchor(x.to_px(), draft.def().anchor.y);
            Ok(SpriteFieldNet::AnchorX(SpritePxNet::from_px(
                draft.def().anchor.x,
            )))
        }
        SpriteFieldNet::AnchorY(y) => {
            draft.set_anchor(draft.def().anchor.x, y.to_px());
            Ok(SpriteFieldNet::AnchorY(SpritePxNet::from_px(
                draft.def().anchor.y,
            )))
        }
        SpriteFieldNet::Fps(fps) => write_fps(draft, fps),
        SpriteFieldNet::FacingOverride { facing, source } => {
            draft.set_facing_override(
                facing.to_facing(),
                source.as_ref().map(SpriteSourceNet::to_source),
            );
            let (stored_facing, source) = stored_override(draft, facing);
            Ok(SpriteFieldNet::FacingOverride {
                facing: stored_facing,
                source,
            })
        }
        SpriteFieldNet::Frame { index, source } => write_frame(draft, *index, &source),
        SpriteFieldNet::Animated(animated) => {
            if *animated {
                draft.enable_animation();
            } else {
                draft.disable_animation();
            }
            Ok(SpriteFieldNet::Animated(SpriteAnimatedNet::new(
                draft.is_animated(),
            )))
        }
    }
}
