// SPDX-License-Identifier: MIT OR Apache-2.0
//! Play mode for in-editor preview.
//!
//! This module handles:
//! - Entering and exiting play mode
//! - Pausing and resuming gameplay
//! - Scene state backup and restoration
//! - Play mode UI indicators

use crate::state::{SceneData, Selection};

/// Play mode state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayState {
    /// Editor mode (normal editing)
    #[default]
    Stopped,
    /// Game is running
    Playing,
    /// Game is paused
    Paused,
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl PlayState {
    /// Check if we're in any play mode (playing or paused)
    pub fn is_active(&self) -> bool {
        matches!(self, PlayState::Playing | PlayState::Paused)
    }

    /// Check if currently playing (not paused)
    pub fn is_playing(&self) -> bool {
        matches!(self, PlayState::Playing)
    }

    /// Check if currently paused
    pub fn is_paused(&self) -> bool {
        matches!(self, PlayState::Paused)
    }

    /// Check if stopped (in editor mode)
    pub fn is_stopped(&self) -> bool {
        matches!(self, PlayState::Stopped)
    }
}

/// Play mode manager - handles entering/exiting play mode
pub struct PlayModeManager {
    /// Current play state
    pub state: PlayState,
    /// Backup of scene before entering play mode
    scene_backup: Option<SceneData>,
    /// Backup of selection before entering play mode
    selection_backup: Option<Selection>,
    /// Time scale for simulation (1.0 = normal speed)
    pub time_scale: f32,
    /// Accumulated delta time for fixed timestep
    accumulated_time: f64,
    /// Frame count since play started
    pub frame_count: u64,
    /// Elapsed time since play started
    pub elapsed_time: f64,
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl PlayModeManager {
    pub fn new() -> Self {
        Self {
            state: PlayState::Stopped,
            scene_backup: None,
            selection_backup: None,
            time_scale: 1.0,
            accumulated_time: 0.0,
            frame_count: 0,
            elapsed_time: 0.0,
        }
    }

    /// Enter play mode
    /// Returns true if successfully entered play mode
    pub fn play(&mut self, scene: &SceneData, selection: &Selection) -> bool {
        match self.state {
            PlayState::Stopped => {
                // Backup current state
                self.scene_backup = Some(scene.clone());
                self.selection_backup = Some(selection.clone());
                self.state = PlayState::Playing;
                self.frame_count = 0;
                self.elapsed_time = 0.0;
                self.accumulated_time = 0.0;
                tracing::info!("Entered play mode");
                true
            }
            PlayState::Paused => {
                // Resume from pause
                self.state = PlayState::Playing;
                tracing::info!("Resumed play mode");
                true
            }
            PlayState::Playing => {
                // Already playing
                false
            }
        }
    }

    /// Pause play mode
    pub fn pause(&mut self) -> bool {
        if self.state == PlayState::Playing {
            self.state = PlayState::Paused;
            tracing::info!("Paused play mode");
            true
        } else {
            false
        }
    }

    /// Toggle pause/resume
    pub fn toggle_pause(&mut self) -> bool {
        match self.state {
            PlayState::Playing => self.pause(),
            PlayState::Paused => {
                self.state = PlayState::Playing;
                tracing::info!("Resumed play mode");
                true
            }
            PlayState::Stopped => false,
        }
    }

    /// Stop play mode and restore scene state
    /// Returns the restored scene data if any
    pub fn stop(&mut self) -> Option<(SceneData, Selection)> {
        if !self.state.is_active() {
            return None;
        }

        self.state = PlayState::Stopped;
        self.frame_count = 0;
        self.elapsed_time = 0.0;
        self.accumulated_time = 0.0;

        let scene = self.scene_backup.take();
        let selection = self.selection_backup.take();

        tracing::info!("Stopped play mode");

        match (scene, selection) {
            (Some(s), Some(sel)) => Some((s, sel)),
            _ => None,
        }
    }

    /// Update the simulation (called each frame while playing)
    /// Returns the number of fixed timesteps to run
    pub fn update(&mut self, delta_time: f64, fixed_timestep: f64) -> u32 {
        if self.state != PlayState::Playing {
            return 0;
        }

        let scaled_delta = delta_time * self.time_scale as f64;
        self.elapsed_time += scaled_delta;
        self.accumulated_time += scaled_delta;
        self.frame_count += 1;

        // Calculate number of fixed timesteps
        let mut steps = 0;
        while self.accumulated_time >= fixed_timestep {
            self.accumulated_time -= fixed_timestep;
            steps += 1;

            // Limit max steps per frame to prevent spiral of death
            if steps >= 8 {
                self.accumulated_time = 0.0;
                break;
            }
        }

        steps
    }

    /// Step forward one frame (while paused)
    pub fn step_frame(&mut self, fixed_timestep: f64) -> bool {
        if self.state == PlayState::Paused {
            self.elapsed_time += fixed_timestep;
            self.frame_count += 1;
            true
        } else {
            false
        }
    }

    /// Check if editing should be disabled
    pub fn is_editing_disabled(&self) -> bool {
        self.state.is_active()
    }

    /// Get a status string for display
    pub fn status_text(&self) -> &'static str {
        match self.state {
            PlayState::Stopped => "Edit Mode",
            PlayState::Playing => "Playing",
            PlayState::Paused => "Paused",
        }
    }

    /// Get the current state
    pub fn current_state(&self) -> PlayState {
        self.state
    }

    /// Reset time scale to normal
    pub fn reset_time_scale(&mut self) {
        self.time_scale = 1.0;
    }

    /// Set time scale (clamped to reasonable range)
    pub fn set_time_scale(&mut self, scale: f32) {
        self.time_scale = scale.clamp(0.0, 10.0);
    }
}

impl Default for PlayModeManager {
    fn default() -> Self {
        Self::new()
    }
}
