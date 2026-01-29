// SPDX-License-Identifier: MIT OR Apache-2.0
//! Project settings and configuration.
//!
//! This module manages project-level settings including:
//! - Project metadata (name, version, company)
//! - Build configuration (platforms, quality settings)
//! - Scene management (startup scene, build scene list)
//! - Input settings
//! - Physics settings
//! - Audio settings


use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Current project settings format version
pub const PROJECT_FORMAT_VERSION: u32 = 1;

/// Project settings file name
pub const PROJECT_FILE_NAME: &str = "project.ordoplay";

/// Target platform for builds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum TargetPlatform {
    Windows,
    Linux,
    MacOS,
    WebGL,
    Android,
    #[serde(rename = "IOS")]
    IOS,
}

impl TargetPlatform {
    /// Get display name for this platform
    pub fn display_name(&self) -> &'static str {
        match self {
            TargetPlatform::Windows => "Windows",
            TargetPlatform::Linux => "Linux",
            TargetPlatform::MacOS => "macOS",
            TargetPlatform::WebGL => "WebGL",
            TargetPlatform::Android => "Android",
            TargetPlatform::IOS => "iOS",
        }
    }

    /// Get all available platforms
    pub fn all() -> &'static [TargetPlatform] {
        &[
            TargetPlatform::Windows,
            TargetPlatform::Linux,
            TargetPlatform::MacOS,
            TargetPlatform::WebGL,
            TargetPlatform::Android,
            TargetPlatform::IOS,
        ]
    }
}

impl Default for TargetPlatform {
    fn default() -> Self {
        #[cfg(target_os = "windows")]
        return TargetPlatform::Windows;
        #[cfg(target_os = "linux")]
        return TargetPlatform::Linux;
        #[cfg(target_os = "macos")]
        return TargetPlatform::MacOS;
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        return TargetPlatform::Windows;
    }
}

/// Build configuration (debug or release)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BuildConfiguration {
    #[default]
    Debug,
    Release,
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl BuildConfiguration {
    pub fn display_name(&self) -> &'static str {
        match self {
            BuildConfiguration::Debug => "Debug",
            BuildConfiguration::Release => "Release",
        }
    }
}

/// Quality level preset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum QualityLevel {
    Low,
    #[default]
    Medium,
    High,
    Ultra,
}

impl QualityLevel {
    pub fn display_name(&self) -> &'static str {
        match self {
            QualityLevel::Low => "Low",
            QualityLevel::Medium => "Medium",
            QualityLevel::High => "High",
            QualityLevel::Ultra => "Ultra",
        }
    }

    pub fn all() -> &'static [QualityLevel] {
        &[
            QualityLevel::Low,
            QualityLevel::Medium,
            QualityLevel::High,
            QualityLevel::Ultra,
        ]
    }
}

/// Texture compression format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TextureCompression {
    None,
    #[default]
    BC,      // Desktop (DXT/BC)
    ASTC,    // Mobile
    ETC2,    // Android/WebGL
}

/// Project metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    /// Project name
    pub name: String,
    /// Project version (semantic versioning)
    pub version: String,
    /// Company/developer name
    pub company: String,
    /// Project description
    #[serde(default)]
    pub description: String,
    /// Project icon path (relative to project root)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<PathBuf>,
}

impl Default for ProjectMetadata {
    fn default() -> Self {
        Self {
            name: "My Game".to_string(),
            version: "0.1.0".to_string(),
            company: "".to_string(),
            description: "".to_string(),
            icon: None,
        }
    }
}

/// Build settings for a specific platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformBuildSettings {
    /// Whether this platform is enabled for building
    pub enabled: bool,
    /// Output directory (relative to project root)
    pub output_dir: PathBuf,
    /// Texture compression format
    pub texture_compression: TextureCompression,
    /// Whether to compress assets
    pub compress_assets: bool,
    /// Whether to include debug symbols
    pub include_debug_symbols: bool,
    /// Custom defines for this platform
    #[serde(default)]
    pub defines: Vec<String>,
}

impl Default for PlatformBuildSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            output_dir: PathBuf::from("Build"),
            texture_compression: TextureCompression::default(),
            compress_assets: true,
            include_debug_symbols: false,
            defines: Vec::new(),
        }
    }
}

/// Scene entry in the build list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildSceneEntry {
    /// Path to the scene file (relative to project root)
    pub path: PathBuf,
    /// Whether to include this scene in the build
    pub enabled: bool,
}

/// Scene management settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SceneSettings {
    /// Startup scene (first scene loaded)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub startup_scene: Option<PathBuf>,
    /// Scenes to include in the build (in order)
    #[serde(default)]
    pub build_scenes: Vec<BuildSceneEntry>,
}

/// Collision layer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionLayerSettings {
    /// Names for each layer
    pub layer_names: Vec<String>,
    /// Collision matrix - layer_matrix[i][j] indicates if layer i collides with layer j
    pub layer_matrix: Vec<Vec<bool>>,
}

impl Default for CollisionLayerSettings {
    fn default() -> Self {
        Self::new(8)
    }
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl CollisionLayerSettings {
    pub fn new(num_layers: usize) -> Self {
        let layer_names: Vec<String> = (0..num_layers)
            .map(|i| match i {
                0 => "Default".to_string(),
                1 => "Player".to_string(),
                2 => "Enemy".to_string(),
                3 => "Environment".to_string(),
                4 => "Projectile".to_string(),
                5 => "Trigger".to_string(),
                _ => format!("Layer {}", i),
            })
            .collect();

        // All layers collide with all layers by default
        let layer_matrix = vec![vec![true; num_layers]; num_layers];

        Self {
            layer_names,
            layer_matrix,
        }
    }

    pub fn should_collide(&self, layer_a: u32, layer_b: u32) -> bool {
        let a = layer_a as usize;
        let b = layer_b as usize;
        if a < self.layer_matrix.len() && b < self.layer_matrix.len() {
            self.layer_matrix[a][b]
        } else {
            true
        }
    }

    pub fn set_collision(&mut self, layer_a: u32, layer_b: u32, collides: bool) {
        let a = layer_a as usize;
        let b = layer_b as usize;
        if a < self.layer_matrix.len() && b < self.layer_matrix.len() {
            self.layer_matrix[a][b] = collides;
            self.layer_matrix[b][a] = collides; // Symmetric
        }
    }
}

/// Physics engine settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsSettings {
    /// Gravity vector
    pub gravity: [f32; 3],
    /// Default physics material friction
    pub default_friction: f32,
    /// Default physics material bounciness
    pub default_bounciness: f32,
    /// Fixed timestep for physics simulation
    pub fixed_timestep: f32,
    /// Maximum substeps per frame
    pub max_substeps: u32,
    /// Enable continuous collision detection
    pub continuous_collision: bool,
    /// Collision layer configuration
    pub collision_layers: CollisionLayerSettings,
}

impl Default for PhysicsSettings {
    fn default() -> Self {
        Self {
            gravity: [0.0, -9.81, 0.0],
            default_friction: 0.5,
            default_bounciness: 0.0,
            fixed_timestep: 1.0 / 60.0,
            max_substeps: 8,
            continuous_collision: true,
            collision_layers: CollisionLayerSettings::default(),
        }
    }
}

/// Audio settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSettings {
    /// Master volume (0-1)
    pub master_volume: f32,
    /// Music volume (0-1)
    pub music_volume: f32,
    /// Sound effects volume (0-1)
    pub sfx_volume: f32,
    /// Maximum simultaneous audio sources
    pub max_audio_sources: u32,
    /// Doppler scale for 3D audio
    pub doppler_scale: f32,
    /// Speed of sound (for doppler calculations)
    pub speed_of_sound: f32,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            music_volume: 1.0,
            sfx_volume: 1.0,
            max_audio_sources: 32,
            doppler_scale: 1.0,
            speed_of_sound: 343.0,
        }
    }
}

/// Graphics/rendering settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicsSettings {
    /// Default quality level
    pub default_quality: QualityLevel,
    /// VSync enabled by default
    pub vsync: bool,
    /// Target frame rate (0 = unlimited)
    pub target_frame_rate: u32,
    /// Default resolution width
    pub default_width: u32,
    /// Default resolution height
    pub default_height: u32,
    /// Fullscreen by default
    pub fullscreen: bool,
    /// Allow resolution change
    pub resizable: bool,
    /// Anti-aliasing samples
    pub msaa_samples: u32,
    /// Shadow quality (0-4)
    pub shadow_quality: u32,
    /// Shadow distance
    pub shadow_distance: f32,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            default_quality: QualityLevel::Medium,
            vsync: true,
            target_frame_rate: 60,
            default_width: 1920,
            default_height: 1080,
            fullscreen: false,
            resizable: true,
            msaa_samples: 4,
            shadow_quality: 2,
            shadow_distance: 150.0,
        }
    }
}

/// Input axis definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputAxis {
    /// Axis name
    pub name: String,
    /// Positive button (keyboard key name)
    #[serde(default)]
    pub positive_button: String,
    /// Negative button (keyboard key name)
    #[serde(default)]
    pub negative_button: String,
    /// Alternative positive button
    #[serde(default)]
    pub alt_positive_button: String,
    /// Alternative negative button
    #[serde(default)]
    pub alt_negative_button: String,
    /// Gravity (how fast the axis returns to 0)
    pub gravity: f32,
    /// Sensitivity
    pub sensitivity: f32,
    /// Dead zone
    pub dead_zone: f32,
    /// Whether to snap to 0 when opposite direction is pressed
    pub snap: bool,
    /// Input type (keyboard, mouse, joystick)
    pub input_type: InputType,
    /// Axis number (for joystick)
    pub axis: u32,
    /// Joystick number
    pub joystick: u32,
}

impl Default for InputAxis {
    fn default() -> Self {
        Self {
            name: "Horizontal".to_string(),
            positive_button: "d".to_string(),
            negative_button: "a".to_string(),
            alt_positive_button: "Right".to_string(),
            alt_negative_button: "Left".to_string(),
            gravity: 3.0,
            sensitivity: 3.0,
            dead_zone: 0.001,
            snap: true,
            input_type: InputType::Keyboard,
            axis: 0,
            joystick: 0,
        }
    }
}

/// Input type for axes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum InputType {
    #[default]
    Keyboard,
    Mouse,
    Joystick,
}

/// Input settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputSettings {
    /// Input axes
    pub axes: Vec<InputAxis>,
}

impl Default for InputSettings {
    fn default() -> Self {
        Self {
            axes: vec![
                InputAxis {
                    name: "Horizontal".to_string(),
                    positive_button: "d".to_string(),
                    negative_button: "a".to_string(),
                    alt_positive_button: "Right".to_string(),
                    alt_negative_button: "Left".to_string(),
                    ..Default::default()
                },
                InputAxis {
                    name: "Vertical".to_string(),
                    positive_button: "w".to_string(),
                    negative_button: "s".to_string(),
                    alt_positive_button: "Up".to_string(),
                    alt_negative_button: "Down".to_string(),
                    ..Default::default()
                },
                InputAxis {
                    name: "Fire1".to_string(),
                    positive_button: "Space".to_string(),
                    negative_button: "".to_string(),
                    ..Default::default()
                },
                InputAxis {
                    name: "Fire2".to_string(),
                    positive_button: "LeftCtrl".to_string(),
                    negative_button: "".to_string(),
                    ..Default::default()
                },
                InputAxis {
                    name: "Jump".to_string(),
                    positive_button: "Space".to_string(),
                    negative_button: "".to_string(),
                    ..Default::default()
                },
                InputAxis {
                    name: "Mouse X".to_string(),
                    input_type: InputType::Mouse,
                    axis: 0,
                    sensitivity: 0.1,
                    ..Default::default()
                },
                InputAxis {
                    name: "Mouse Y".to_string(),
                    input_type: InputType::Mouse,
                    axis: 1,
                    sensitivity: 0.1,
                    ..Default::default()
                },
            ],
        }
    }
}

/// Complete project settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    /// Settings format version
    pub version: u32,
    /// Project metadata
    pub metadata: ProjectMetadata,
    /// Scene settings
    pub scenes: SceneSettings,
    /// Physics settings
    pub physics: PhysicsSettings,
    /// Audio settings
    pub audio: AudioSettings,
    /// Graphics settings
    pub graphics: GraphicsSettings,
    /// Input settings
    pub input: InputSettings,
    /// Platform-specific build settings
    #[serde(default)]
    pub platform_settings: HashMap<TargetPlatform, PlatformBuildSettings>,
    /// Current build configuration
    #[serde(default)]
    pub build_configuration: BuildConfiguration,
    /// Current target platform
    #[serde(default)]
    pub target_platform: TargetPlatform,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        let mut platform_settings = HashMap::new();
        for platform in TargetPlatform::all() {
            platform_settings.insert(*platform, PlatformBuildSettings::default());
        }

        Self {
            version: PROJECT_FORMAT_VERSION,
            metadata: ProjectMetadata::default(),
            scenes: SceneSettings::default(),
            physics: PhysicsSettings::default(),
            audio: AudioSettings::default(),
            graphics: GraphicsSettings::default(),
            input: InputSettings::default(),
            platform_settings,
            build_configuration: BuildConfiguration::default(),
            target_platform: TargetPlatform::default(),
        }
    }
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl ProjectSettings {
    /// Create new project settings with the given name
    pub fn new(name: impl Into<String>) -> Self {
        let mut settings = Self::default();
        settings.metadata.name = name.into();
        settings
    }

    /// Load project settings from a file
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let settings: ProjectSettings = ron::from_str(&content).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })?;

        // Version check
        if settings.version > PROJECT_FORMAT_VERSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Project version {} is newer than supported version {}",
                    settings.version, PROJECT_FORMAT_VERSION
                ),
            ));
        }

        Ok(settings)
    }

    /// Save project settings to a file
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let config = ron::ser::PrettyConfig::default()
            .struct_names(true)
            .enumerate_arrays(false);

        let content = ron::ser::to_string_pretty(self, config).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })?;

        std::fs::write(path, content)
    }

    /// Get the project file path for a project directory
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn project_file_path(project_dir: &Path) -> PathBuf {
        project_dir.join(PROJECT_FILE_NAME)
    }

    /// Check if a directory contains a valid project
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn is_project_directory(dir: &Path) -> bool {
        Self::project_file_path(dir).exists()
    }

    /// Get platform-specific build settings
    pub fn get_platform_settings(&self, platform: TargetPlatform) -> &PlatformBuildSettings {
        self.platform_settings
            .get(&platform)
            .unwrap_or_else(|| {
                static DEFAULT: PlatformBuildSettings = PlatformBuildSettings {
                    enabled: true,
                    output_dir: PathBuf::new(),
                    texture_compression: TextureCompression::BC,
                    compress_assets: true,
                    include_debug_symbols: false,
                    defines: Vec::new(),
                };
                &DEFAULT
            })
    }

    /// Get mutable platform-specific build settings
    pub fn get_platform_settings_mut(&mut self, platform: TargetPlatform) -> &mut PlatformBuildSettings {
        self.platform_settings
            .entry(platform)
            .or_insert_with(PlatformBuildSettings::default)
    }

    /// Add a scene to the build list
    pub fn add_build_scene(&mut self, path: PathBuf) {
        // Don't add duplicates
        if self.scenes.build_scenes.iter().any(|s| s.path == path) {
            return;
        }

        self.scenes.build_scenes.push(BuildSceneEntry {
            path,
            enabled: true,
        });
    }

    /// Remove a scene from the build list
    pub fn remove_build_scene(&mut self, path: &Path) {
        self.scenes.build_scenes.retain(|s| s.path != path);
    }

    /// Move a scene up in the build order
    pub fn move_scene_up(&mut self, index: usize) {
        if index > 0 && index < self.scenes.build_scenes.len() {
            self.scenes.build_scenes.swap(index, index - 1);
        }
    }

    /// Move a scene down in the build order
    pub fn move_scene_down(&mut self, index: usize) {
        if index < self.scenes.build_scenes.len().saturating_sub(1) {
            self.scenes.build_scenes.swap(index, index + 1);
        }
    }

    /// Set the startup scene
    pub fn set_startup_scene(&mut self, path: Option<PathBuf>) {
        self.scenes.startup_scene = path;
    }
}

/// Project manager for handling project lifecycle
pub struct ProjectManager {
    /// Current project directory
    pub project_dir: Option<PathBuf>,
    /// Current project settings
    pub settings: ProjectSettings,
    /// Whether settings have been modified
    pub dirty: bool,
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl ProjectManager {
    /// Create a new project manager
    pub fn new() -> Self {
        Self {
            project_dir: None,
            settings: ProjectSettings::default(),
            dirty: false,
        }
    }

    /// Check if a project is currently open
    pub fn is_project_open(&self) -> bool {
        self.project_dir.is_some()
    }

    /// Get the project name
    pub fn project_name(&self) -> &str {
        &self.settings.metadata.name
    }

    /// Open an existing project
    pub fn open_project(&mut self, project_dir: &Path) -> std::io::Result<()> {
        let settings_path = ProjectSettings::project_file_path(project_dir);
        let settings = ProjectSettings::load(&settings_path)?;

        self.project_dir = Some(project_dir.to_path_buf());
        self.settings = settings;
        self.dirty = false;

        tracing::info!("Opened project: {} at {:?}", self.settings.metadata.name, project_dir);
        Ok(())
    }

    /// Create a new project
    pub fn create_project(&mut self, project_dir: &Path, name: &str) -> std::io::Result<()> {
        // Create project directory if it doesn't exist
        std::fs::create_dir_all(project_dir)?;

        // Create default subdirectories
        let subdirs = ["Assets", "Scenes", "Build", "Library"];
        for subdir in &subdirs {
            std::fs::create_dir_all(project_dir.join(subdir))?;
        }

        // Create project settings
        let settings = ProjectSettings::new(name);
        let settings_path = ProjectSettings::project_file_path(project_dir);
        settings.save(&settings_path)?;

        self.project_dir = Some(project_dir.to_path_buf());
        self.settings = settings;
        self.dirty = false;

        tracing::info!("Created new project: {} at {:?}", name, project_dir);
        Ok(())
    }

    /// Save current project settings
    pub fn save_project(&mut self) -> std::io::Result<()> {
        let Some(project_dir) = &self.project_dir else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No project is open",
            ));
        };

        let settings_path = ProjectSettings::project_file_path(project_dir);
        self.settings.save(&settings_path)?;
        self.dirty = false;

        tracing::info!("Saved project settings");
        Ok(())
    }

    /// Close the current project
    pub fn close_project(&mut self) {
        self.project_dir = None;
        self.settings = ProjectSettings::default();
        self.dirty = false;
    }

    /// Mark settings as modified
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Check if settings have unsaved changes
    pub fn has_unsaved_changes(&self) -> bool {
        self.dirty
    }

    /// Get the assets directory path
    pub fn assets_dir(&self) -> Option<PathBuf> {
        self.project_dir.as_ref().map(|p| p.join("Assets"))
    }

    /// Get the scenes directory path
    pub fn scenes_dir(&self) -> Option<PathBuf> {
        self.project_dir.as_ref().map(|p| p.join("Scenes"))
    }

    /// Get the build output directory path
    pub fn build_dir(&self) -> Option<PathBuf> {
        self.project_dir.as_ref().map(|p| p.join("Build"))
    }

    /// Get the library (cache) directory path
    pub fn library_dir(&self) -> Option<PathBuf> {
        self.project_dir.as_ref().map(|p| p.join("Library"))
    }
}

impl Default for ProjectManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = ProjectSettings::default();
        assert_eq!(settings.version, PROJECT_FORMAT_VERSION);
        assert_eq!(settings.metadata.name, "My Game");
        assert!(!settings.platform_settings.is_empty());
    }

    #[test]
    fn test_serialization() {
        let settings = ProjectSettings::new("Test Project");
        let ron_str = ron::ser::to_string_pretty(&settings, ron::ser::PrettyConfig::default()).unwrap();
        let loaded: ProjectSettings = ron::from_str(&ron_str).unwrap();
        assert_eq!(loaded.metadata.name, "Test Project");
    }

    #[test]
    fn test_scene_management() {
        let mut settings = ProjectSettings::default();

        settings.add_build_scene(PathBuf::from("Scenes/Level1.scene"));
        settings.add_build_scene(PathBuf::from("Scenes/Level2.scene"));

        assert_eq!(settings.scenes.build_scenes.len(), 2);

        // Don't add duplicates
        settings.add_build_scene(PathBuf::from("Scenes/Level1.scene"));
        assert_eq!(settings.scenes.build_scenes.len(), 2);

        settings.remove_build_scene(Path::new("Scenes/Level1.scene"));
        assert_eq!(settings.scenes.build_scenes.len(), 1);
    }
}
