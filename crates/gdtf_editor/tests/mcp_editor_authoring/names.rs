/// The mode-tab write this session opens its form with.
pub(crate) const EDITOR_SET_MODE: &str = "editor.set_mode";

/// The blank-draft write this session starts its draft from.
pub(crate) const EDITOR_NEW: &str = "editor.new";

/// The single-field write this session authors with.
pub(crate) const EDITOR_SET_FIELD: &str = "editor.set_field";

/// The list-field write this session authors with.
pub(crate) const EDITOR_LIST_OP: &str = "editor.list_op";

/// The draft save this session ends its writes with.
pub(crate) const EDITOR_SAVE: &str = "editor.save";

/// The draft read this session reads its writes back through.
pub(crate) const EDITOR_DRAFT: &str = "editor.draft";

/// The newest-save read this session reads its save back through.
pub(crate) const EDITOR_LAST_SAVE: &str = "editor.last_save";
