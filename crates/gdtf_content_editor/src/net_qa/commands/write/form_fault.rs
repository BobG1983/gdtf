//! Why a form write did not reach the draft, shared by the two draft-write commands.

use gdtf_qa_protocol::command::{ArgumentFault, RefusalNote};

use crate::net_qa::wire::EditorModeNet;

/// The note the closed Sprite animation gate is refused with.
pub(in crate::net_qa::commands::write) const ANIMATION_OFF: RefusalNote = RefusalNote::from_static(
    "the Sprite draft's animation gate is off, and the fps, the frames and every frame source \
     are drawn only while it is on",
);

/// The note the Terrain form's emplacement-only controls are refused with.
pub(in crate::net_qa::commands::write) const NOT_AN_EMPLACEMENT: RefusalNote =
    RefusalNote::from_static(
        "the Terrain draft commits its mounted weapon and its entry sides only while its kind is \
         Emplacement, so this write would silently do nothing",
    );

/// The note a foreign arm is refused with, naming the tab that is actually open.
pub(in crate::net_qa::commands::write) fn foreign_arm_note(mode: EditorModeNet) -> RefusalNote {
    RefusalNote::from_owned(format!(
        "that field or list belongs to another form. The {mode:?} tab is the one open, so only \
         its own fields and lists can be written"
    ))
}

/// Why a form write did not reach the draft.
pub(in crate::net_qa::commands::write) enum FormWriteFault {
    /// The arm names a field or list of a form other than the open tab.
    ForeignArm,
    /// A gate the form draws the control behind is closed, so the setter would do nothing.
    Gated(RefusalNote),
    /// A registry the control reads its own options from is absent or empty.
    MissingModel(RefusalNote),
    /// The value, index or operation cannot be written as asked.
    BadArguments(ArgumentFault),
}

impl FormWriteFault {
    /// A bad-arguments fault carrying the one line that says what was wrong.
    pub(in crate::net_qa::commands::write) const fn bad(detail: String) -> Self {
        Self::BadArguments(ArgumentFault::new(detail))
    }
}
