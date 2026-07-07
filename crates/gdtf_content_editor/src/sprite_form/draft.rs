//! The SPRITE-mode form's **working model** (GTW-664): the state-scoped [`SpriteDraft`]
//! resource the egui form's controls write and the save reads.
//!
//! The draft holds the sprite's NAME (a text-field buffer — the file stem IS the
//! registry key, the GTW-663 stem-key model) plus the family's own
//! [`SpriteDef`] record, so the edited model IS the loader schema — projecting to the
//! save is a copy, never a parallel schema (the GTW-636 gang-draft precedent: holding
//! the loader type means every authored field round-trips by construction). Every edit
//! flows through a named PURE mutator, so the anchor-clamp invariant (see
//! [`set_anchor`](SpriteDraft::set_anchor)) is owned here and headless-testable.

use bevy::prelude::*;
use gdtf_content_families::sprites::{
    SpriteAnchor, SpriteAnimation, SpriteDef, SpriteFacing, SpriteFacings, SpriteFps,
    SpriteImagePath, SpriteName, SpritePx, SpriteSource,
};

/// The playback rate a freshly enabled animation seeds — a neutral 1 frame/sec starting
/// value the author tunes (the spawn-seed-sentinel shape: no shipped def documents a
/// canonical rate, so the seed is deliberately inert, not a balance claim).
const SEED_FPS: SpriteFps = SpriteFps::new(1.0);

/// The empty def a fresh draft seeds — a [`File`](SpriteSource::File) source with an
/// empty path (the author types the real one) anchored at the sprite's top-left, no
/// facings, no animation. `const` so [`SpriteDraft::new_sprite`] stays `const` (the
/// `ArmorDraft::new_armor` parity).
const fn seed_def() -> SpriteDef {
    SpriteDef {
        source:    SpriteSource::File(SpriteImagePath::new(String::new())),
        anchor:    SpriteAnchor {
            x: SpritePx::new(0),
            y: SpritePx::new(0),
        },
        facings:   None,
        animation: None,
    }
}

/// Whether the Sprite mode's ONE-SHOT open-with-a-sprite seed has run yet (GTW-664).
///
/// The Gang / Armor / Injury modes open with the FIRST member (sorted by name) already
/// loaded; the Sprite mode keeps that parity via a one-shot autoload the shell runs on
/// the first Sprite-mode frame. A closed enum (no-bare-types — a lifecycle phase is a
/// domain value, not a bare `bool`): [`Pending`](AutoloadState::Pending) until the shell
/// has seen a resolved [`SpriteDefRegistry`](gdtf_content_families::sprites::SpriteDefRegistry),
/// then [`Done`](AutoloadState::Done) forever (a "New sprite" press must never be
/// clobbered by a late re-autoload).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AutoloadState {
    /// The one-shot registry seed has not run yet.
    Pending,
    /// The seed ran (or the draft was explicitly loaded / minted) — never re-seed.
    Done,
}

/// The in-progress SPRITE-mode authoring DRAFT — the state-scoped resource the egui
/// form's controls write and the save projects into the loader's `(`[`SpriteName`]`,
/// `[`SpriteDef`]`)` pair (GTW-664 C2/C3).
///
/// A state-scoped [`Resource`] (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` —
/// bevy-traps #1). The name is a bare [`String`] only as the text-field buffer (the
/// documented `GangDraft` `name` exception); it folds into a [`SpriteName`] on
/// projection. The def is the family's own [`SpriteDef`] record (see the module docs).
/// Private fields with named accessors / mutators (no-bare-types rule 5): every field
/// edit goes through a pure mutator so the anchor-clamp invariant holds by construction.
/// NOT `Eq`: the optional [`SpriteAnimation`] carries an `f32` rate.
#[derive(Resource, Clone, PartialEq, Debug)]
pub struct SpriteDraft {
    /// The sprite's NAME buffer — the registry key / file stem the save sanitizes.
    name:     String,
    /// The sprite's editable definition — the loader-schema record itself.
    def:      SpriteDef,
    /// The one-shot open-with-a-sprite seed phase (see [`AutoloadState`]).
    autoload: AutoloadState,
}

impl SpriteDraft {
    /// A fresh draft for a NEW sprite: an empty name, the empty `seed_def` (the author
    /// fills the real source in) — the "New sprite" press. Autoload is `Done`: a
    /// deliberate new sprite must never be clobbered by the one-shot registry seed (the
    /// `ArmorDraft::new_armor` parity).
    #[must_use]
    pub const fn new_sprite() -> Self {
        Self {
            name:     String::new(),
            def:      seed_def(),
            autoload: AutoloadState::Done,
        }
    }

    /// Whether the one-shot open-with-a-sprite seed is still pending — the shell checks
    /// this each Sprite-mode frame and runs the autoload exactly once (idempotent under
    /// the egui multipass re-run: the first pass marks it done).
    #[must_use]
    pub const fn autoload_pending(&self) -> bool {
        matches!(self.autoload, AutoloadState::Pending)
    }

    /// Mark the one-shot seed as done WITHOUT loading anything — the empty-registry
    /// branch (the Gang / Armor modes' "nothing loaded — start empty" parity).
    pub const fn mark_autoloaded(&mut self) {
        self.autoload = AutoloadState::Done;
    }

    /// Load an existing sprite def into the form (the load `ComboBox` / autoload path):
    /// the registry KEY becomes the name buffer and the def is copied in VERBATIM as the
    /// working record — an authored file is the author's truth, so even an out-of-bounds
    /// anchor is preserved as loaded (only interactive [`set_anchor`](SpriteDraft::set_anchor)
    /// / [`set_base_source`](SpriteDraft::set_base_source) writes clamp). Marks the
    /// one-shot seed done.
    pub fn load_sprite(&mut self, name: &SpriteName, def: &SpriteDef) {
        name.as_str().clone_into(&mut self.name);
        self.def = def.clone();
        self.autoload = AutoloadState::Done;
    }

    /// The sprite's current NAME buffer.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the sprite's name (committed from the text field).
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// The sprite's current definition (read-only — the form's render source and the
    /// save projection).
    #[must_use]
    pub const fn def(&self) -> &SpriteDef {
        &self.def
    }

    /// The inclusive `(w, h)` bounds the anchor clamps into, when they are knowable
    /// CHEAPLY AND HEADLESSLY: a [`Sheet`](SpriteSource::Sheet) source's authored rect
    /// IS the sprite's pixel extent, so its `w`/`h` bound the sprite-local anchor. A
    /// [`File`](SpriteSource::File) source's dims live in the image asset (an async
    /// decode the pure model cannot reach), so the model invents NO clamp for it
    /// (GTW-664 C4 — the UI layer additionally ranges the drag by the LOADED image dims
    /// when the preview texture has resolved).
    #[must_use]
    pub const fn anchor_bounds(&self) -> Option<(SpritePx, SpritePx)> {
        match &self.def.source {
            SpriteSource::File(_) => None,
            SpriteSource::Sheet { rect, .. } => Some((rect.w, rect.h)),
        }
    }

    /// Set the sprite's anchor, clamped into [`anchor_bounds`](SpriteDraft::anchor_bounds)
    /// when the bounds are knowable (the anchor is a CONTINUOUS sprite-local position, so
    /// the full `0..=w` / `0..=h` span is legal — `(8, 8)` is the exact center of a
    /// 16×16 tile, `(16, 16)` its bottom-right corner).
    pub fn set_anchor(&mut self, x: SpritePx, y: SpritePx) {
        self.def.anchor = match self.anchor_bounds() {
            Some((w, h)) => SpriteAnchor {
                x: SpritePx::new((*x).min(*w)),
                y: SpritePx::new((*y).min(*h)),
            },
            None => SpriteAnchor { x, y },
        };
    }

    /// Replace the def's BASE source, then re-clamp the anchor into the new source's
    /// bounds — so shrinking a sheet rect can never strand the authored anchor outside
    /// the sprite (the invariant lives here, not in the UI).
    pub fn set_base_source(&mut self, source: SpriteSource) {
        self.def.source = source;
        let SpriteAnchor { x, y } = self.def.anchor;
        self.set_anchor(x, y);
    }

    /// The authored source override for `facing`, or [`None`] when the facing falls back
    /// to the base source.
    #[must_use]
    pub fn facing_override(&self, facing: SpriteFacing) -> Option<&SpriteSource> {
        self.def.facings.as_ref().and_then(|map| map.get(&facing))
    }

    /// Set (or clear, with [`None`]) one facing's source override. An emptied map folds
    /// back to `facings: None` — the authored RON stays minimal (an absent map and an
    /// empty map mean the same thing: every facing draws the base source).
    pub fn set_facing_override(&mut self, facing: SpriteFacing, source: Option<SpriteSource>) {
        let mut entries: Vec<(SpriteFacing, SpriteSource)> = self
            .def
            .facings
            .as_ref()
            .map(|map| map.iter().map(|(k, v)| (*k, v.clone())).collect())
            .unwrap_or_default();
        entries.retain(|(entry_facing, _)| *entry_facing != facing);
        if let Some(source) = source {
            entries.push((facing, source));
        }
        self.def.facings = (!entries.is_empty()).then(|| SpriteFacings::new(entries));
    }

    /// Turn the animation section ON: seeds `Some(animation)` with the base source as
    /// the single starting frame at `SEED_FPS` (the natural first row — the author
    /// adds / retargets frames from there). A no-op when already animated, so the
    /// checkbox write is idempotent under the egui multipass re-run (bevy-traps #8).
    pub fn enable_animation(&mut self) {
        if self.def.animation.is_none() {
            self.def.animation = Some(SpriteAnimation {
                fps:    SEED_FPS,
                frames: vec![self.def.source.clone()],
            });
        }
    }

    /// Turn the animation section OFF — the sprite is a static image again.
    pub fn disable_animation(&mut self) {
        self.def.animation = None;
    }

    /// Set the animation's playback rate. A no-op while the animation is off.
    pub const fn set_fps(&mut self, fps: SpriteFps) {
        if let Some(animation) = &mut self.def.animation {
            animation.fps = fps;
        }
    }

    /// Append a frame: a copy of the LAST frame (the likeliest starting point for the
    /// next cel — the author retargets its rect/path). A no-op while the animation is
    /// off.
    pub fn add_frame(&mut self) {
        let base = self.def.source.clone();
        if let Some(animation) = &mut self.def.animation {
            let next = animation.frames.last().cloned().unwrap_or(base);
            animation.frames.push(next);
        }
    }

    /// Remove the frame at `index`. A no-op at one remaining frame (the way to author
    /// "no frames" is turning the OPTIONAL animation off — the Remove button mirrors the
    /// injury effects-list affordance) or for an out-of-range index.
    pub fn remove_frame(&mut self, index: usize) {
        if let Some(animation) = &mut self.def.animation
            && animation.frames.len() > 1
            && index < animation.frames.len()
        {
            animation.frames.remove(index);
        }
    }

    /// Swap the frame at `index` one slot EARLIER in the ordered sequence. A no-op at
    /// the first frame / out of range / while the animation is off.
    pub fn move_frame_up(&mut self, index: usize) {
        if let Some(animation) = &mut self.def.animation
            && index > 0
            && index < animation.frames.len()
        {
            animation.frames.swap(index, index - 1);
        }
    }

    /// Swap the frame at `index` one slot LATER in the ordered sequence. A no-op at the
    /// last frame / out of range / while the animation is off.
    pub fn move_frame_down(&mut self, index: usize) {
        if let Some(animation) = &mut self.def.animation
            && index + 1 < animation.frames.len()
        {
            animation.frames.swap(index, index + 1);
        }
    }

    /// Replace the frame at `index` with `source` (the frame row's source editor
    /// commit). A no-op for an out-of-range index / while the animation is off.
    pub fn set_frame(&mut self, index: usize, source: SpriteSource) {
        if let Some(animation) = &mut self.def.animation
            && let Some(frame) = animation.frames.get_mut(index)
        {
            *frame = source;
        }
    }
}

impl Default for SpriteDraft {
    /// The `OnEnter(Editing)` seed: an EMPTY draft with the one-shot open-with-a-sprite
    /// autoload still `Pending` — the shell seeds it from the resolved
    /// [`SpriteDefRegistry`](gdtf_content_families::sprites::SpriteDefRegistry) on the
    /// first Sprite-mode frame (or marks it done when no defs are loaded).
    fn default() -> Self {
        Self {
            name:     String::new(),
            def:      seed_def(),
            autoload: AutoloadState::Pending,
        }
    }
}
