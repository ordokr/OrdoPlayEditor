// SPDX-License-Identifier: MIT OR Apache-2.0
//! Sequence containing multiple tracks.

use crate::track::{Track, TrackId};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a sequence
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SequenceId(pub Uuid);

impl SequenceId {
    /// Create a new random sequence ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SequenceId {
    fn default() -> Self {
        Self::new()
    }
}

/// Playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackState {
    /// Stopped
    #[default]
    Stopped,
    /// Playing forward
    Playing,
    /// Paused
    Paused,
    /// Playing in reverse
    Reverse,
}

/// A sequence of tracks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sequence {
    /// Unique sequence ID
    pub id: SequenceId,
    /// Sequence name
    pub name: String,
    /// Tracks in this sequence
    tracks: IndexMap<TrackId, Track>,
    /// Sequence duration (can be longer than tracks)
    pub duration: f32,
    /// Frame rate
    pub frame_rate: f32,
    /// Whether the sequence loops
    pub looping: bool,
}

impl Sequence {
    /// Create a new sequence
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: SequenceId::new(),
            name: name.into(),
            tracks: IndexMap::new(),
            duration: 10.0,
            frame_rate: 30.0,
            looping: false,
        }
    }

    /// Add a track
    pub fn add_track(&mut self, track: Track) -> TrackId {
        let id = track.id;
        self.tracks.insert(id, track);
        id
    }

    /// Remove a track
    pub fn remove_track(&mut self, track_id: TrackId) -> Option<Track> {
        self.tracks.swap_remove(&track_id)
    }

    /// Get a track
    pub fn track(&self, track_id: TrackId) -> Option<&Track> {
        self.tracks.get(&track_id)
    }

    /// Get a mutable track
    pub fn track_mut(&mut self, track_id: TrackId) -> Option<&mut Track> {
        self.tracks.get_mut(&track_id)
    }

    /// Get all tracks
    pub fn tracks(&self) -> impl Iterator<Item = &Track> {
        self.tracks.values()
    }

    /// Get track count
    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    /// Get the duration based on track content
    pub fn content_duration(&self) -> f32 {
        self.tracks.values()
            .map(|t| t.duration())
            .fold(0.0, f32::max)
    }

    /// Convert time to frame number
    pub fn time_to_frame(&self, time: f32) -> u32 {
        (time * self.frame_rate) as u32
    }

    /// Convert frame number to time
    pub fn frame_to_time(&self, frame: u32) -> f32 {
        frame as f32 / self.frame_rate
    }
}

impl Default for Sequence {
    fn default() -> Self {
        Self::new("Untitled Sequence")
    }
}

/// Playback controller for sequences
pub struct PlaybackController {
    /// Current playback time
    pub time: f32,
    /// Playback state
    pub state: PlaybackState,
    /// Playback speed multiplier
    pub speed: f32,
    /// Loop start point (for loop range)
    pub loop_start: Option<f32>,
    /// Loop end point (for loop range)
    pub loop_end: Option<f32>,
    /// Events triggered this frame
    pending_events: Vec<(TrackId, String)>,
}

impl PlaybackController {
    /// Create a new playback controller
    pub fn new() -> Self {
        Self {
            time: 0.0,
            state: PlaybackState::Stopped,
            speed: 1.0,
            loop_start: None,
            loop_end: None,
            pending_events: Vec::new(),
        }
    }

    /// Update playback with delta time
    pub fn update(&mut self, delta_time: f32, sequence: &Sequence) {
        match self.state {
            PlaybackState::Playing => {
                self.time += delta_time * self.speed;
                self.check_bounds(sequence);
            }
            PlaybackState::Reverse => {
                self.time -= delta_time * self.speed;
                self.check_bounds_reverse(sequence);
            }
            PlaybackState::Paused | PlaybackState::Stopped => {}
        }

        // Collect event triggers
        self.collect_events(sequence);
    }

    /// Check and handle end of sequence
    fn check_bounds(&mut self, sequence: &Sequence) {
        let end_time = self.loop_end.unwrap_or(sequence.duration);

        if self.time >= end_time {
            if sequence.looping || self.loop_end.is_some() {
                let start = self.loop_start.unwrap_or(0.0);
                self.time = start + (self.time - end_time);
            } else {
                self.time = end_time;
                self.state = PlaybackState::Stopped;
            }
        }
    }

    /// Check and handle reverse playback bounds
    fn check_bounds_reverse(&mut self, sequence: &Sequence) {
        let start_time = self.loop_start.unwrap_or(0.0);

        if self.time <= start_time {
            if sequence.looping || self.loop_start.is_some() {
                let end = self.loop_end.unwrap_or(sequence.duration);
                self.time = end - (start_time - self.time);
            } else {
                self.time = start_time;
                self.state = PlaybackState::Stopped;
            }
        }
    }

    /// Collect events that should trigger at current time
    fn collect_events(&mut self, sequence: &Sequence) {
        self.pending_events.clear();

        for track in sequence.tracks() {
            if track.muted || track.track_type != crate::track::TrackType::Event {
                continue;
            }

            // Check for events at current time (within a small window)
            for keyframe in track.keyframes_in_range(self.time - 0.016, self.time) {
                if let crate::keyframe::KeyframeValue::Event(event_name) = &keyframe.value {
                    self.pending_events.push((track.id, event_name.clone()));
                }
            }
        }
    }

    /// Get pending events and clear them
    pub fn take_events(&mut self) -> Vec<(TrackId, String)> {
        std::mem::take(&mut self.pending_events)
    }

    /// Play from current position
    pub fn play(&mut self) {
        self.state = PlaybackState::Playing;
    }

    /// Pause playback
    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing || self.state == PlaybackState::Reverse {
            self.state = PlaybackState::Paused;
        }
    }

    /// Stop and reset to beginning
    pub fn stop(&mut self) {
        self.state = PlaybackState::Stopped;
        self.time = self.loop_start.unwrap_or(0.0);
    }

    /// Toggle play/pause
    pub fn toggle_playback(&mut self) {
        match self.state {
            PlaybackState::Playing | PlaybackState::Reverse => self.pause(),
            PlaybackState::Paused | PlaybackState::Stopped => self.play(),
        }
    }

    /// Play in reverse
    pub fn play_reverse(&mut self) {
        self.state = PlaybackState::Reverse;
    }

    /// Seek to specific time
    pub fn seek(&mut self, time: f32) {
        self.time = time.max(0.0);
    }

    /// Set loop range
    pub fn set_loop_range(&mut self, start: f32, end: f32) {
        self.loop_start = Some(start);
        self.loop_end = Some(end);
    }

    /// Clear loop range
    pub fn clear_loop_range(&mut self) {
        self.loop_start = None;
        self.loop_end = None;
    }

    /// Is currently playing (forward or reverse)
    pub fn is_playing(&self) -> bool {
        matches!(self.state, PlaybackState::Playing | PlaybackState::Reverse)
    }

    /// Get current frame number for a sequence
    pub fn current_frame(&self, sequence: &Sequence) -> u32 {
        sequence.time_to_frame(self.time)
    }

    /// Evaluate all tracks at current time
    pub fn evaluate_all(&self, sequence: &Sequence) -> Vec<(TrackId, crate::keyframe::KeyframeValue)> {
        let mut results = Vec::new();

        for track in sequence.tracks() {
            if track.muted {
                continue;
            }

            if let Some(value) = track.evaluate(self.time) {
                results.push((track.id, value));
            }
        }

        results
    }
}

impl Default for PlaybackController {
    fn default() -> Self {
        Self::new()
    }
}

/// Marker for loop region
#[derive(Debug, Clone, Copy)]
pub struct LoopMarker {
    /// Start time
    pub start: f32,
    /// End time
    pub end: f32,
}

/// Time marker in the sequence
#[derive(Debug, Clone)]
pub struct TimeMarker {
    /// Time position
    pub time: f32,
    /// Marker name
    pub name: String,
    /// Marker color
    pub color: [u8; 3],
}
