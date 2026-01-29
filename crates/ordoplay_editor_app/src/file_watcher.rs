// SPDX-License-Identifier: MIT OR Apache-2.0
//! File system watcher for detecting asset changes.
//!
//! Provides debounced file system events for hot-reloading assets
//! and keeping the asset browser in sync with the filesystem.


use notify_debouncer_full::{
    new_debouncer,
    notify::{self, RecommendedWatcher, RecursiveMode},
    DebounceEventResult, Debouncer, RecommendedCache,
};
use parking_lot::RwLock;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::Duration;

/// Events emitted by the file watcher
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone)]
pub enum FileEvent {
    /// A file was created
    Created(PathBuf),
    /// A file was modified
    Modified(PathBuf),
    /// A file was deleted
    Deleted(PathBuf),
    /// A file was renamed (old path, new path)
    Renamed(PathBuf, PathBuf),
    /// An error occurred
    Error(String),
}

/// Configuration for the file watcher
#[derive(Debug, Clone)]
pub struct FileWatcherConfig {
    /// Debounce duration for events
    pub debounce_duration: Duration,
    /// Whether to watch directories recursively
    pub recursive: bool,
    /// File extensions to watch (empty = watch all)
    pub extensions: HashSet<String>,
}

impl Default for FileWatcherConfig {
    fn default() -> Self {
        Self {
            debounce_duration: Duration::from_millis(250),
            recursive: true,
            extensions: HashSet::new(),
        }
    }
}

impl FileWatcherConfig {
    /// Create a config that watches common asset file types
    pub fn for_assets() -> Self {
        let mut extensions = HashSet::new();
        // Images
        extensions.insert("png".to_string());
        extensions.insert("jpg".to_string());
        extensions.insert("jpeg".to_string());
        extensions.insert("gif".to_string());
        extensions.insert("bmp".to_string());
        extensions.insert("tga".to_string());
        extensions.insert("hdr".to_string());
        extensions.insert("exr".to_string());
        extensions.insert("webp".to_string());
        // 3D models
        extensions.insert("glb".to_string());
        extensions.insert("gltf".to_string());
        extensions.insert("obj".to_string());
        extensions.insert("fbx".to_string());
        // Audio
        extensions.insert("wav".to_string());
        extensions.insert("mp3".to_string());
        extensions.insert("ogg".to_string());
        extensions.insert("flac".to_string());
        // Shaders
        extensions.insert("wgsl".to_string());
        extensions.insert("glsl".to_string());
        extensions.insert("hlsl".to_string());
        // Scripts
        extensions.insert("lua".to_string());
        extensions.insert("wasm".to_string());
        // Materials and scenes
        extensions.insert("ron".to_string());
        extensions.insert("mat".to_string());
        extensions.insert("scene".to_string());

        Self {
            debounce_duration: Duration::from_millis(250),
            recursive: true,
            extensions,
        }
    }
}

/// File system watcher for asset changes
#[allow(dead_code)] // Intentionally kept for API completeness
pub struct FileWatcher {
    /// The underlying debounced watcher
    _watcher: Debouncer<RecommendedWatcher, RecommendedCache>,
    /// Receiver for file events
    event_rx: Receiver<FileEvent>,
    /// Watched directories
    watched_dirs: Arc<RwLock<HashSet<PathBuf>>>,
    /// Configuration
    config: FileWatcherConfig,
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl FileWatcher {
    /// Create a new file watcher with the given configuration
    pub fn new(config: FileWatcherConfig) -> Result<Self, notify::Error> {
        let (event_tx, event_rx) = mpsc::channel();
        let extensions = config.extensions.clone();

        // Create the debounced watcher
        let watcher = new_debouncer(
            config.debounce_duration,
            None,
            move |result: DebounceEventResult| {
                match result {
                    Ok(events) => {
                        for event in events {
                            // Filter by extension if configured
                            let paths: Vec<_> = event.paths.iter()
                                .filter(|p| {
                                    if extensions.is_empty() {
                                        true
                                    } else {
                                        p.extension()
                                            .and_then(|e| e.to_str())
                                            .map(|e| extensions.contains(&e.to_lowercase()))
                                            .unwrap_or(false)
                                    }
                                })
                                .cloned()
                                .collect();

                            if paths.is_empty() {
                                continue;
                            }

                            // Convert notify events to our events
                            use notify::EventKind;
                            match event.kind {
                                EventKind::Create(_) => {
                                    for path in paths {
                                        let _ = event_tx.send(FileEvent::Created(path));
                                    }
                                }
                                EventKind::Modify(_) => {
                                    for path in paths {
                                        let _ = event_tx.send(FileEvent::Modified(path));
                                    }
                                }
                                EventKind::Remove(_) => {
                                    for path in paths {
                                        let _ = event_tx.send(FileEvent::Deleted(path));
                                    }
                                }
                                EventKind::Any | EventKind::Access(_) | EventKind::Other => {}
                            }
                        }
                    }
                    Err(errors) => {
                        for error in errors {
                            let _ = event_tx.send(FileEvent::Error(error.to_string()));
                        }
                    }
                }
            },
        )?;

        Ok(Self {
            _watcher: watcher,
            event_rx,
            watched_dirs: Arc::new(RwLock::new(HashSet::new())),
            config,
        })
    }

    /// Create a file watcher configured for asset watching
    pub fn for_assets() -> Result<Self, notify::Error> {
        Self::new(FileWatcherConfig::for_assets())
    }

    /// Watch a directory for changes
    pub fn watch(&mut self, path: impl AsRef<Path>) -> Result<(), notify::Error> {
        let path = path.as_ref().to_path_buf();
        let mode = if self.config.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        self._watcher.watch(&path, mode)?;
        self.watched_dirs.write().insert(path.clone());
        tracing::info!("Watching directory for changes: {:?}", path);
        Ok(())
    }

    /// Stop watching a directory
    pub fn unwatch(&mut self, path: impl AsRef<Path>) -> Result<(), notify::Error> {
        let path = path.as_ref();
        self._watcher.unwatch(path)?;
        self.watched_dirs.write().remove(path);
        tracing::info!("Stopped watching directory: {:?}", path);
        Ok(())
    }

    /// Check if a directory is being watched
    pub fn is_watching(&self, path: &Path) -> bool {
        self.watched_dirs.read().contains(path)
    }

    /// Get all watched directories
    pub fn watched_directories(&self) -> Vec<PathBuf> {
        self.watched_dirs.read().iter().cloned().collect()
    }

    /// Poll for pending file events (non-blocking)
    pub fn poll_events(&self) -> Vec<FileEvent> {
        let mut events = Vec::new();
        loop {
            match self.event_rx.try_recv() {
                Ok(event) => events.push(event),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    tracing::warn!("File watcher channel disconnected");
                    break;
                }
            }
        }
        events
    }

    /// Get configuration
    pub fn config(&self) -> &FileWatcherConfig {
        &self.config
    }
}

/// Manages file watchers and aggregates events
#[allow(dead_code)] // Intentionally kept for API completeness
pub struct FileWatcherManager {
    /// Active watchers
    watchers: Vec<FileWatcher>,
    /// Pending events from all watchers
    pending_events: Vec<FileEvent>,
    /// Paths that have been modified (for tracking)
    modified_paths: HashSet<PathBuf>,
    /// Paths that need thumbnail refresh
    needs_thumbnail_refresh: HashSet<PathBuf>,
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl FileWatcherManager {
    /// Create a new file watcher manager
    pub fn new() -> Self {
        Self {
            watchers: Vec::new(),
            pending_events: Vec::new(),
            modified_paths: HashSet::new(),
            needs_thumbnail_refresh: HashSet::new(),
        }
    }

    /// Add a watcher
    pub fn add_watcher(&mut self, watcher: FileWatcher) {
        self.watchers.push(watcher);
    }

    /// Create and add a watcher for a directory
    pub fn watch_directory(&mut self, path: impl AsRef<Path>) -> Result<(), notify::Error> {
        let mut watcher = FileWatcher::for_assets()?;
        watcher.watch(path)?;
        self.watchers.push(watcher);
        Ok(())
    }

    /// Poll all watchers for events
    pub fn poll(&mut self) -> &[FileEvent] {
        self.pending_events.clear();
        self.modified_paths.clear();

        for watcher in &self.watchers {
            let events = watcher.poll_events();
            for event in events {
                match &event {
                    FileEvent::Created(path) | FileEvent::Modified(path) => {
                        self.modified_paths.insert(path.clone());
                        self.needs_thumbnail_refresh.insert(path.clone());
                    }
                    FileEvent::Deleted(path) => {
                        self.modified_paths.insert(path.clone());
                        self.needs_thumbnail_refresh.remove(path);
                    }
                    FileEvent::Renamed(old, new) => {
                        self.modified_paths.insert(old.clone());
                        self.modified_paths.insert(new.clone());
                        self.needs_thumbnail_refresh.remove(old);
                        self.needs_thumbnail_refresh.insert(new.clone());
                    }
                    FileEvent::Error(_) => {}
                }
                self.pending_events.push(event);
            }
        }

        &self.pending_events
    }

    /// Check if any events occurred
    pub fn has_events(&self) -> bool {
        !self.pending_events.is_empty()
    }

    /// Get paths that were modified in the last poll
    pub fn modified_paths(&self) -> &HashSet<PathBuf> {
        &self.modified_paths
    }

    /// Get and clear paths that need thumbnail refresh
    pub fn take_thumbnail_refresh_paths(&mut self) -> HashSet<PathBuf> {
        std::mem::take(&mut self.needs_thumbnail_refresh)
    }

    /// Check if a specific path was modified
    pub fn was_modified(&self, path: &Path) -> bool {
        self.modified_paths.contains(path)
    }

    /// Get the number of active watchers
    pub fn watcher_count(&self) -> usize {
        self.watchers.len()
    }
}

impl Default for FileWatcherManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = FileWatcherConfig::default();
        assert!(config.recursive);
        assert!(config.extensions.is_empty());
    }

    #[test]
    fn test_config_for_assets() {
        let config = FileWatcherConfig::for_assets();
        assert!(config.extensions.contains("png"));
        assert!(config.extensions.contains("glb"));
        assert!(config.extensions.contains("wgsl"));
    }
}
