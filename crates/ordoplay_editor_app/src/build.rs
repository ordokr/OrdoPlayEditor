// SPDX-License-Identifier: MIT OR Apache-2.0
//! Build system for exporting projects.
//!
//! This module handles:
//! - Exporting scenes to runtime format
//! - Asset processing and compression
//! - Platform-specific build generation
//! - Build progress reporting

use crate::project::{BuildConfiguration, ProjectSettings, TargetPlatform, TextureCompression};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Build progress reporting
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone)]
pub struct BuildProgress {
    /// Current step description
    pub step: String,
    /// Current progress (0-100)
    pub progress: u32,
    /// Total steps
    pub total_steps: u32,
    /// Current step number
    pub current_step: u32,
    /// Whether build is complete
    pub complete: bool,
    /// Error message if any
    pub error: Option<String>,
}

impl Default for BuildProgress {
    fn default() -> Self {
        Self {
            step: "Preparing...".to_string(),
            progress: 0,
            total_steps: 1,
            current_step: 0,
            complete: false,
            error: None,
        }
    }
}

/// Shared build state for async builds
pub struct BuildState {
    pub progress: AtomicU32,
    pub cancelled: AtomicBool,
    pub step: parking_lot::Mutex<String>,
    pub error: parking_lot::Mutex<Option<String>>,
    pub complete: AtomicBool,
}

impl BuildState {
    pub fn new() -> Self {
        Self {
            progress: AtomicU32::new(0),
            cancelled: AtomicBool::new(false),
            step: parking_lot::Mutex::new("Initializing...".to_string()),
            error: parking_lot::Mutex::new(None),
            complete: AtomicBool::new(false),
        }
    }

    pub fn set_step(&self, step: impl Into<String>) {
        *self.step.lock() = step.into();
    }

    pub fn set_progress(&self, progress: u32) {
        self.progress.store(progress.min(100), Ordering::Relaxed);
    }

    pub fn set_error(&self, error: impl Into<String>) {
        *self.error.lock() = Some(error.into());
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn mark_complete(&self) {
        self.complete.store(true, Ordering::Relaxed);
        self.progress.store(100, Ordering::Relaxed);
    }

    pub fn get_progress(&self) -> BuildProgress {
        BuildProgress {
            step: self.step.lock().clone(),
            progress: self.progress.load(Ordering::Relaxed),
            total_steps: 0,
            current_step: 0,
            complete: self.complete.load(Ordering::Relaxed),
            error: self.error.lock().clone(),
        }
    }
}

impl Default for BuildState {
    fn default() -> Self {
        Self::new()
    }
}

/// Build result
#[derive(Debug)]
pub enum BuildResult {
    Success {
        output_dir: PathBuf,
        build_time_secs: f64,
        assets_processed: u32,
        scenes_processed: u32,
    },
    Cancelled,
    Failed(String),
}

/// Build options
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone)]
pub struct BuildOptions {
    /// Target platform
    pub platform: TargetPlatform,
    /// Build configuration
    pub configuration: BuildConfiguration,
    /// Output directory
    pub output_dir: PathBuf,
    /// Whether to clean output directory first
    pub clean: bool,
    /// Texture compression format
    pub texture_compression: TextureCompression,
    /// Whether to compress assets
    pub compress_assets: bool,
    /// Whether to include debug symbols
    pub include_debug_symbols: bool,
}

impl BuildOptions {
    /// Create build options from project settings
    pub fn from_settings(settings: &ProjectSettings, project_dir: &Path) -> Self {
        let platform = settings.target_platform;
        let platform_settings = settings.get_platform_settings(platform);

        Self {
            platform,
            configuration: settings.build_configuration,
            output_dir: project_dir.join(&platform_settings.output_dir).join(platform.display_name()),
            clean: true,
            texture_compression: platform_settings.texture_compression,
            compress_assets: platform_settings.compress_assets,
            include_debug_symbols: platform_settings.include_debug_symbols,
        }
    }
}

/// Build system
pub struct BuildSystem;

impl BuildSystem {
    /// Start a build (synchronous, for simplicity)
    pub fn build(
        settings: &ProjectSettings,
        project_dir: &Path,
        state: &BuildState,
    ) -> BuildResult {
        let start_time = std::time::Instant::now();
        let options = BuildOptions::from_settings(settings, project_dir);

        tracing::info!("Starting build for {:?} {:?}", options.platform, options.configuration);
        state.set_step("Preparing build...");
        state.set_progress(0);

        // Create output directory
        if options.clean && options.output_dir.exists() {
            state.set_step("Cleaning output directory...");
            if let Err(e) = std::fs::remove_dir_all(&options.output_dir) {
                state.set_error(format!("Failed to clean output directory: {}", e));
                return BuildResult::Failed(format!("Failed to clean output directory: {}", e));
            }
        }

        if let Err(e) = std::fs::create_dir_all(&options.output_dir) {
            state.set_error(format!("Failed to create output directory: {}", e));
            return BuildResult::Failed(format!("Failed to create output directory: {}", e));
        }

        if state.is_cancelled() {
            return BuildResult::Cancelled;
        }

        // Process scenes
        state.set_step("Processing scenes...");
        state.set_progress(10);

        let scenes_processed = Self::process_scenes(settings, project_dir, &options, state);
        if state.is_cancelled() {
            return BuildResult::Cancelled;
        }

        // Process assets
        state.set_step("Processing assets...");
        state.set_progress(40);

        let assets_processed = Self::process_assets(project_dir, &options, state);
        if state.is_cancelled() {
            return BuildResult::Cancelled;
        }

        // Generate runtime config
        state.set_step("Generating runtime config...");
        state.set_progress(80);

        if let Err(e) = Self::generate_runtime_config(settings, &options) {
            state.set_error(format!("Failed to generate runtime config: {}", e));
            return BuildResult::Failed(format!("Failed to generate runtime config: {}", e));
        }

        // Finalize
        state.set_step("Finalizing build...");
        state.set_progress(95);

        if state.is_cancelled() {
            return BuildResult::Cancelled;
        }

        state.mark_complete();
        let build_time = start_time.elapsed().as_secs_f64();

        tracing::info!(
            "Build completed in {:.2}s: {} scenes, {} assets",
            build_time,
            scenes_processed,
            assets_processed
        );

        BuildResult::Success {
            output_dir: options.output_dir,
            build_time_secs: build_time,
            assets_processed,
            scenes_processed,
        }
    }

    fn process_scenes(
        settings: &ProjectSettings,
        project_dir: &Path,
        options: &BuildOptions,
        state: &BuildState,
    ) -> u32 {
        let scenes_dir = options.output_dir.join("Scenes");
        let _ = std::fs::create_dir_all(&scenes_dir);

        let mut processed = 0;
        let total = settings.scenes.build_scenes.len();

        for (i, scene_entry) in settings.scenes.build_scenes.iter().enumerate() {
            if state.is_cancelled() {
                break;
            }

            if !scene_entry.enabled {
                continue;
            }

            let progress = 10 + ((i as u32 * 30) / total.max(1) as u32);
            state.set_progress(progress);
            state.set_step(format!("Processing scene: {}", scene_entry.path.display()));

            let source_path = project_dir.join(&scene_entry.path);
            if !source_path.exists() {
                tracing::warn!("Scene not found: {:?}", source_path);
                continue;
            }

            // Copy scene file to output
            let dest_name = scene_entry.path.file_name().unwrap_or_default();
            let dest_path = scenes_dir.join(dest_name);

            if let Err(e) = std::fs::copy(&source_path, &dest_path) {
                tracing::error!("Failed to copy scene {:?}: {}", source_path, e);
                continue;
            }

            processed += 1;
        }

        processed
    }

    fn process_assets(
        project_dir: &Path,
        options: &BuildOptions,
        state: &BuildState,
    ) -> u32 {
        let assets_source = project_dir.join("Assets");
        let assets_dest = options.output_dir.join("Assets");

        if !assets_source.exists() {
            return 0;
        }

        let _ = std::fs::create_dir_all(&assets_dest);

        // Count files first for progress
        let files: Vec<walkdir::DirEntry> = walkdir::WalkDir::new(&assets_source)
            .into_iter()
            .filter_map(|e: Result<walkdir::DirEntry, walkdir::Error>| e.ok())
            .filter(|e: &walkdir::DirEntry| e.file_type().is_file())
            .collect();

        let total = files.len();
        let mut processed = 0;

        for (i, entry) in files.iter().enumerate() {
            if state.is_cancelled() {
                break;
            }

            let progress = 40 + ((i as u32 * 40) / total.max(1) as u32);
            state.set_progress(progress);

            let entry_path = entry.path();
            let rel_path = entry_path.strip_prefix(&assets_source).unwrap_or(entry_path);
            let dest_path = assets_dest.join(rel_path);

            // Create parent directories
            if let Some(parent) = dest_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            // Process based on file type
            let extension = entry_path.extension()
                .and_then(|s: &std::ffi::OsStr| s.to_str())
                .unwrap_or("");

            let result = match extension.to_lowercase().as_str() {
                "png" | "jpg" | "jpeg" | "bmp" | "tga" => {
                    Self::process_texture(entry.path(), &dest_path, options)
                }
                _ => {
                    // Just copy other files
                    std::fs::copy(entry.path(), &dest_path).map(|_| ())
                }
            };

            if let Err(e) = result {
                tracing::warn!("Failed to process asset {:?}: {}", entry.path(), e);
                continue;
            }

            processed += 1;
        }

        processed
    }

    fn process_texture(
        source: &Path,
        dest: &Path,
        _options: &BuildOptions,
    ) -> std::io::Result<()> {
        // For now, just copy textures
        // In a real implementation, this would:
        // - Resize based on quality settings
        // - Compress based on texture_compression setting
        // - Generate mipmaps
        std::fs::copy(source, dest)?;
        Ok(())
    }

    fn generate_runtime_config(
        settings: &ProjectSettings,
        options: &BuildOptions,
    ) -> std::io::Result<()> {
        // Generate a runtime configuration file
        let config = RuntimeConfig {
            project_name: settings.metadata.name.clone(),
            version: settings.metadata.version.clone(),
            startup_scene: settings.scenes.startup_scene.clone()
                .or_else(|| settings.scenes.build_scenes.first().map(|s| s.path.clone())),
            graphics: RuntimeGraphicsConfig {
                default_width: settings.graphics.default_width,
                default_height: settings.graphics.default_height,
                fullscreen: settings.graphics.fullscreen,
                vsync: settings.graphics.vsync,
                msaa: settings.graphics.msaa_samples,
            },
            audio: RuntimeAudioConfig {
                master_volume: settings.audio.master_volume,
                music_volume: settings.audio.music_volume,
                sfx_volume: settings.audio.sfx_volume,
            },
            is_debug: options.configuration == BuildConfiguration::Debug,
        };

        let config_path = options.output_dir.join("config.ron");
        let config_str = ron::ser::to_string_pretty(&config, ron::ser::PrettyConfig::default())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        std::fs::write(config_path, config_str)
    }
}

/// Runtime configuration file format
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RuntimeConfig {
    project_name: String,
    version: String,
    startup_scene: Option<PathBuf>,
    graphics: RuntimeGraphicsConfig,
    audio: RuntimeAudioConfig,
    is_debug: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RuntimeGraphicsConfig {
    default_width: u32,
    default_height: u32,
    fullscreen: bool,
    vsync: bool,
    msaa: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RuntimeAudioConfig {
    master_volume: f32,
    music_volume: f32,
    sfx_volume: f32,
}

/// Build manager for the editor
pub struct BuildManager {
    /// Current build state (if building)
    pub current_build: Option<Arc<BuildState>>,
    /// Last build result
    pub last_result: Option<BuildResult>,
}

impl BuildManager {
    pub fn new() -> Self {
        Self {
            current_build: None,
            last_result: None,
        }
    }

    /// Check if a build is in progress
    pub fn is_building(&self) -> bool {
        self.current_build.as_ref()
            .map(|s| !s.complete.load(Ordering::Relaxed))
            .unwrap_or(false)
    }

    /// Get current build progress
    pub fn get_progress(&self) -> Option<BuildProgress> {
        self.current_build.as_ref().map(|s| s.get_progress())
    }

    /// Cancel current build
    pub fn cancel(&self) {
        if let Some(state) = &self.current_build {
            state.cancel();
        }
    }

    /// Start a new build (blocking for simplicity)
    pub fn start_build(&mut self, settings: &ProjectSettings, project_dir: &Path) -> &Option<BuildResult> {
        let state = Arc::new(BuildState::new());
        self.current_build = Some(Arc::clone(&state));

        let result = BuildSystem::build(settings, project_dir, &state);
        self.last_result = Some(result);
        self.current_build = None;

        &self.last_result
    }
}

impl Default for BuildManager {
    fn default() -> Self {
        Self::new()
    }
}
