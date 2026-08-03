//! the loader type means every authored field round-trips by construction). Every edit
use bevy::prelude::*;
use gdtf_content_families::sprites::{
    SpriteAnchor, SpriteAnimation, SpriteDef, SpriteFacing, SpriteFacings, SpriteFps,
    SpriteImagePath, SpriteName, SpritePx, SpriteSource,
};

const SEED_FPS: SpriteFps = SpriteFps::new(1.0);

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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AutoloadState {
        Pending,
        Done,
}

#[derive(Resource, Clone, PartialEq, Debug)]
pub struct SpriteDraft {
        name:     String,
        def:      SpriteDef,
        autoload: AutoloadState,
}

impl SpriteDraft {
                    #[must_use]
    pub const fn new_sprite() -> Self {
        Self {
            name:     String::new(),
            def:      seed_def(),
            autoload: AutoloadState::Done,
        }
    }

                #[must_use]
    pub const fn autoload_pending(&self) -> bool {
        matches!(self.autoload, AutoloadState::Pending)
    }

            pub const fn mark_autoloaded(&mut self) {
        self.autoload = AutoloadState::Done;
    }

                            pub fn load_sprite(&mut self, name: &SpriteName, def: &SpriteDef) {
        name.as_str().clone_into(&mut self.name);
        self.def = def.clone();
        self.autoload = AutoloadState::Done;
    }

        #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

        pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

            #[must_use]
    pub const fn def(&self) -> &SpriteDef {
        &self.def
    }

                                #[must_use]
    pub const fn anchor_bounds(&self) -> Option<(SpritePx, SpritePx)> {
        match &self.def.source {
            SpriteSource::File(_) => None,
            SpriteSource::Sheet { rect, .. } => Some((rect.w, rect.h)),
        }
    }

                    pub fn set_anchor(&mut self, x: SpritePx, y: SpritePx) {
        self.def.anchor = match self.anchor_bounds() {
            Some((w, h)) => SpriteAnchor {
                x: SpritePx::new((*x).min(*w)),
                y: SpritePx::new((*y).min(*h)),
            },
            None => SpriteAnchor { x, y },
        };
    }

                pub fn set_base_source(&mut self, source: SpriteSource) {
        self.def.source = source;
        let SpriteAnchor { x, y } = self.def.anchor;
        self.set_anchor(x, y);
    }

            #[must_use]
    pub fn facing_override(&self, facing: SpriteFacing) -> Option<&SpriteSource> {
        self.def.facings.as_ref().and_then(|map| map.get(&facing))
    }

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

                    pub fn enable_animation(&mut self) {
        if self.def.animation.is_none() {
            self.def.animation = Some(SpriteAnimation {
                fps:    SEED_FPS,
                frames: vec![self.def.source.clone()],
            });
        }
    }

        pub fn disable_animation(&mut self) {
        self.def.animation = None;
    }

        pub const fn set_fps(&mut self, fps: SpriteFps) {
        if let Some(animation) = &mut self.def.animation {
            animation.fps = fps;
        }
    }

                pub fn add_frame(&mut self) {
        let base = self.def.source.clone();
        if let Some(animation) = &mut self.def.animation {
            let next = animation.frames.last().cloned().unwrap_or(base);
            animation.frames.push(next);
        }
    }

                pub fn remove_frame(&mut self, index: usize) {
        if let Some(animation) = &mut self.def.animation
            && animation.frames.len() > 1
            && index < animation.frames.len()
        {
            animation.frames.remove(index);
        }
    }

            pub fn move_frame_up(&mut self, index: usize) {
        if let Some(animation) = &mut self.def.animation
            && index > 0
            && index < animation.frames.len()
        {
            animation.frames.swap(index, index - 1);
        }
    }

            pub fn move_frame_down(&mut self, index: usize) {
        if let Some(animation) = &mut self.def.animation
            && index + 1 < animation.frames.len()
        {
            animation.frames.swap(index, index + 1);
        }
    }

            pub fn set_frame(&mut self, index: usize, source: SpriteSource) {
        if let Some(animation) = &mut self.def.animation
            && let Some(frame) = animation.frames.get_mut(index)
        {
            *frame = source;
        }
    }
}

impl Default for SpriteDraft {
                    fn default() -> Self {
        Self {
            name:     String::new(),
            def:      seed_def(),
            autoload: AutoloadState::Pending,
        }
    }
}
