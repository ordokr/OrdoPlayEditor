// SPDX-License-Identifier: MIT OR Apache-2.0
//! Audio system for the editor.
//!
//! This module provides:
//! - Audio engine using rodio (when "audio" feature is enabled)
//! - Audio source playback management
//! - 3D spatial audio
//! - Audio mixer with volume controls
//!
//! When the "audio" feature is not enabled, a stub implementation is provided
//! that logs warnings but does not play audio.


#[cfg(feature = "audio")]
use crate::components::Component;
use crate::components::AudioSourceComponent;
use crate::state::{EntityId, SceneData, Transform};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Audio listener position (typically attached to camera)
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone, Copy, Default)]
pub struct AudioListener {
    pub position: [f32; 3],
    pub forward: [f32; 3],
    pub up: [f32; 3],
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl AudioListener {
    pub fn new() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            forward: [0.0, 0.0, -1.0],
            up: [0.0, 1.0, 0.0],
        }
    }

    pub fn from_transform(transform: &Transform) -> Self {
        // Convert rotation to forward vector (simplified, assuming Y-up)
        let yaw = transform.rotation[1].to_radians();
        let pitch = transform.rotation[0].to_radians();

        let forward = [
            yaw.sin() * pitch.cos(),
            -pitch.sin(),
            -yaw.cos() * pitch.cos(),
        ];

        Self {
            position: transform.position,
            forward,
            up: [0.0, 1.0, 0.0],
        }
    }
}

/// Mixer channel for volume control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioChannel {
    Master,
    Music,
    Sfx,
    Voice,
    Ambient,
}

/// Audio mixer settings
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone)]
pub struct AudioMixer {
    /// Channel volumes (0.0 to 1.0)
    pub volumes: HashMap<AudioChannel, f32>,
    /// Muted channels
    pub muted: HashMap<AudioChannel, bool>,
}

impl Default for AudioMixer {
    fn default() -> Self {
        let mut volumes = HashMap::new();
        volumes.insert(AudioChannel::Master, 1.0);
        volumes.insert(AudioChannel::Music, 1.0);
        volumes.insert(AudioChannel::Sfx, 1.0);
        volumes.insert(AudioChannel::Voice, 1.0);
        volumes.insert(AudioChannel::Ambient, 1.0);

        let muted = HashMap::new();

        Self { volumes, muted }
    }
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl AudioMixer {
    pub fn get_volume(&self, channel: AudioChannel) -> f32 {
        let master = *self.volumes.get(&AudioChannel::Master).unwrap_or(&1.0);
        let channel_vol = *self.volumes.get(&channel).unwrap_or(&1.0);
        let is_muted = *self.muted.get(&channel).unwrap_or(&false);

        if is_muted || *self.muted.get(&AudioChannel::Master).unwrap_or(&false) {
            0.0
        } else {
            master * channel_vol
        }
    }

    pub fn set_volume(&mut self, channel: AudioChannel, volume: f32) {
        self.volumes.insert(channel, volume.clamp(0.0, 1.0));
    }

    pub fn set_muted(&mut self, channel: AudioChannel, muted: bool) {
        self.muted.insert(channel, muted);
    }

    pub fn toggle_mute(&mut self, channel: AudioChannel) {
        let current = *self.muted.get(&channel).unwrap_or(&false);
        self.muted.insert(channel, !current);
    }
}

/// Audio listener component for entities
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct AudioListenerComponent {
    /// Whether this is the active listener
    pub active: bool,
}

impl Default for AudioListenerComponent {
    fn default() -> Self {
        Self { active: true }
    }
}

// ============================================================================
// Audio Engine Implementation (with rodio)
// ============================================================================

#[cfg(feature = "audio")]
mod engine {
    use super::*;
    use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
    use std::fs::File;
    use std::io::BufReader;

    /// Playing audio source instance
    struct PlayingSource {
        entity_id: EntityId,
        sink: Sink,
        channel: AudioChannel,
        base_volume: f32,
        is_spatial: bool,
        loop_audio: bool,
        clip_path: PathBuf,
    }

    /// Audio engine managing all audio playback
    pub struct AudioEngine {
        /// Output stream (must be kept alive)
        _stream: Option<OutputStream>,
        /// Stream handle for creating sinks
        stream_handle: Option<OutputStreamHandle>,
        /// Currently playing sources
        playing_sources: HashMap<EntityId, PlayingSource>,
        /// Audio mixer
        pub mixer: AudioMixer,
        /// Audio listener
        pub listener: AudioListener,
        /// Assets root path
        assets_path: Option<PathBuf>,
        /// Whether audio is initialized
        initialized: bool,
    }

    impl Default for AudioEngine {
        fn default() -> Self {
            Self::new()
        }
    }

    impl AudioEngine {
        pub fn new() -> Self {
            // Try to initialize audio output
            let (stream, stream_handle, initialized) = match OutputStream::try_default() {
                Ok((stream, handle)) => {
                    tracing::info!("Audio engine initialized successfully");
                    (Some(stream), Some(handle), true)
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize audio: {}. Audio will be disabled.", e);
                    (None, None, false)
                }
            };

            Self {
                _stream: stream,
                stream_handle,
                playing_sources: HashMap::new(),
                mixer: AudioMixer::default(),
                listener: AudioListener::new(),
                assets_path: None,
                initialized,
            }
        }

        /// Check if audio is available
        pub fn is_available(&self) -> bool {
            self.initialized
        }

        /// Set the assets root path
        pub fn set_assets_path(&mut self, path: PathBuf) {
            self.assets_path = Some(path);
        }

        /// Resolve a clip path to an absolute path
        fn resolve_clip_path(&self, clip: &str) -> Option<PathBuf> {
            if clip.is_empty() {
                return None;
            }

            let clip_path = PathBuf::from(clip);

            // If it's an absolute path, use it directly
            if clip_path.is_absolute() && clip_path.exists() {
                return Some(clip_path);
            }

            // Try relative to assets path
            if let Some(assets) = &self.assets_path {
                let full_path = assets.join(&clip_path);
                if full_path.exists() {
                    return Some(full_path);
                }
            }

            // Try as-is
            if clip_path.exists() {
                return Some(clip_path);
            }

            None
        }

        /// Initialize audio sources from scene
        pub fn initialize_from_scene(&mut self, scene: &SceneData) {
            self.stop_all();

            if !self.initialized {
                return;
            }

            // Find and start all audio sources with play_on_awake
            for (entity_id, entity_data) in &scene.entities {
                for component in &entity_data.components {
                    if let Component::AudioSource(audio) = component {
                        if audio.play_on_awake && !audio.clip.is_empty() {
                            self.play_source(*entity_id, audio, &entity_data.transform);
                        }
                    }
                }
            }

            tracing::info!("Audio initialized: {} sources playing", self.playing_sources.len());
        }

        /// Play an audio source
        pub fn play_source(&mut self, entity_id: EntityId, audio: &AudioSourceComponent, transform: &Transform) {
            if !self.initialized {
                return;
            }

            let stream_handle = match &self.stream_handle {
                Some(h) => h,
                None => return,
            };

            // Stop any existing playback for this entity
            self.stop_source(entity_id);

            // Resolve the clip path
            let clip_path = match self.resolve_clip_path(&audio.clip) {
                Some(p) => p,
                None => {
                    tracing::warn!("Audio clip not found: {}", audio.clip);
                    return;
                }
            };

            // Try to load and play the audio file
            let file = match File::open(&clip_path) {
                Ok(f) => f,
                Err(e) => {
                    tracing::warn!("Failed to open audio file {:?}: {}", clip_path, e);
                    return;
                }
            };

            let source = match Decoder::new(BufReader::new(file)) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("Failed to decode audio file {:?}: {}", clip_path, e);
                    return;
                }
            };

            // Create sink
            let sink: Sink = match Sink::try_new(stream_handle) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("Failed to create audio sink: {}", e);
                    return;
                }
            };

            // Calculate volume with spatial attenuation if needed
            let base_volume = audio.volume * audio.pitch.abs();
            let channel = AudioChannel::Sfx; // Default to SFX channel
            let spatial_volume = if audio.spatial {
                self.calculate_spatial_volume(transform, audio.min_distance, audio.max_distance)
            } else {
                1.0
            };

            let final_volume = base_volume * spatial_volume * self.mixer.get_volume(channel);

            sink.set_volume(final_volume);
            sink.set_speed(audio.pitch.abs());

            // Append source
            sink.append(source);

            // Store the playing source
            self.playing_sources.insert(entity_id, PlayingSource {
                entity_id,
                sink,
                channel,
                base_volume,
                is_spatial: audio.spatial,
                loop_audio: audio.loop_audio,
                clip_path,
            });

            tracing::debug!("Started playing audio for entity {:?}", entity_id);
        }

        /// Stop an audio source
        pub fn stop_source(&mut self, entity_id: EntityId) {
            if let Some(source) = self.playing_sources.remove(&entity_id) {
                source.sink.stop();
            }
        }

        /// Stop all audio
        pub fn stop_all(&mut self) {
            for (_, source) in self.playing_sources.drain() {
                source.sink.stop();
            }
        }

        /// Pause an audio source
        pub fn pause_source(&mut self, entity_id: EntityId) {
            if let Some(source) = self.playing_sources.get(&entity_id) {
                source.sink.pause();
            }
        }

        /// Resume an audio source
        pub fn resume_source(&mut self, entity_id: EntityId) {
            if let Some(source) = self.playing_sources.get(&entity_id) {
                source.sink.play();
            }
        }

        /// Pause all audio
        pub fn pause_all(&mut self) {
            for source in self.playing_sources.values() {
                source.sink.pause();
            }
        }

        /// Resume all audio
        pub fn resume_all(&mut self) {
            for source in self.playing_sources.values() {
                source.sink.play();
            }
        }

        /// Update audio system (call each frame)
        pub fn update(&mut self, scene: &SceneData) {
            if !self.initialized {
                return;
            }

            // Remove finished sources
            self.playing_sources.retain(|_entity_id, source| {
                if source.sink.empty() {
                    if source.loop_audio {
                        // Reload and play again for looping
                        if let Ok(file) = File::open(&source.clip_path) {
                            if let Ok(new_source) = Decoder::new(BufReader::new(file)) {
                                source.sink.append(new_source);
                                return true;
                            }
                        }
                    }
                    false
                } else {
                    true
                }
            });

            // Update spatial audio volumes
            for (entity_id, source) in &mut self.playing_sources {
                if source.is_spatial {
                    if let Some(entity) = scene.get(entity_id) {
                        // Find the audio source component to get min/max distance
                        let (min_dist, max_dist) = entity.components.iter()
                            .find_map(|c| {
                                if let Component::AudioSource(audio) = c {
                                    Some((audio.min_distance, audio.max_distance))
                                } else {
                                    None
                                }
                            })
                            .unwrap_or((1.0, 50.0));

                        let spatial_volume = self.calculate_spatial_volume(
                            &entity.transform,
                            min_dist,
                            max_dist,
                        );

                        let final_volume = source.base_volume * spatial_volume * self.mixer.get_volume(source.channel);
                        source.sink.set_volume(final_volume);
                    }
                }
            }
        }

        /// Calculate spatial audio volume based on distance
        fn calculate_spatial_volume(&self, source_transform: &Transform, min_distance: f32, max_distance: f32) -> f32 {
            let dx = source_transform.position[0] - self.listener.position[0];
            let dy = source_transform.position[1] - self.listener.position[1];
            let dz = source_transform.position[2] - self.listener.position[2];
            let distance = (dx * dx + dy * dy + dz * dz).sqrt();

            if distance <= min_distance {
                1.0
            } else if distance >= max_distance {
                0.0
            } else {
                // Linear falloff between min and max distance
                1.0 - (distance - min_distance) / (max_distance - min_distance)
            }
        }

        /// Set the audio listener position
        pub fn set_listener(&mut self, listener: AudioListener) {
            self.listener = listener;
        }

        /// Set listener from a transform (usually camera)
        pub fn set_listener_transform(&mut self, transform: &Transform) {
            self.listener = AudioListener::from_transform(transform);
        }

        /// Check if an entity's audio is playing
        pub fn is_playing(&self, entity_id: EntityId) -> bool {
            self.playing_sources.get(&entity_id)
                .map(|s| !s.sink.empty() && !s.sink.is_paused())
                .unwrap_or(false)
        }

        /// Check if an entity's audio is paused
        pub fn is_paused(&self, entity_id: EntityId) -> bool {
            self.playing_sources.get(&entity_id)
                .map(|s| s.sink.is_paused())
                .unwrap_or(false)
        }

        /// Get the number of currently playing sources
        pub fn playing_count(&self) -> usize {
            self.playing_sources.len()
        }

        /// Preview an audio clip (for asset browser)
        pub fn preview_clip(&mut self, clip_path: &Path) -> bool {
            if !self.initialized {
                return false;
            }

            let stream_handle = match &self.stream_handle {
                Some(h) => h,
                None => return false,
            };

            let file = match File::open(clip_path) {
                Ok(f) => f,
                Err(_) => return false,
            };

            let source = match Decoder::new(BufReader::new(file)) {
                Ok(s) => s,
                Err(_) => return false,
            };

            let sink: Sink = match Sink::try_new(stream_handle) {
                Ok(s) => s,
                Err(_) => return false,
            };

            sink.set_volume(self.mixer.get_volume(AudioChannel::Master));
            sink.append(source);
            sink.detach(); // Let it play independently

            true
        }
    }
}

// ============================================================================
// Stub Audio Engine Implementation (without rodio)
// ============================================================================

#[cfg(not(feature = "audio"))]
mod engine {
    use super::*;

    /// Audio engine stub (no audio support)
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub struct AudioEngine {
        /// Audio mixer
        pub mixer: AudioMixer,
        /// Audio listener
        pub listener: AudioListener,
        /// Assets root path
        assets_path: Option<PathBuf>,
        /// Log warning once
        warned: bool,
    }

    impl Default for AudioEngine {
        fn default() -> Self {
            Self::new()
        }
    }

    #[allow(dead_code)] // Intentionally kept for API completeness
    impl AudioEngine {
        pub fn new() -> Self {
            tracing::info!("Audio engine: stub implementation (audio feature not enabled)");
            Self {
                mixer: AudioMixer::default(),
                listener: AudioListener::new(),
                assets_path: None,
                warned: false,
            }
        }

        fn warn_once(&mut self) {
            if !self.warned {
                tracing::warn!("Audio playback not available: compile with --features audio");
                self.warned = true;
            }
        }

        /// Check if audio is available
        pub fn is_available(&self) -> bool {
            false
        }

        /// Set the assets root path
        pub fn set_assets_path(&mut self, path: PathBuf) {
            self.assets_path = Some(path);
        }

        /// Initialize audio sources from scene
        pub fn initialize_from_scene(&mut self, _scene: &SceneData) {
            self.warn_once();
        }

        /// Play an audio source
        pub fn play_source(&mut self, _entity_id: EntityId, _audio: &AudioSourceComponent, _transform: &Transform) {
            self.warn_once();
        }

        /// Stop an audio source
        pub fn stop_source(&mut self, _entity_id: EntityId) {}

        /// Stop all audio
        pub fn stop_all(&mut self) {}

        /// Pause an audio source
        pub fn pause_source(&mut self, _entity_id: EntityId) {}

        /// Resume an audio source
        pub fn resume_source(&mut self, _entity_id: EntityId) {}

        /// Pause all audio
        pub fn pause_all(&mut self) {}

        /// Resume all audio
        pub fn resume_all(&mut self) {}

        /// Update audio system (call each frame)
        pub fn update(&mut self, _scene: &SceneData) {}

        /// Set the audio listener position
        pub fn set_listener(&mut self, listener: AudioListener) {
            self.listener = listener;
        }

        /// Set listener from a transform (usually camera)
        pub fn set_listener_transform(&mut self, transform: &Transform) {
            self.listener = AudioListener::from_transform(transform);
        }

        /// Check if an entity's audio is playing
        pub fn is_playing(&self, _entity_id: EntityId) -> bool {
            false
        }

        /// Check if an entity's audio is paused
        pub fn is_paused(&self, _entity_id: EntityId) -> bool {
            false
        }

        /// Get the number of currently playing sources
        pub fn playing_count(&self) -> usize {
            0
        }

        /// Preview an audio clip (for asset browser)
        pub fn preview_clip(&mut self, _clip_path: &Path) -> bool {
            self.warn_once();
            false
        }
    }
}

// Re-export AudioEngine from the appropriate module
#[allow(unused_imports)]
pub use engine::AudioEngine;
