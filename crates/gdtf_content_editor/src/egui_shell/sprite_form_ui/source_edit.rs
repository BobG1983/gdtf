//! The SPRITE form's **reusable source editor** (GTW-664 C2) — one widget stack for
//! every place a [`SpriteSource`] is authored (the base source, a facing override, an
//! animation frame): the File/Sheet kind combo, the VALIDATED image-path field, and the
//! sheet-rect drags.
//!
//! Clone-edit-return shape: the caller hands the CURRENT source in, the editor edits a
//! clone, and a change comes back as `Some(new)` for the caller to route through the
//! right [`SpriteDraft`](crate::sprite_form::SpriteDraft) mutator — so every commit
//! flows through the model's named mutators (the anchor re-clamp invariant lives there)
//! and the widget itself stays multipass-idempotent (bevy-traps #8: an unchanged pass
//! returns [`None`] and writes nothing).

use bevy_egui::egui;
use gdtf_battle_presenter::SheetRole;
use gdtf_content_families::sprites::{SpriteImagePath, SpritePx, SpriteRect, SpriteSource};

use super::cache::SpritePreviewCache;

/// The source-kind segment of the editor — which [`SpriteSource`] variant the row
/// authors. A named closed enum (no-bare-types) mirroring the schema's two variants.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SourceKindChoice {
    /// A standalone image file is the whole sprite.
    File,
    /// A rect region cut out of a shared sprite sheet.
    Sheet,
}

impl SourceKindChoice {
    /// The kind of an existing source.
    const fn of(source: &SpriteSource) -> Self {
        match source {
            SpriteSource::File(_) => Self::File,
            SpriteSource::Sheet { .. } => Self::Sheet,
        }
    }

    /// The combo label.
    const fn label(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Sheet => "Sheet",
        }
    }
}

/// Draw one source editor over `source`; returns `Some(edited)` when any control
/// changed this pass (see the [module doc](self) for the clone-edit-return shape).
/// `id_salt` disambiguates the combos across the many rows one panel stacks (the
/// injury-form per-row salt lesson).
pub(super) fn source_editor(
    ui: &mut egui::Ui,
    id_salt: &str,
    source: &SpriteSource,
    cache: &mut SpritePreviewCache,
) -> Option<SpriteSource> {
    let mut edited = source.clone();
    let mut changed = false;

    let current = SourceKindChoice::of(&edited);
    let mut kind = current;
    ui.horizontal(|ui| {
        ui.label("Source");
        egui::ComboBox::from_id_salt(format!("{id_salt}_source_kind"))
            .selected_text(kind.label())
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut kind,
                    SourceKindChoice::File,
                    SourceKindChoice::File.label(),
                );
                ui.selectable_value(
                    &mut kind,
                    SourceKindChoice::Sheet,
                    SourceKindChoice::Sheet.label(),
                );
            });
    });
    if kind != current {
        // Kind switch = template conversion (the injury effects-list precedent: only a
        // REAL kind change replaces the payload; an opened-then-closed combo never wipes
        // a tuned one). The path survives the switch both ways.
        edited = converted(edited, kind);
        changed = true;
    }

    match &mut edited {
        SpriteSource::File(path) => {
            changed |= path_field(ui, id_salt, "Image path", path, cache);
        }
        SpriteSource::Sheet { sheet, rect } => {
            changed |= path_field(ui, id_salt, "Sheet path", sheet, cache);
            changed |= rect_fields(ui, rect);
        }
    }

    changed.then_some(edited)
}

/// Convert a source to the newly picked kind, keeping the authored image path: File →
/// Sheet seeds the rect at the sheet origin spanning one presenter terrain cell
/// ([`SheetRole::Terrain::tile_px`] — the same 16px cell every seeded def cuts, derived
/// not re-spelled); Sheet → File keeps the sheet image as the standalone file.
fn converted(source: SpriteSource, kind: SourceKindChoice) -> SpriteSource {
    let tile = SheetRole::Terrain.tile_px();
    match (source, kind) {
        (SpriteSource::File(path), SourceKindChoice::Sheet) => SpriteSource::Sheet {
            sheet: path,
            rect:  SpriteRect {
                x: SpritePx::new(0),
                y: SpritePx::new(0),
                w: SpritePx::new(tile),
                h: SpritePx::new(tile),
            },
        },
        (SpriteSource::Sheet { sheet, .. }, SourceKindChoice::File) => SpriteSource::File(sheet),
        (source, _) => source,
    }
}

/// One VALIDATED image-path text field (GTW-664 C2's ruled shape — a validated text
/// path, not a file browser): a single-line edit plus a live existence verdict against
/// the workspace assets root (memoized in the cache — one stat per distinct string).
fn path_field(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    path: &mut SpriteImagePath,
    cache: &mut SpritePreviewCache,
) -> bool {
    let mut buffer = path.as_str().to_owned();
    ui.label(label);
    let changed = ui
        .add(egui::TextEdit::singleline(&mut buffer).id_salt(format!("{id_salt}_{label}")))
        .changed();
    if changed {
        *path = SpriteImagePath::new(buffer.clone());
    }
    if buffer.trim().is_empty() {
        ui.colored_label(
            egui::Color32::YELLOW,
            "enter an assets/-relative image path",
        );
    } else if cache.path_exists(path) {
        ui.colored_label(egui::Color32::LIGHT_GREEN, "found under assets/");
    } else {
        ui.colored_label(egui::Color32::LIGHT_RED, "MISSING under assets/");
    }
    changed
}

/// The sheet-rect drags — `x`/`y` (sheet position) and `w`/`h` (cut extent), each a
/// [`SpritePx`] over the full `u32` type range (the schema documents no sheet bound —
/// no invented clamp, the injury payload-drag precedent).
fn rect_fields(ui: &mut egui::Ui, rect: &mut SpriteRect) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= px_drag(ui, "x", &mut rect.x);
        changed |= px_drag(ui, "y", &mut rect.y);
        changed |= px_drag(ui, "w", &mut rect.w);
        changed |= px_drag(ui, "h", &mut rect.h);
    });
    changed
}

/// One labeled [`SpritePx`] drag; commits through the newtype constructor.
fn px_drag(ui: &mut egui::Ui, label: &str, px: &mut SpritePx) -> bool {
    let mut value = **px;
    ui.label(label);
    let changed = ui.add(egui::DragValue::new(&mut value)).changed();
    if changed {
        *px = SpritePx::new(value);
    }
    changed
}
