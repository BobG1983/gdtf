//! Queued and in-flight captures, generic over an opaque host payload.

use std::collections::VecDeque;

use bevy::prelude::*;

use super::{frames::FramesLeft, outcome::CaptureOutcome, stem::ShotStem};
use crate::path::CapturePath;

pub(super) enum CaptureDestination {
    Stem(Option<ShotStem>),
    Exact(CapturePath),
}

pub(super) struct CaptureRequest<P> {
    pub(super) destination: CaptureDestination,
    pub(super) payload:     P,
}

pub(super) enum CaptureStage {
    Settling(FramesLeft),
    Capturing(FramesLeft),
}

pub(super) struct InFlightCapture<P> {
    pub(super) path:    CapturePath,
    pub(super) payload: P,
    pub(super) stage:   CaptureStage,
}

/// Captures waiting to run, and captures already running.
#[derive(Resource)]
pub struct CaptureQueue<P: Send + Sync + 'static> {
    queued:    VecDeque<CaptureRequest<P>>,
    in_flight: Vec<InFlightCapture<P>>,
}

impl<P: Send + Sync + 'static> Default for CaptureQueue<P> {
    fn default() -> Self {
        Self {
            queued:    VecDeque::new(),
            in_flight: Vec::new(),
        }
    }
}

impl<P: Send + Sync + 'static> CaptureQueue<P> {
    /// Queue a capture that lands under the shot directory under `stem`.
    pub fn push(&mut self, stem: Option<ShotStem>, payload: P) {
        self.queued.push_back(CaptureRequest {
            destination: CaptureDestination::Stem(stem),
            payload,
        });
    }

    /// Queue a capture that lands at exactly `path`.
    pub fn push_to(&mut self, path: CapturePath, payload: P) {
        self.queued.push_back(CaptureRequest {
            destination: CaptureDestination::Exact(path),
            payload,
        });
    }

    /// Whether nothing is queued and nothing is in flight.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.queued.is_empty() && self.in_flight.is_empty()
    }

    pub(super) fn take_queued(&mut self) -> Vec<CaptureRequest<P>> {
        self.queued.drain(..).collect()
    }

    pub(super) fn take_in_flight(&mut self) -> Vec<InFlightCapture<P>> {
        core::mem::take(&mut self.in_flight)
    }

    pub(super) fn keep(&mut self, capture: InFlightCapture<P>) {
        self.in_flight.push(capture);
    }
}

/// One finished capture and the payload its requester queued.
pub struct CaptureCompletion<P> {
    outcome: CaptureOutcome,
    payload: P,
}

impl<P> CaptureCompletion<P> {
    /// Borrow how the capture finished.
    #[must_use]
    pub const fn outcome(&self) -> &CaptureOutcome {
        &self.outcome
    }

    /// Split into the outcome and the payload.
    #[must_use]
    pub fn into_parts(self) -> (CaptureOutcome, P) {
        (self.outcome, self.payload)
    }
}

/// Finished captures waiting for their host to answer.
#[derive(Resource)]
pub struct CaptureCompletions<P: Send + Sync + 'static>(Vec<CaptureCompletion<P>>);

impl<P: Send + Sync + 'static> Default for CaptureCompletions<P> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<P: Send + Sync + 'static> CaptureCompletions<P> {
    /// Record a finished capture.
    pub fn push(&mut self, outcome: CaptureOutcome, payload: P) {
        self.0.push(CaptureCompletion { outcome, payload });
    }

    /// Whether nothing has finished.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// How many finished captures are waiting.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Walk the finished captures without taking them.
    pub fn iter(&self) -> impl Iterator<Item = &CaptureCompletion<P>> {
        self.0.iter()
    }

    /// Take every finished capture.
    #[must_use]
    pub fn drain(&mut self) -> Vec<CaptureCompletion<P>> {
        core::mem::take(&mut self.0)
    }
}
