// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hot reload system for assets.
//!
//! Monitors file changes and automatically reloads modified assets,
//! including textures, materials, shaders, and scripts.


use crate::file_watcher::{FileEvent, FileWatcherManager};
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Types of assets that can be hot-reloaded
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HotReloadAssetType {
    /// Texture files (png, jpg, etc.)
    Texture,
    /// Shader files (wgsl, glsl, hlsl)
    Shader,
    /// Material files
    Material,
    /// Script files (lua, wasm)
    Script,
    /// Scene files
    Scene,
    /// Animation files
    Animation,
    /// Audio files
    Audio,
    /// Unknown/other
    Other,
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl HotReloadAssetType {
    /// Detect asset type from file extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            // Textures
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "tga" | "dds" | "ktx2" | "exr" | "hdr"
            | "webp" => Self::Texture,
            // Shaders
            "wgsl" | "glsl" | "hlsl" | "spv" | "vert" | "frag" | "comp" => Self::Shader,
            // Materials
            "mat" | "material" => Self::Material,
            // Scripts
            "lua" | "wasm" | "rs" => Self::Script,
            // Scenes
            "scene" | "ron" => Self::Scene,
            // Animations
            "anim" | "animation" => Self::Animation,
            // Audio
            "wav" | "mp3" | "ogg" | "flac" => Self::Audio,
            // Other
            _ => Self::Other,
        }
    }

    /// Check if this asset type from a path
    pub fn from_path(path: &Path) -> Self {
        path.extension()
            .and_then(|e| e.to_str())
            .map(Self::from_extension)
            .unwrap_or(Self::Other)
    }
}

/// A pending hot reload event
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone)]
pub struct HotReloadEvent {
    /// Path to the modified asset
    pub path: PathBuf,
    /// Type of asset
    pub asset_type: HotReloadAssetType,
    /// When the change was detected
    pub detected_at: Instant,
    /// Whether this is a deletion
    pub is_deletion: bool,
}

/// Callback type for hot reload notifications
#[allow(dead_code)] // Intentionally kept for API completeness
pub type HotReloadCallback = Box<dyn Fn(&HotReloadEvent) + Send + Sync>;

/// Hot reload manager coordinates asset reloading
#[allow(dead_code)] // Intentionally kept for API completeness
pub struct HotReloadManager {
    /// File watcher
    watcher: Option<FileWatcherManager>,
    /// Pending reload events
    pending_events: Vec<HotReloadEvent>,
    /// Paths currently being processed
    processing: HashSet<PathBuf>,
    /// Callbacks registered for specific asset types
    callbacks: HashMap<HotReloadAssetType, Vec<HotReloadCallback>>,
    /// Global callbacks (called for all asset types)
    global_callbacks: Vec<HotReloadCallback>,
    /// Statistics
    stats: Arc<RwLock<HotReloadStats>>,
    /// Debounce duration (wait this long after last change before reloading)
    debounce_duration: Duration,
    /// Asset types to watch (empty = watch all)
    watched_types: HashSet<HotReloadAssetType>,
    /// Enabled state
    enabled: bool,
}

/// Statistics for hot reload operations
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone, Default)]
pub struct HotReloadStats {
    /// Total assets reloaded
    pub total_reloaded: usize,
    /// Reloads by asset type
    pub by_type: HashMap<HotReloadAssetType, usize>,
    /// Failed reloads
    pub failed: usize,
    /// Last reload time
    pub last_reload: Option<Instant>,
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl HotReloadManager {
    /// Create a new hot reload manager
    pub fn new() -> Self {
        Self {
            watcher: None,
            pending_events: Vec::new(),
            processing: HashSet::new(),
            callbacks: HashMap::new(),
            global_callbacks: Vec::new(),
            stats: Arc::new(RwLock::new(HotReloadStats::default())),
            debounce_duration: Duration::from_millis(100),
            watched_types: HashSet::new(),
            enabled: true,
        }
    }

    /// Start watching a directory for changes
    pub fn watch_directory(&mut self, path: impl AsRef<Path>) -> Result<(), notify::Error> {
        let mut watcher = FileWatcherManager::new();
        watcher.watch_directory(path)?;
        self.watcher = Some(watcher);
        Ok(())
    }

    /// Set the debounce duration
    pub fn set_debounce_duration(&mut self, duration: Duration) {
        self.debounce_duration = duration;
    }

    /// Enable or disable hot reload
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if hot reload is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Set which asset types to watch (empty = watch all)
    pub fn set_watched_types(&mut self, types: HashSet<HotReloadAssetType>) {
        self.watched_types = types;
    }

    /// Register a callback for a specific asset type
    pub fn on_reload(&mut self, asset_type: HotReloadAssetType, callback: HotReloadCallback) {
        self.callbacks
            .entry(asset_type)
            .or_default()
            .push(callback);
    }

    /// Register a global callback (called for all asset types)
    pub fn on_any_reload(&mut self, callback: HotReloadCallback) {
        self.global_callbacks.push(callback);
    }

    /// Poll for file changes and process them
    pub fn poll(&mut self) -> Vec<HotReloadEvent> {
        if !self.enabled {
            return Vec::new();
        }

        // Poll the file watcher
        if let Some(ref mut watcher) = self.watcher {
            let events = watcher.poll();
            for event in events {
                let (path, is_deletion) = match event {
                    FileEvent::Created(p) | FileEvent::Modified(p) => (p.clone(), false),
                    FileEvent::Deleted(p) => (p.clone(), true),
                    FileEvent::Renamed(_, new) => (new.clone(), false),
                    FileEvent::Error(_) => continue,
                };

                let asset_type = HotReloadAssetType::from_path(&path);

                // Filter by watched types
                if !self.watched_types.is_empty() && !self.watched_types.contains(&asset_type) {
                    continue;
                }

                // Add to pending if not already processing
                if !self.processing.contains(&path) {
                    self.pending_events.push(HotReloadEvent {
                        path,
                        asset_type,
                        detected_at: Instant::now(),
                        is_deletion,
                    });
                }
            }
        }

        // Process events that have passed the debounce window
        let now = Instant::now();
        let mut ready_events = Vec::new();
        let mut not_ready = Vec::new();
        for event in self.pending_events.drain(..) {
            if now.duration_since(event.detected_at) >= self.debounce_duration {
                ready_events.push(event);
            } else {
                not_ready.push(event);
            }
        }
        self.pending_events = not_ready;

        // Fire callbacks and update stats
        for event in &ready_events {
            self.fire_callbacks(event);
            self.update_stats(event);
        }

        ready_events
    }

    /// Fire callbacks for an event
    fn fire_callbacks(&self, event: &HotReloadEvent) {
        // Fire type-specific callbacks
        if let Some(callbacks) = self.callbacks.get(&event.asset_type) {
            for callback in callbacks {
                callback(event);
            }
        }

        // Fire global callbacks
        for callback in &self.global_callbacks {
            callback(event);
        }
    }

    /// Update statistics
    fn update_stats(&self, event: &HotReloadEvent) {
        let mut stats = self.stats.write();
        stats.total_reloaded += 1;
        *stats.by_type.entry(event.asset_type).or_insert(0) += 1;
        stats.last_reload = Some(Instant::now());
    }

    /// Get statistics
    pub fn stats(&self) -> HotReloadStats {
        self.stats.read().clone()
    }

    /// Get pending event count
    pub fn pending_count(&self) -> usize {
        self.pending_events.len()
    }

    /// Check if there are pending reloads
    pub fn has_pending(&self) -> bool {
        !self.pending_events.is_empty()
    }

    /// Get the paths of textures that were modified
    pub fn get_modified_texture_paths(&self, events: &[HotReloadEvent]) -> Vec<PathBuf> {
        events
            .iter()
            .filter(|e| e.asset_type == HotReloadAssetType::Texture && !e.is_deletion)
            .map(|e| e.path.clone())
            .collect()
    }

    /// Get the paths of shaders that were modified
    pub fn get_modified_shader_paths(&self, events: &[HotReloadEvent]) -> Vec<PathBuf> {
        events
            .iter()
            .filter(|e| e.asset_type == HotReloadAssetType::Shader && !e.is_deletion)
            .map(|e| e.path.clone())
            .collect()
    }
}

impl Default for HotReloadManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Integration helper for connecting hot reload to the editor
#[allow(dead_code)] // Intentionally kept for API completeness
pub struct HotReloadIntegration {
    /// Modified texture paths (for thumbnail reload)
    pub modified_textures: Vec<PathBuf>,
    /// Modified shader paths (for shader recompilation)
    pub modified_shaders: Vec<PathBuf>,
    /// Modified material paths
    pub modified_materials: Vec<PathBuf>,
    /// Modified script paths
    pub modified_scripts: Vec<PathBuf>,
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl HotReloadIntegration {
    /// Process hot reload events and categorize them
    pub fn from_events(events: &[HotReloadEvent]) -> Self {
        let mut result = Self {
            modified_textures: Vec::new(),
            modified_shaders: Vec::new(),
            modified_materials: Vec::new(),
            modified_scripts: Vec::new(),
        };

        for event in events {
            if event.is_deletion {
                continue;
            }

            match event.asset_type {
                HotReloadAssetType::Texture => {
                    result.modified_textures.push(event.path.clone());
                }
                HotReloadAssetType::Shader => {
                    result.modified_shaders.push(event.path.clone());
                }
                HotReloadAssetType::Material => {
                    result.modified_materials.push(event.path.clone());
                }
                HotReloadAssetType::Script => {
                    result.modified_scripts.push(event.path.clone());
                }
                _ => {}
            }
        }

        result
    }

    /// Check if there are any modified assets
    pub fn has_changes(&self) -> bool {
        !self.modified_textures.is_empty()
            || !self.modified_shaders.is_empty()
            || !self.modified_materials.is_empty()
            || !self.modified_scripts.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_type_detection() {
        assert_eq!(
            HotReloadAssetType::from_extension("png"),
            HotReloadAssetType::Texture
        );
        assert_eq!(
            HotReloadAssetType::from_extension("wgsl"),
            HotReloadAssetType::Shader
        );
        assert_eq!(
            HotReloadAssetType::from_extension("lua"),
            HotReloadAssetType::Script
        );
    }

    #[test]
    fn test_manager_creation() {
        let manager = HotReloadManager::new();
        assert!(manager.is_enabled());
        assert_eq!(manager.pending_count(), 0);
    }
}
