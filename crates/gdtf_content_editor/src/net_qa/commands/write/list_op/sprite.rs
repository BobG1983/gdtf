//! The Sprite draft's animation frames: add, remove, and the one reorder in the editor.

use crate::{
    net_qa::{
        commands::write::form_fault::{ANIMATION_OFF, FormWriteFault},
        wire::{EditorListMemberNet, EditorListOpNet, SpriteSourceNet},
    },
    sprite_form::SpriteDraft,
};

/// The animation frames the draft holds, as the reply reads them back.
pub(super) fn members(draft: &SpriteDraft) -> Vec<EditorListMemberNet> {
    draft
        .def()
        .animation
        .as_ref()
        .map(|animation| {
            animation
                .frames
                .iter()
                .map(|frame| EditorListMemberNet::SpriteFrame(SpriteSourceNet::from_source(frame)))
                .collect()
        })
        .unwrap_or_default()
}

// How many frames the animation holds right now.
fn frame_count(draft: &SpriteDraft) -> usize {
    draft
        .def()
        .animation
        .as_ref()
        .map_or(0, |animation| animation.frames.len())
}

fn remove(draft: &mut SpriteDraft, index: usize) -> Result<(), FormWriteFault> {
    let held = frame_count(draft);
    if held <= 1 {
        return Err(FormWriteFault::bad(
            "an animation keeps at least one frame, so its Remove button is disabled while one \
             remains"
                .to_owned(),
        ));
    }
    if index >= held {
        return Err(FormWriteFault::bad(format!(
            "frame {index} is past the end of a list holding {held}"
        )));
    }
    draft.remove_frame(index);
    Ok(())
}

fn move_up(draft: &mut SpriteDraft, index: usize) -> Result<(), FormWriteFault> {
    let held = frame_count(draft);
    if index == 0 || index >= held {
        return Err(FormWriteFault::bad(format!(
            "frame {index} has no slot above it in a list holding {held}"
        )));
    }
    draft.move_frame_up(index);
    Ok(())
}

fn move_down(draft: &mut SpriteDraft, index: usize) -> Result<(), FormWriteFault> {
    let held = frame_count(draft);
    if index + 1 >= held {
        return Err(FormWriteFault::bad(format!(
            "frame {index} has no slot below it in a list holding {held}"
        )));
    }
    draft.move_frame_down(index);
    Ok(())
}

// Every frame control is drawn only while animation is on.
const fn behind_the_gate(draft: &SpriteDraft) -> Result<(), FormWriteFault> {
    if draft.is_animated() {
        Ok(())
    } else {
        Err(FormWriteFault::Gated(ANIMATION_OFF))
    }
}

/// Apply one operation to the animation's frame list.
pub(super) fn apply(draft: &mut SpriteDraft, op: EditorListOpNet) -> Result<(), FormWriteFault> {
    match op {
        EditorListOpNet::Toggle(_) => Err(FormWriteFault::bad(
            "the frame list is authored by adding, removing and reordering rows, so it offers \
             no toggle"
                .to_owned(),
        )),
        EditorListOpNet::SetAt(..) => Err(FormWriteFault::bad(
            "one frame source is rewritten through the `Sprite(Frame(index: n, source: …))` \
             field arm, not through the list"
                .to_owned(),
        )),
        EditorListOpNet::Add => {
            behind_the_gate(draft)?;
            draft.add_frame();
            Ok(())
        }
        EditorListOpNet::Remove(index) => {
            behind_the_gate(draft)?;
            remove(draft, *index)
        }
        EditorListOpNet::MoveUp(index) => {
            behind_the_gate(draft)?;
            move_up(draft, *index)
        }
        EditorListOpNet::MoveDown(index) => {
            behind_the_gate(draft)?;
            move_down(draft, *index)
        }
    }
}
