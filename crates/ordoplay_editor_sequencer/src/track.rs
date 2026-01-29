// SPDX-License-Identifier: MIT OR Apache-2.0
//! Track definitions for the sequencer.

use crate::keyframe::Keyframe;
use crate::binding::EntityBinding;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a track
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrackId(pub Uuid);

impl TrackId {
    /// Create a new random track ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TrackId {
    fn default() -> Self {
        Self::new()
    }
}

/// Type of track
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackType {
    /// Transform (position, rotation, scale)
    Transform,
    /// Property animation
    Property,
    /// Event triggers
    Event,
    /// Audio playback
    Audio,
    /// Camera settings
    Camera,
    /// Custom track
    Custom,
}

impl TrackType {
    /// Get the display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Transform => "Transform",
            Self::Property => "Property",
            Self::Event => "Event",
            Self::Audio => "Audio",
            Self::Camera => "Camera",
            Self::Custom => "Custom",
        }
    }

    /// Get the track color
    pub fn color(&self) -> [u8; 3] {
        match self {
            Self::Transform => [100, 150, 255],
            Self::Property => [150, 255, 100],
            Self::Event => [255, 200, 100],
            Self::Audio => [200, 100, 255],
            Self::Camera => [255, 100, 150],
            Self::Custom => [150, 150, 150],
        }
    }
}

/// A track in the sequencer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    /// Unique track ID
    pub id: TrackId,
    /// Track name
    pub name: String,
    /// Track type
    pub track_type: TrackType,
    /// Entity binding
    pub binding: Option<EntityBinding>,
    /// Keyframes in this track
    pub keyframes: Vec<Keyframe>,
    /// Whether the track is muted
    pub muted: bool,
    /// Whether the track is locked
    pub locked: bool,
    /// Track color override
    pub color: Option<[u8; 3]>,
}

impl Track {
    /// Create a new track
    pub fn new(name: impl Into<String>, track_type: TrackType) -> Self {
        Self {
            id: TrackId::new(),
            name: name.into(),
            track_type,
            binding: None,
            keyframes: Vec::new(),
            muted: false,
            locked: false,
            color: None,
        }
    }

    /// Add a keyframe
    pub fn add_keyframe(&mut self, keyframe: Keyframe) {
        self.keyframes.push(keyframe);
        self.sort_keyframes();
    }

    /// Remove a keyframe
    pub fn remove_keyframe(&mut self, keyframe_id: crate::keyframe::KeyframeId) {
        self.keyframes.retain(|k| k.id != keyframe_id);
    }

    /// Sort keyframes by time
    fn sort_keyframes(&mut self) {
        self.keyframes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    /// Get keyframe at time (if exists)
    pub fn keyframe_at(&self, time: f32) -> Option<&Keyframe> {
        self.keyframes.iter().find(|k| (k.time - time).abs() < 0.001)
    }

    /// Get the duration (time of last keyframe)
    pub fn duration(&self) -> f32 {
        self.keyframes.last().map(|k| k.time).unwrap_or(0.0)
    }

    /// Find keyframes surrounding a time
    fn find_keyframes(&self, time: f32) -> (Option<&Keyframe>, Option<&Keyframe>) {
        if self.keyframes.is_empty() {
            return (None, None);
        }

        // Find the first keyframe at or after time
        let next_idx = self.keyframes.iter().position(|k| k.time >= time);

        match next_idx {
            None => {
                // Time is past all keyframes
                (self.keyframes.last(), None)
            }
            Some(0) => {
                // Time is before or at first keyframe
                (None, self.keyframes.first())
            }
            Some(idx) => {
                // Between two keyframes
                (Some(&self.keyframes[idx - 1]), Some(&self.keyframes[idx]))
            }
        }
    }

    /// Evaluate the track value at a given time
    pub fn evaluate(&self, time: f32) -> Option<crate::keyframe::KeyframeValue> {
        if self.keyframes.is_empty() {
            return None;
        }

        let (prev, next) = self.find_keyframes(time);

        match (prev, next) {
            (None, None) => None,
            (Some(kf), None) | (None, Some(kf)) => Some(kf.value.clone()),
            (Some(a), Some(b)) => {
                if (b.time - a.time).abs() < 0.0001 {
                    return Some(b.value.clone());
                }
                let t = (time - a.time) / (b.time - a.time);
                a.value.interpolate(&b.value, t, a.interpolation)
            }
        }
    }

    /// Get keyframes in a time range
    pub fn keyframes_in_range(&self, start: f32, end: f32) -> Vec<&Keyframe> {
        self.keyframes
            .iter()
            .filter(|k| k.time >= start && k.time <= end)
            .collect()
    }

    /// Move keyframe to a new time
    pub fn move_keyframe(&mut self, keyframe_id: crate::keyframe::KeyframeId, new_time: f32) {
        if let Some(kf) = self.keyframes.iter_mut().find(|k| k.id == keyframe_id) {
            kf.time = new_time;
        }
        self.sort_keyframes();
    }

    /// Get mutable keyframe by ID
    pub fn keyframe_mut(&mut self, keyframe_id: crate::keyframe::KeyframeId) -> Option<&mut Keyframe> {
        self.keyframes.iter_mut().find(|k| k.id == keyframe_id)
    }

    /// Get keyframe by ID
    pub fn keyframe(&self, keyframe_id: crate::keyframe::KeyframeId) -> Option<&Keyframe> {
        self.keyframes.iter().find(|k| k.id == keyframe_id)
    }

    /// Get the effective color for this track
    pub fn effective_color(&self) -> [u8; 3] {
        self.color.unwrap_or_else(|| self.track_type.color())
    }

    /// Get keyframe count
    pub fn keyframe_count(&self) -> usize {
        self.keyframes.len()
    }

    /// Create a transform keyframe with position, rotation, scale
    /// Note: For full transform animation, use `TransformTrack` which has separate channels
    pub fn create_transform_keyframe(
        &mut self,
        time: f32,
        position: [f32; 3],
        _rotation: [f32; 4],
        _scale: [f32; 3],
    ) {
        // For basic tracks, we only store position as Vec3
        // For full transform animation, use TransformTrack which has separate channels
        self.add_keyframe(Keyframe::new(time, crate::keyframe::KeyframeValue::Vec3(position)));
    }

    /// Check if keyframe exists near time
    pub fn has_keyframe_near(&self, time: f32, threshold: f32) -> bool {
        self.keyframes.iter().any(|k| (k.time - time).abs() < threshold)
    }

    /// Get nearest keyframe to time
    pub fn nearest_keyframe(&self, time: f32) -> Option<&Keyframe> {
        self.keyframes.iter().min_by(|a, b| {
            let da = (a.time - time).abs();
            let db = (b.time - time).abs();
            da.partial_cmp(&db).unwrap()
        })
    }

    /// Insert or update keyframe at time
    pub fn set_keyframe_at(&mut self, time: f32, value: crate::keyframe::KeyframeValue) {
        // Check if keyframe exists near this time
        let threshold = 0.001;
        if let Some(idx) = self.keyframes.iter().position(|k| (k.time - time).abs() < threshold) {
            self.keyframes[idx].value = value;
        } else {
            self.add_keyframe(Keyframe::new(time, value));
        }
    }

    /// Duplicate keyframe to new time
    pub fn duplicate_keyframe(&mut self, keyframe_id: crate::keyframe::KeyframeId, new_time: f32) -> Option<crate::keyframe::KeyframeId> {
        if let Some(source) = self.keyframe(keyframe_id).cloned() {
            let mut new_kf = source;
            new_kf.id = crate::keyframe::KeyframeId::new();
            new_kf.time = new_time;
            let new_id = new_kf.id;
            self.add_keyframe(new_kf);
            Some(new_id)
        } else {
            None
        }
    }

    /// Scale all keyframes by a time factor
    pub fn scale_time(&mut self, factor: f32) {
        for kf in &mut self.keyframes {
            kf.time *= factor;
        }
    }

    /// Offset all keyframes by a time delta
    pub fn offset_time(&mut self, delta: f32) {
        for kf in &mut self.keyframes {
            kf.time = (kf.time + delta).max(0.0);
        }
        self.sort_keyframes();
    }

    /// Reverse all keyframes
    pub fn reverse(&mut self) {
        if self.keyframes.len() < 2 {
            return;
        }
        let duration = self.duration();
        for kf in &mut self.keyframes {
            kf.time = duration - kf.time;
        }
        self.sort_keyframes();
    }

    /// Get all keyframes
    pub fn keyframes(&self) -> &[Keyframe] {
        &self.keyframes
    }
}

/// Transform track with position, rotation, scale channels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformTrack {
    /// Base track data
    pub base: Track,
    /// Position channel keyframes
    pub position: Vec<Keyframe>,
    /// Rotation channel keyframes (quaternion)
    pub rotation: Vec<Keyframe>,
    /// Scale channel keyframes
    pub scale: Vec<Keyframe>,
}

impl TransformTrack {
    /// Create a new transform track
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            base: Track::new(name, TrackType::Transform),
            position: Vec::new(),
            rotation: Vec::new(),
            scale: Vec::new(),
        }
    }

    /// Add a position keyframe
    pub fn add_position(&mut self, time: f32, value: [f32; 3]) {
        let kf = Keyframe::new(time, crate::keyframe::KeyframeValue::Vec3(value));
        self.position.push(kf);
        self.position.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    /// Add a rotation keyframe
    pub fn add_rotation(&mut self, time: f32, value: [f32; 4]) {
        let kf = Keyframe::new(time, crate::keyframe::KeyframeValue::Vec4(value));
        self.rotation.push(kf);
        self.rotation.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    /// Add a scale keyframe
    pub fn add_scale(&mut self, time: f32, value: [f32; 3]) {
        let kf = Keyframe::new(time, crate::keyframe::KeyframeValue::Vec3(value));
        self.scale.push(kf);
        self.scale.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    /// Evaluate position at time
    pub fn evaluate_position(&self, time: f32) -> Option<[f32; 3]> {
        evaluate_channel_vec3(&self.position, time)
    }

    /// Evaluate rotation at time
    pub fn evaluate_rotation(&self, time: f32) -> Option<[f32; 4]> {
        evaluate_channel_vec4(&self.rotation, time)
    }

    /// Evaluate scale at time
    pub fn evaluate_scale(&self, time: f32) -> Option<[f32; 3]> {
        evaluate_channel_vec3(&self.scale, time)
    }
}

/// Audio track with clip references and volume
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTrack {
    /// Base track data
    pub base: Track,
    /// Audio clips on this track
    pub clips: Vec<AudioClip>,
    /// Volume keyframes
    pub volume: Vec<Keyframe>,
    /// Pan keyframes (-1 to 1)
    pub pan: Vec<Keyframe>,
}

/// An audio clip on a track
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioClip {
    /// Clip ID
    pub id: Uuid,
    /// Start time in sequence
    pub start_time: f32,
    /// End time in sequence
    pub end_time: f32,
    /// Asset path to audio file
    pub asset_path: String,
    /// Start offset within the audio file
    pub clip_start: f32,
    /// Clip volume multiplier
    pub volume: f32,
    /// Fade in duration
    pub fade_in: f32,
    /// Fade out duration
    pub fade_out: f32,
}

impl AudioTrack {
    /// Create a new audio track
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            base: Track::new(name, TrackType::Audio),
            clips: Vec::new(),
            volume: Vec::new(),
            pan: Vec::new(),
        }
    }

    /// Add an audio clip
    pub fn add_clip(&mut self, clip: AudioClip) {
        self.clips.push(clip);
        self.clips.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());
    }

    /// Get clips at a specific time
    pub fn clips_at(&self, time: f32) -> Vec<&AudioClip> {
        self.clips.iter()
            .filter(|c| c.start_time <= time && c.end_time >= time)
            .collect()
    }

    /// Evaluate volume at time
    pub fn evaluate_volume(&self, time: f32) -> f32 {
        evaluate_channel_float(&self.volume, time).unwrap_or(1.0)
    }

    /// Evaluate pan at time
    pub fn evaluate_pan(&self, time: f32) -> f32 {
        evaluate_channel_float(&self.pan, time).unwrap_or(0.0)
    }
}

/// Camera track for cinematic cameras
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraTrack {
    /// Base track data
    pub base: Track,
    /// Field of view keyframes (degrees)
    pub fov: Vec<Keyframe>,
    /// Focus distance keyframes (for depth of field)
    pub focus_distance: Vec<Keyframe>,
    /// Aperture keyframes (f-stop)
    pub aperture: Vec<Keyframe>,
    /// Camera cuts (instant transitions)
    pub cuts: Vec<CameraCut>,
}

/// A camera cut (instant transition)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraCut {
    /// Time of the cut
    pub time: f32,
    /// Target camera entity
    pub target_camera: Option<crate::binding::EntityId>,
    /// Blend duration (0 for instant cut)
    pub blend_duration: f32,
    /// Blend curve type
    pub blend_type: CameraBlendType,
}

/// Camera blend type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CameraBlendType {
    /// Hard cut (instant)
    #[default]
    Cut,
    /// Linear blend
    Linear,
    /// Ease in/out
    EaseInOut,
    /// Custom curve
    Custom,
}

impl CameraTrack {
    /// Create a new camera track
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            base: Track::new(name, TrackType::Camera),
            fov: Vec::new(),
            focus_distance: Vec::new(),
            aperture: Vec::new(),
            cuts: Vec::new(),
        }
    }

    /// Add a FOV keyframe
    pub fn add_fov(&mut self, time: f32, value: f32) {
        let kf = Keyframe::new(time, crate::keyframe::KeyframeValue::Float(value));
        self.fov.push(kf);
        self.fov.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    /// Evaluate FOV at time
    pub fn evaluate_fov(&self, time: f32) -> f32 {
        evaluate_channel_float(&self.fov, time).unwrap_or(60.0)
    }

    /// Add a camera cut
    pub fn add_cut(&mut self, cut: CameraCut) {
        self.cuts.push(cut);
        self.cuts.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    /// Get active camera at time
    pub fn active_camera_at(&self, time: f32) -> Option<&CameraCut> {
        self.cuts.iter()
            .rfind(|c| c.time <= time)
    }
}

/// Event track for triggering callbacks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTrack {
    /// Base track data
    pub base: Track,
    /// Event markers
    pub events: Vec<EventMarker>,
}

/// An event marker on the timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMarker {
    /// Marker ID
    pub id: Uuid,
    /// Time of the event
    pub time: f32,
    /// Event name/type
    pub event_name: String,
    /// Additional parameters (JSON-like)
    pub parameters: std::collections::HashMap<String, String>,
}

impl EventTrack {
    /// Create a new event track
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            base: Track::new(name, TrackType::Event),
            events: Vec::new(),
        }
    }

    /// Add an event marker
    pub fn add_event(&mut self, time: f32, name: impl Into<String>) -> Uuid {
        let id = Uuid::new_v4();
        self.events.push(EventMarker {
            id,
            time,
            event_name: name.into(),
            parameters: std::collections::HashMap::new(),
        });
        self.events.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
        id
    }

    /// Get events in time range
    pub fn events_in_range(&self, start: f32, end: f32) -> Vec<&EventMarker> {
        self.events.iter()
            .filter(|e| e.time >= start && e.time <= end)
            .collect()
    }
}

// Helper functions for channel evaluation

fn evaluate_channel_float(keyframes: &[Keyframe], time: f32) -> Option<f32> {
    if keyframes.is_empty() {
        return None;
    }

    let next_idx = keyframes.iter().position(|k| k.time >= time);

    match next_idx {
        None => keyframes.last()?.value.as_float(),
        Some(0) => keyframes.first()?.value.as_float(),
        Some(idx) => {
            let a = &keyframes[idx - 1];
            let b = &keyframes[idx];
            if (b.time - a.time).abs() < 0.0001 {
                return b.value.as_float();
            }
            let t = (time - a.time) / (b.time - a.time);
            let va = a.value.as_float()?;
            let vb = b.value.as_float()?;
            Some(crate::keyframe::Interpolation::lerp(va, vb, t))
        }
    }
}

fn evaluate_channel_vec3(keyframes: &[Keyframe], time: f32) -> Option<[f32; 3]> {
    if keyframes.is_empty() {
        return None;
    }

    let next_idx = keyframes.iter().position(|k| k.time >= time);

    match next_idx {
        None => keyframes.last()?.value.as_vec3(),
        Some(0) => keyframes.first()?.value.as_vec3(),
        Some(idx) => {
            let a = &keyframes[idx - 1];
            let b = &keyframes[idx];
            if (b.time - a.time).abs() < 0.0001 {
                return b.value.as_vec3();
            }
            let t = (time - a.time) / (b.time - a.time);
            let va = a.value.as_vec3()?;
            let vb = b.value.as_vec3()?;
            Some(crate::keyframe::Interpolation::lerp_vec3(va, vb, t))
        }
    }
}

fn evaluate_channel_vec4(keyframes: &[Keyframe], time: f32) -> Option<[f32; 4]> {
    if keyframes.is_empty() {
        return None;
    }

    let next_idx = keyframes.iter().position(|k| k.time >= time);

    match next_idx {
        None => keyframes.last()?.value.as_vec4(),
        Some(0) => keyframes.first()?.value.as_vec4(),
        Some(idx) => {
            let a = &keyframes[idx - 1];
            let b = &keyframes[idx];
            if (b.time - a.time).abs() < 0.0001 {
                return b.value.as_vec4();
            }
            let t = (time - a.time) / (b.time - a.time);
            let va = a.value.as_vec4()?;
            let vb = b.value.as_vec4()?;
            // Use slerp for quaternion interpolation
            Some(crate::keyframe::Interpolation::slerp(va, vb, t))
        }
    }
}
