// SPDX-License-Identifier: MIT OR Apache-2.0
//! Asset browser panel - File/asset navigation.


use crate::file_watcher::{FileEvent, FileWatcherManager};
use crate::panel_types::PanelType;
use crate::state::EditorState;
use crate::thumbnail::{ThumbnailManager, ThumbnailState};
use egui_wgpu::wgpu;
use std::collections::HashSet;
use std::path::PathBuf;

/// View mode for assets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetViewMode {
    /// Grid view with thumbnails
    Grid,
    /// List view with details
    List,
}

/// Asset type for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetType {
    All,
    Mesh,
    Texture,
    Material,
    Audio,
    Scene,
    Prefab,
    Script,
    Shader,
    Font,
    Animation,
    Unknown,
}

impl AssetType {
    fn name(&self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Mesh => "Meshes",
            Self::Texture => "Textures",
            Self::Material => "Materials",
            Self::Audio => "Audio",
            Self::Scene => "Scenes",
            Self::Prefab => "Prefabs",
            Self::Script => "Scripts",
            Self::Shader => "Shaders",
            Self::Font => "Fonts",
            Self::Animation => "Animations",
            Self::Unknown => "Unknown",
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            Self::All => "\u{f07c}",      // folder-open
            Self::Mesh => "\u{f1b2}",     // cube
            Self::Texture => "\u{f03e}",  // image
            Self::Material => "\u{f5aa}", // palette
            Self::Audio => "\u{f001}",    // music
            Self::Scene => "\u{f03d}",    // film
            Self::Prefab => "\u{f1b3}",   // cubes
            Self::Script => "\u{f121}",   // code
            Self::Shader => "\u{f0eb}",   // lightbulb
            Self::Font => "\u{f031}",     // font
            Self::Animation => "\u{f008}",// film
            Self::Unknown => "\u{f15b}",  // file
        }
    }

    fn color(&self) -> egui::Color32 {
        match self {
            Self::All => egui::Color32::from_rgb(180, 180, 180),
            Self::Mesh => egui::Color32::from_rgb(100, 200, 255),
            Self::Texture => egui::Color32::from_rgb(255, 150, 100),
            Self::Material => egui::Color32::from_rgb(200, 100, 255),
            Self::Audio => egui::Color32::from_rgb(100, 255, 150),
            Self::Scene => egui::Color32::from_rgb(255, 200, 100),
            Self::Prefab => egui::Color32::from_rgb(100, 200, 200),
            Self::Script => egui::Color32::from_rgb(255, 255, 100),
            Self::Shader => egui::Color32::from_rgb(150, 200, 255),
            Self::Font => egui::Color32::from_rgb(200, 200, 200),
            Self::Animation => egui::Color32::from_rgb(255, 150, 200),
            Self::Unknown => egui::Color32::from_rgb(150, 150, 150),
        }
    }

    /// Detect asset type from file extension
    fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            // Meshes
            "glb" | "gltf" | "obj" | "fbx" | "dae" => Self::Mesh,
            // Textures
            "png" | "jpg" | "jpeg" | "bmp" | "tga" | "dds" | "ktx2" | "exr" | "hdr" => Self::Texture,
            // Materials
            "mat" | "material" => Self::Material,
            // Audio
            "wav" | "mp3" | "ogg" | "flac" => Self::Audio,
            // Scenes
            "scene" | "ron" => Self::Scene,
            // Prefabs
            "prefab" => Self::Prefab,
            // Scripts
            "rs" | "lua" | "wasm" => Self::Script,
            // Shaders
            "wgsl" | "glsl" | "hlsl" | "spv" => Self::Shader,
            // Fonts
            "ttf" | "otf" | "woff" | "woff2" => Self::Font,
            // Animations
            "anim" | "animation" => Self::Animation,
            // Unknown
            _ => Self::Unknown,
        }
    }
}

/// Directory tree entry
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    pub name: String,
    pub path: PathBuf,
    pub children: Vec<DirectoryEntry>,
    pub expanded: bool,
}

/// Mock asset entry
#[derive(Debug, Clone)]
pub struct AssetEntry {
    pub name: String,
    pub path: PathBuf,
    pub asset_type: AssetType,
    pub is_folder: bool,
}

/// Sorting options for assets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortMode {
    #[default]
    Name,
    DateModified,
    Type,
    Size,
}

impl SortMode {
    fn name(&self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::DateModified => "Date Modified",
            Self::Type => "Type",
            Self::Size => "Size",
        }
    }
}

/// The asset browser panel
#[allow(dead_code)] // Intentionally kept for API completeness
pub struct AssetBrowserPanel {
    /// Root assets directory
    pub root_path: PathBuf,
    /// Current directory path
    pub current_path: PathBuf,
    /// View mode
    pub view_mode: AssetViewMode,
    /// Asset type filter
    pub filter: AssetType,
    /// Search query
    pub search: String,
    /// Search field has focus
    search_focused: bool,
    /// Selected assets
    pub selected: Vec<PathBuf>,
    /// Assets in current directory (from filesystem)
    assets: Vec<AssetEntry>,
    /// Directory tree
    directory_tree: Vec<DirectoryEntry>,
    /// Expanded directories in tree
    expanded_dirs: HashSet<PathBuf>,
    /// Grid icon size
    pub icon_size: f32,
    /// Show directory tree panel
    pub show_tree: bool,
    /// Tree panel width
    pub tree_width: f32,
    /// Navigation history
    history: Vec<PathBuf>,
    /// Current history index
    history_index: usize,
    /// Pending asset to drag (for drag-drop)
    pub dragging_asset: Option<PathBuf>,
    /// Thumbnail manager for image previews
    pub thumbnail_manager: ThumbnailManager,
    /// Whether to show thumbnails (vs icons)
    pub show_thumbnails: bool,
    /// Sorting mode
    pub sort_mode: SortMode,
    /// Sort ascending
    pub sort_ascending: bool,
    /// Favorite paths (bookmarks)
    pub favorites: Vec<PathBuf>,
    /// Recent assets
    pub recent_assets: Vec<PathBuf>,
    /// Maximum recent assets to track
    max_recent: usize,
    /// Whether filesystem needs refresh
    needs_refresh: bool,
    /// Last refresh time
    last_refresh: std::time::Instant,
    /// File watcher for auto-refresh
    file_watcher: Option<FileWatcherManager>,
    /// Paths that were modified and need attention
    modified_paths: HashSet<PathBuf>,
    /// Path currently being renamed (inline rename mode)
    renaming_path: Option<PathBuf>,
    /// Buffer for the new name during inline rename
    rename_buffer: String,
    /// Whether the rename text field should request focus
    rename_focus_request: bool,
    /// Path pending deletion confirmation
    pending_delete: Option<PathBuf>,
}

impl AssetBrowserPanel {
    /// Create a new asset browser panel
    pub fn new() -> Self {
        let root = PathBuf::from("assets");

        // Initialize file watcher
        let file_watcher = Self::create_file_watcher(&root);

        let mut panel = Self {
            root_path: root.clone(),
            current_path: root.clone(),
            view_mode: AssetViewMode::Grid,
            filter: AssetType::All,
            search: String::new(),
            search_focused: false,
            selected: Vec::new(),
            assets: Vec::new(),
            directory_tree: Vec::new(),
            expanded_dirs: HashSet::new(),
            icon_size: 64.0,
            show_tree: true,
            tree_width: 180.0,
            history: vec![root.clone()],
            history_index: 0,
            dragging_asset: None,
            thumbnail_manager: ThumbnailManager::new(),
            show_thumbnails: true,
            sort_mode: SortMode::Name,
            sort_ascending: true,
            favorites: Vec::new(),
            recent_assets: Vec::new(),
            max_recent: 20,
            needs_refresh: true,
            last_refresh: std::time::Instant::now(),
            file_watcher,
            modified_paths: HashSet::new(),
            renaming_path: None,
            rename_buffer: String::new(),
            rename_focus_request: false,
            pending_delete: None,
        };

        panel.expanded_dirs.insert(root);
        panel.refresh_filesystem();
        panel
    }

    /// Create a file watcher for the given root path
    fn create_file_watcher(root: &PathBuf) -> Option<FileWatcherManager> {
        if !root.exists() {
            return None;
        }

        let mut manager = FileWatcherManager::new();
        match manager.watch_directory(root) {
            Ok(()) => {
                tracing::info!("Asset file watcher started for: {:?}", root);
                Some(manager)
            }
            Err(e) => {
                tracing::warn!("Failed to start file watcher: {}", e);
                None
            }
        }
    }

    /// Poll file watcher for changes and trigger refresh if needed
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn poll_file_watcher(&mut self) {
        let Some(ref mut watcher) = self.file_watcher else {
            return;
        };

        let events = watcher.poll();
        if events.is_empty() {
            return;
        }

        // Process events
        let mut needs_tree_refresh = false;
        let mut needs_current_refresh = false;

        for event in events {
            match event {
                FileEvent::Created(path) | FileEvent::Modified(path) | FileEvent::Deleted(path) => {
                    self.modified_paths.insert(path.clone());

                    // Check if this affects the current directory
                    if let Some(parent) = path.parent() {
                        if parent == self.current_path {
                            needs_current_refresh = true;
                        }
                    }

                    // Check if it's a directory change (affects tree)
                    if path.is_dir() || !path.exists() {
                        needs_tree_refresh = true;
                    }
                }
                FileEvent::Renamed(old, new) => {
                    self.modified_paths.insert(old.clone());
                    self.modified_paths.insert(new.clone());
                    needs_tree_refresh = true;
                    needs_current_refresh = true;
                }
                FileEvent::Error(msg) => {
                    tracing::warn!("File watcher error: {}", msg);
                }
            }
        }

        // Refresh as needed
        if needs_tree_refresh {
            self.scan_directory_tree();
        }
        if needs_current_refresh {
            self.scan_current_directory();
        }

        // Clear modified paths after processing
        if !self.modified_paths.is_empty() {
            tracing::debug!(
                "File watcher detected {} modifications",
                self.modified_paths.len()
            );
        }
    }

    /// Get paths that were recently modified (for hot reload)
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn take_modified_paths(&mut self) -> HashSet<PathBuf> {
        std::mem::take(&mut self.modified_paths)
    }

    /// Check if file watcher is active
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn has_file_watcher(&self) -> bool {
        self.file_watcher.is_some()
    }

    /// Refresh the filesystem view
    pub fn refresh_filesystem(&mut self) {
        self.scan_directory_tree();
        self.scan_current_directory();
        self.needs_refresh = false;
        self.last_refresh = std::time::Instant::now();
    }

    /// Scan the root directory to build the tree
    fn scan_directory_tree(&mut self) {
        self.directory_tree = if self.root_path.exists() {
            vec![self.scan_directory_recursive(&self.root_path.clone(), 0)]
        } else {
            // Fall back to mock tree if assets folder doesn't exist
            self.build_mock_tree_data()
        };
    }

    fn scan_directory_recursive(&self, path: &PathBuf, depth: usize) -> DirectoryEntry {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());

        let mut children = Vec::new();

        // Limit recursion depth to prevent performance issues
        if depth < 5 {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let entry_path = entry.path();
                    if entry_path.is_dir() {
                        // Skip hidden directories
                        let dir_name = entry_path.file_name().unwrap_or_default().to_string_lossy();
                        if !dir_name.starts_with('.') {
                            children.push(self.scan_directory_recursive(&entry_path, depth + 1));
                        }
                    }
                }
            }
        }

        // Sort children by name
        children.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        DirectoryEntry {
            name,
            path: path.clone(),
            children,
            expanded: self.expanded_dirs.contains(path),
        }
    }

    /// Scan the current directory for assets
    fn scan_current_directory(&mut self) {
        self.assets.clear();

        if !self.current_path.exists() {
            // Fall back to mock assets
            self.add_mock_assets();
            return;
        }

        if let Ok(entries) = std::fs::read_dir(&self.current_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                // Skip hidden files
                if name.starts_with('.') {
                    continue;
                }

                let is_folder = path.is_dir();
                let asset_type = if is_folder {
                    AssetType::All
                } else {
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    AssetType::from_extension(ext)
                };

                self.assets.push(AssetEntry {
                    name,
                    path,
                    asset_type,
                    is_folder,
                });
            }
        }

        // Sort assets
        self.sort_assets();
    }

    /// Sort assets according to current sort mode
    fn sort_assets(&mut self) {
        self.assets.sort_by(|a, b| {
            // Folders always first
            if a.is_folder != b.is_folder {
                return if a.is_folder {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                };
            }

            let cmp = match self.sort_mode {
                SortMode::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortMode::Type => {
                    let type_cmp = a.asset_type.name().cmp(b.asset_type.name());
                    if type_cmp == std::cmp::Ordering::Equal {
                        a.name.to_lowercase().cmp(&b.name.to_lowercase())
                    } else {
                        type_cmp
                    }
                }
                SortMode::DateModified => {
                    // Get modification times
                    let a_time = std::fs::metadata(&a.path)
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    let b_time = std::fs::metadata(&b.path)
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    a_time.cmp(&b_time)
                }
                SortMode::Size => {
                    let a_size = std::fs::metadata(&a.path).map(|m| m.len()).unwrap_or(0);
                    let b_size = std::fs::metadata(&b.path).map(|m| m.len()).unwrap_or(0);
                    a_size.cmp(&b_size)
                }
            };

            if self.sort_ascending {
                cmp
            } else {
                cmp.reverse()
            }
        });
    }

    /// Add path to recent assets
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn add_to_recent(&mut self, path: PathBuf) {
        // Remove if already exists
        self.recent_assets.retain(|p| p != &path);
        // Add to front
        self.recent_assets.insert(0, path);
        // Trim to max size
        self.recent_assets.truncate(self.max_recent);
    }

    /// Toggle favorite status for a path
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn toggle_favorite(&mut self, path: PathBuf) {
        if self.favorites.contains(&path) {
            self.favorites.retain(|p| p != &path);
        } else {
            self.favorites.push(path);
        }
    }

    /// Check if path is favorited
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn is_favorite(&self, path: &PathBuf) -> bool {
        self.favorites.contains(path)
    }

    fn build_mock_tree_data(&self) -> Vec<DirectoryEntry> {
        vec![
            DirectoryEntry {
                name: "assets".to_string(),
                path: PathBuf::from("assets"),
                expanded: true,
                children: vec![
                    DirectoryEntry {
                        name: "models".to_string(),
                        path: PathBuf::from("assets/models"),
                        expanded: false,
                        children: vec![],
                    },
                    DirectoryEntry {
                        name: "textures".to_string(),
                        path: PathBuf::from("assets/textures"),
                        expanded: false,
                        children: vec![],
                    },
                    DirectoryEntry {
                        name: "materials".to_string(),
                        path: PathBuf::from("assets/materials"),
                        expanded: false,
                        children: vec![],
                    },
                    DirectoryEntry {
                        name: "scenes".to_string(),
                        path: PathBuf::from("assets/scenes"),
                        expanded: false,
                        children: vec![],
                    },
                ],
            },
        ]
    }

    /// Update thumbnail manager with graphics context
    /// Call this each frame before rendering
    pub fn update_thumbnails(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        egui_renderer: &mut egui_wgpu::Renderer,
    ) {
        self.thumbnail_manager.update(device, queue, egui_renderer);
    }

    fn add_mock_assets(&mut self) {
        self.assets = vec![
            AssetEntry {
                name: "models".to_string(),
                path: PathBuf::from("assets/models"),
                asset_type: AssetType::All,
                is_folder: true,
            },
            AssetEntry {
                name: "textures".to_string(),
                path: PathBuf::from("assets/textures"),
                asset_type: AssetType::All,
                is_folder: true,
            },
            AssetEntry {
                name: "materials".to_string(),
                path: PathBuf::from("assets/materials"),
                asset_type: AssetType::All,
                is_folder: true,
            },
            AssetEntry {
                name: "shaders".to_string(),
                path: PathBuf::from("assets/shaders"),
                asset_type: AssetType::All,
                is_folder: true,
            },
            AssetEntry {
                name: "scenes".to_string(),
                path: PathBuf::from("assets/scenes"),
                asset_type: AssetType::All,
                is_folder: true,
            },
            AssetEntry {
                name: "audio".to_string(),
                path: PathBuf::from("assets/audio"),
                asset_type: AssetType::All,
                is_folder: true,
            },
            AssetEntry {
                name: "player.glb".to_string(),
                path: PathBuf::from("assets/player.glb"),
                asset_type: AssetType::Mesh,
                is_folder: false,
            },
            AssetEntry {
                name: "environment.glb".to_string(),
                path: PathBuf::from("assets/environment.glb"),
                asset_type: AssetType::Mesh,
                is_folder: false,
            },
            AssetEntry {
                name: "albedo.png".to_string(),
                path: PathBuf::from("assets/albedo.png"),
                asset_type: AssetType::Texture,
                is_folder: false,
            },
            AssetEntry {
                name: "normal.png".to_string(),
                path: PathBuf::from("assets/normal.png"),
                asset_type: AssetType::Texture,
                is_folder: false,
            },
            AssetEntry {
                name: "default_material.mat".to_string(),
                path: PathBuf::from("assets/default_material.mat"),
                asset_type: AssetType::Material,
                is_folder: false,
            },
            AssetEntry {
                name: "main.scene".to_string(),
                path: PathBuf::from("assets/main.scene"),
                asset_type: AssetType::Scene,
                is_folder: false,
            },
            AssetEntry {
                name: "pbr_shader.wgsl".to_string(),
                path: PathBuf::from("assets/pbr_shader.wgsl"),
                asset_type: AssetType::Shader,
                is_folder: false,
            },
        ];
    }

    /// Navigate to a directory
    pub fn navigate_to(&mut self, path: PathBuf) {
        if path != self.current_path {
            // Add to history
            self.history_index += 1;
            self.history.truncate(self.history_index);
            self.history.push(path.clone());
            self.current_path = path;
            // Refresh the current directory contents
            self.scan_current_directory();
        }
    }

    /// Go back in history
    pub fn go_back(&mut self) {
        if self.history_index > 0 {
            self.history_index -= 1;
            self.current_path = self.history[self.history_index].clone();
            self.scan_current_directory();
        }
    }

    /// Go forward in history
    pub fn go_forward(&mut self) {
        if self.history_index < self.history.len() - 1 {
            self.history_index += 1;
            self.current_path = self.history[self.history_index].clone();
            self.scan_current_directory();
        }
    }

    /// Check if can go back
    pub fn can_go_back(&self) -> bool {
        self.history_index > 0
    }

    /// Check if can go forward
    pub fn can_go_forward(&self) -> bool {
        self.history_index < self.history.len() - 1
    }

    /// Render the asset browser panel
    pub fn ui(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        // Handle keyboard shortcuts
        let ctrl_f_pressed = ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::F));
        let f5_pressed = ui.input(|i| i.key_pressed(egui::Key::F5));

        if ctrl_f_pressed {
            self.search_focused = true;
        }

        if f5_pressed {
            self.refresh_filesystem();
        }

        // Toolbar
        ui.horizontal(|ui| {
            // Navigation buttons
            ui.add_enabled_ui(self.can_go_back(), |ui| {
                if ui.button("<").on_hover_text("Back (Alt+Left)").clicked() {
                    self.go_back();
                }
            });
            ui.add_enabled_ui(self.can_go_forward(), |ui| {
                if ui.button(">").on_hover_text("Forward (Alt+Right)").clicked() {
                    self.go_forward();
                }
            });

            // Up button
            ui.add_enabled_ui(self.current_path != self.root_path, |ui| {
                if ui.button("^").on_hover_text("Up (Alt+Up)").clicked() {
                    if let Some(parent) = self.current_path.parent() {
                        self.navigate_to(parent.to_path_buf());
                    }
                }
            });

            // Refresh button
            if ui.button("R").on_hover_text("Refresh (F5)").clicked() {
                self.refresh_filesystem();
            }

            ui.separator();

            // Breadcrumb navigation
            self.render_breadcrumb(ui);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // View mode toggle
                if ui.selectable_label(self.view_mode == AssetViewMode::List, "List").on_hover_text("List View").clicked() {
                    self.view_mode = AssetViewMode::List;
                }
                if ui.selectable_label(self.view_mode == AssetViewMode::Grid, "Grid").on_hover_text("Grid View").clicked() {
                    self.view_mode = AssetViewMode::Grid;
                }

                ui.separator();

                // Icon size slider (grid mode only)
                if self.view_mode == AssetViewMode::Grid {
                    ui.add(egui::Slider::new(&mut self.icon_size, 32.0..=128.0).show_value(false));
                }

                ui.separator();

                // Sort dropdown
                let sort_text = format!("{} {}", self.sort_mode.name(), if self.sort_ascending { "^" } else { "v" });
                egui::ComboBox::from_id_salt("asset_sort")
                    .selected_text(sort_text)
                    .show_ui(ui, |ui| {
                        for mode in [SortMode::Name, SortMode::DateModified, SortMode::Type, SortMode::Size] {
                            if ui.selectable_label(self.sort_mode == mode, mode.name()).clicked() {
                                if self.sort_mode == mode {
                                    self.sort_ascending = !self.sort_ascending;
                                } else {
                                    self.sort_mode = mode;
                                    self.sort_ascending = true;
                                }
                                self.sort_assets();
                            }
                        }
                    });

                ui.separator();

                // Filter dropdown
                egui::ComboBox::from_id_salt("asset_filter")
                    .selected_text(self.filter.name())
                    .show_ui(ui, |ui| {
                        for filter in [
                            AssetType::All,
                            AssetType::Mesh,
                            AssetType::Texture,
                            AssetType::Material,
                            AssetType::Audio,
                            AssetType::Scene,
                            AssetType::Prefab,
                            AssetType::Shader,
                            AssetType::Script,
                            AssetType::Font,
                        ] {
                            ui.selectable_value(&mut self.filter, filter, filter.name());
                        }
                    });

                ui.separator();

                // Search with keyboard focus handling
                let search_response = ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text("Search (Ctrl+F)...")
                        .desired_width(150.0),
                );

                // Focus search if Ctrl+F was pressed
                if self.search_focused {
                    search_response.request_focus();
                    self.search_focused = false;
                }

                if !self.search.is_empty() {
                    if ui.small_button("X").clicked() {
                        self.search.clear();
                    }
                }
            });
        });

        ui.separator();

        // Main content area with optional tree panel
        if self.show_tree {
            egui::SidePanel::left("asset_tree_panel")
                .resizable(true)
                .default_width(self.tree_width)
                .width_range(100.0..=300.0)
                .show_inside(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        self.render_directory_tree(ui);
                    });
                });
        }

        // Content area
        egui::ScrollArea::vertical().show(ui, |ui| {
            match self.view_mode {
                AssetViewMode::Grid => self.grid_view(ui, state),
                AssetViewMode::List => self.list_view(ui, state),
            }
        });
    }

    fn render_breadcrumb(&mut self, ui: &mut egui::Ui) {
        let components: Vec<_> = self.current_path.components().collect();
        let mut accumulated_path = PathBuf::new();
        let mut clicked_path: Option<PathBuf> = None;

        for (i, component) in components.iter().enumerate() {
            accumulated_path.push(component);

            if i > 0 {
                ui.label("/");
            }

            let name = component.as_os_str().to_string_lossy();
            let path_clone = accumulated_path.clone();

            if ui.add(egui::Label::new(
                egui::RichText::new(name.as_ref())
                    .color(if i == components.len() - 1 {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_rgb(150, 150, 200)
                    })
            ).sense(egui::Sense::click())).clicked() {
                clicked_path = Some(path_clone);
            }
        }

        // Apply navigation after the loop to avoid borrow conflicts
        if let Some(path) = clicked_path {
            self.navigate_to(path);
        }
    }

    fn render_directory_tree(&mut self, ui: &mut egui::Ui) {
        let tree = self.directory_tree.clone();
        for entry in tree {
            self.render_tree_entry(ui, &entry, 0);
        }
    }

    fn render_tree_entry(&mut self, ui: &mut egui::Ui, entry: &DirectoryEntry, depth: usize) {
        let indent = depth as f32 * 16.0;
        let is_expanded = self.expanded_dirs.contains(&entry.path);
        let is_selected = self.current_path == entry.path;
        let has_children = !entry.children.is_empty();

        ui.horizontal(|ui| {
            ui.add_space(indent);

            // Expand/collapse button
            if has_children {
                let icon = if is_expanded { "\u{f0d7}" } else { "\u{f0da}" }; // caret-down : caret-right
                if ui.add(egui::Label::new(icon).sense(egui::Sense::click())).clicked() {
                    if is_expanded {
                        self.expanded_dirs.remove(&entry.path);
                    } else {
                        self.expanded_dirs.insert(entry.path.clone());
                    }
                }
            } else {
                ui.add_space(16.0);
            }

            // Folder icon
            let folder_icon = if is_expanded { "\u{f07c}" } else { "\u{f07b}" };
            ui.label(egui::RichText::new(folder_icon).color(egui::Color32::from_rgb(255, 200, 80)));

            // Name
            let name_color = if is_selected {
                egui::Color32::WHITE
            } else {
                egui::Color32::from_rgb(200, 200, 200)
            };

            let response = ui.add(
                egui::Label::new(egui::RichText::new(&entry.name).color(name_color))
                    .sense(egui::Sense::click())
            );

            if response.clicked() {
                self.navigate_to(entry.path.clone());
            }

            // Highlight selected
            if is_selected {
                let rect = response.rect.expand(2.0);
                ui.painter().rect_stroke(rect, 2.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 150, 255)));
            }
        });

        // Render children if expanded
        if is_expanded {
            for child in &entry.children {
                self.render_tree_entry(ui, child, depth + 1);
            }
        }
    }

    fn grid_view(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        let available_width = ui.available_width();
        let item_width = self.icon_size + 16.0;
        let columns = ((available_width / item_width) as usize).max(1);

        // Collect filtered asset data before iterating
        let filtered_data: Vec<(PathBuf, String, AssetType, bool, bool)> = self.assets.iter()
            .filter(|a| self.matches_filter(a))
            .map(|a| (a.path.clone(), a.name.clone(), a.asset_type, a.is_folder, self.selected.contains(&a.path)))
            .collect();

        let mut new_path: Option<PathBuf> = None;
        let mut new_selection: Option<PathBuf> = None;
        let mut open_path: Option<PathBuf> = None;

        // Request thumbnails for visible texture assets
        if self.show_thumbnails {
            for (path, _, asset_type, is_folder, _) in &filtered_data {
                if !*is_folder && *asset_type == AssetType::Texture {
                    self.thumbnail_manager.request_thumbnail(path);
                }
            }
        }

        egui::Grid::new("asset_grid")
            .num_columns(columns)
            .spacing([8.0, 8.0])
            .show(ui, |ui| {
                for (i, (path, name, asset_type, is_folder, is_selected)) in filtered_data.iter().enumerate() {
                    if i > 0 && i % columns == 0 {
                        ui.end_row();
                    }

                    let size = egui::vec2(self.icon_size + 8.0, self.icon_size + 24.0);

                    ui.allocate_ui(size, |ui| {
                        let response = ui.vertical_centered(|ui| {
                            let icon_rect = ui.allocate_space(egui::vec2(self.icon_size, self.icon_size)).1;

                            if *is_selected {
                                ui.painter().rect_filled(
                                    icon_rect.expand(4.0),
                                    4.0,
                                    egui::Color32::from_rgba_unmultiplied(100, 150, 255, 80),
                                );
                            }

                            // Try to render thumbnail for texture assets
                            let rendered_thumbnail = if self.show_thumbnails && !*is_folder && *asset_type == AssetType::Texture {
                                self.render_thumbnail(ui, path, icon_rect)
                            } else {
                                false
                            };

                            // Fall back to icon if no thumbnail
                            if !rendered_thumbnail {
                                let icon = if *is_folder { "\u{f07b}" } else { asset_type.icon() };
                                let icon_color = if *is_folder {
                                    egui::Color32::from_rgb(255, 200, 80)
                                } else {
                                    egui::Color32::from_rgb(180, 180, 180)
                                };

                                ui.painter().text(
                                    icon_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    icon,
                                    egui::FontId::proportional(self.icon_size * 0.5),
                                    icon_color,
                                );
                            }

                            let max_chars = (self.icon_size / 8.0) as usize;
                            let display_name = if name.len() > max_chars {
                                format!("{}...", &name[..max_chars.saturating_sub(3)])
                            } else {
                                name.clone()
                            };

                            ui.label(egui::RichText::new(display_name).small());
                        });

                        if response.response.clicked() {
                            if *is_folder {
                                new_path = Some(path.clone());
                            } else {
                                new_selection = Some(path.clone());
                            }
                        }

                        if response.response.double_clicked() {
                            if *is_folder {
                                new_path = Some(path.clone());
                            } else {
                                open_path = Some(path.clone());
                            }
                        }
                    });
                }
            });

        // Apply deferred state changes
        if let Some(path) = new_path {
            self.navigate_to(path);
        }
        if let Some(path) = new_selection {
            self.selected = vec![path];
        }
        if let Some(path) = open_path {
            self.open_asset(state, &path);
        }
    }

    fn list_view(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        // Collect filtered assets
        let filtered: Vec<_> = self.assets.iter()
            .filter(|a| self.matches_filter(a))
            .cloned()
            .collect();

        // Table header
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Name").strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("Type").strong());
            });
        });
        ui.separator();

        for asset in filtered {
            let is_selected = self.selected.contains(&asset.path);
            let icon = if asset.is_folder { "\u{f07b}" } else { asset.asset_type.icon() };
            let icon_color = if asset.is_folder {
                egui::Color32::from_rgb(255, 200, 80)
            } else {
                asset.asset_type.color()
            };

            let response = ui.horizontal(|ui| {
                // Selection highlight
                if is_selected {
                    let rect = ui.available_rect_before_wrap();
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(rect.min, egui::vec2(ui.available_width(), 20.0)),
                        0.0,
                        egui::Color32::from_rgba_unmultiplied(100, 150, 255, 50),
                    );
                }

                // Icon
                ui.label(egui::RichText::new(icon).color(icon_color));

                // Name
                let name_response = ui.add(
                    egui::Label::new(&asset.name).sense(egui::Sense::click())
                );

                // Show asset type on the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(asset.asset_type.name()).color(asset.asset_type.color()).small());
                });

                name_response
            });

            if response.inner.clicked() {
                if asset.is_folder {
                    self.navigate_to(asset.path.clone());
                } else {
                    self.selected = vec![asset.path.clone()];
                }
            }

            if response.inner.double_clicked() && !asset.is_folder {
                self.open_asset(state, &asset.path);
            }

            // Context menu
            response.inner.context_menu(|ui| {
                if ui.button("Open").clicked() {
                    if asset.is_folder {
                        self.navigate_to(asset.path.clone());
                    } else {
                        self.open_asset(state, &asset.path);
                    }
                    ui.close_menu();
                }
                if ui.button("Show in Explorer").clicked() {
                    Self::show_in_explorer(&asset.path);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Copy Path").clicked() {
                    ui.output_mut(|o| o.copied_text = asset.path.display().to_string());
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Rename").clicked() {
                    self.begin_rename(&asset.path);
                    ui.close_menu();
                }
                if ui.button("Delete").clicked() {
                    self.pending_delete = Some(asset.path.clone());
                    ui.close_menu();
                }
            });
        }

        // Handle pending delete
        if let Some(path) = self.pending_delete.take() {
            self.delete_asset(&path);
        }

        // Handle inline rename: render overlay/check for completion
        if self.renaming_path.is_some() {
            self.show_rename_ui(ui);
        }
    }

    /// Render a thumbnail for the given path in the given rect
    /// Returns true if a thumbnail was rendered, false if fallback to icon is needed
    fn render_thumbnail(&self, ui: &mut egui::Ui, path: &PathBuf, rect: egui::Rect) -> bool {
        match self.thumbnail_manager.get_state(path) {
            ThumbnailState::Ready(texture_id) => {
                // Draw the thumbnail image
                ui.painter().image(
                    texture_id,
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );

                // Draw subtle border
                ui.painter().rect_stroke(
                    rect,
                    4.0,
                    egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
                );

                true
            }
            ThumbnailState::Loading => {
                // Draw loading indicator
                ui.painter().rect_filled(rect, 4.0, egui::Color32::from_gray(45));

                let center = rect.center();
                let time = ui.input(|i| i.time);
                let angle = time as f32 * 3.0;
                let radius = rect.width().min(rect.height()) * 0.25;

                // Background circle
                ui.painter().circle_stroke(
                    center,
                    radius,
                    egui::Stroke::new(2.0, egui::Color32::from_gray(60)),
                );

                // Spinning arc
                let start_angle = angle;
                let end_angle = angle + std::f32::consts::PI * 0.75;
                let points: Vec<egui::Pos2> = (0..20)
                    .map(|i| {
                        let t = i as f32 / 19.0;
                        let a = start_angle + (end_angle - start_angle) * t;
                        egui::pos2(center.x + radius * a.cos(), center.y + radius * a.sin())
                    })
                    .collect();

                ui.painter().add(egui::Shape::line(
                    points,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 150, 255)),
                ));

                // Request repaint for animation
                ui.ctx().request_repaint();

                true
            }
            ThumbnailState::Failed(_) | ThumbnailState::UseDefault | ThumbnailState::NotLoaded => {
                // Use fallback icon
                false
            }
        }
    }

    fn matches_filter(&self, asset: &AssetEntry) -> bool {
        // Search filter
        if !self.search.is_empty() {
            if !asset.name.to_lowercase().contains(&self.search.to_lowercase()) {
                return false;
            }
        }

        // Type filter
        if self.filter != AssetType::All && !asset.is_folder {
            if asset.asset_type != self.filter {
                return false;
            }
        }

        true
    }

    /// Open the OS file manager and select/reveal the given path
    fn show_in_explorer(path: &std::path::Path) {
        let canonical = std::fs::canonicalize(path)
            .unwrap_or_else(|_| path.to_path_buf());

        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("explorer")
                .arg(format!("/select,\"{}\"", canonical.display()))
                .spawn();
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(dir) = canonical.parent() {
                let _ = std::process::Command::new("xdg-open")
                    .arg(dir)
                    .spawn();
            }
        }

        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open")
                .arg("-R")
                .arg(&canonical)
                .spawn();
        }

        tracing::info!("Show in explorer: {}", canonical.display());
    }

    /// Begin inline rename for the given path
    fn begin_rename(&mut self, path: &std::path::Path) {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        self.renaming_path = Some(path.to_path_buf());
        self.rename_buffer = name;
        self.rename_focus_request = true;
    }

    /// Finish inline rename — actually rename the file/directory on disk
    fn finish_rename(&mut self) {
        if let Some(old_path) = self.renaming_path.take() {
            let new_name = self.rename_buffer.trim();
            if new_name.is_empty() {
                tracing::warn!("Rename cancelled: empty name");
                return;
            }
            if let Some(parent) = old_path.parent() {
                let new_path = parent.join(new_name);
                if new_path == old_path {
                    return; // no change
                }
                match std::fs::rename(&old_path, &new_path) {
                    Ok(()) => {
                        tracing::info!("Renamed {} -> {}", old_path.display(), new_path.display());
                        for sel in &mut self.selected {
                            if *sel == old_path {
                                *sel = new_path.clone();
                            }
                        }
                        self.needs_refresh = true;
                        self.scan_current_directory();
                    }
                    Err(e) => {
                        tracing::error!("Failed to rename {}: {}", old_path.display(), e);
                    }
                }
            }
        }
        self.rename_buffer.clear();
    }

    /// Cancel inline rename
    fn cancel_rename(&mut self) {
        self.renaming_path = None;
        self.rename_buffer.clear();
    }

    /// Delete a file or directory
    fn delete_asset(&mut self, path: &std::path::Path) {
        let path_str = path.display().to_string();
        let result = if path.is_dir() {
            tracing::info!("Deleting directory: {}", path_str);
            std::fs::remove_dir_all(path)
        } else {
            tracing::info!("Deleting file: {}", path_str);
            std::fs::remove_file(path)
        };
        match result {
            Ok(()) => {
                tracing::info!("Deleted: {}", path_str);
                self.selected.retain(|p| p.as_path() != path);
                self.needs_refresh = true;
                self.scan_current_directory();
            }
            Err(e) => {
                tracing::error!("Failed to delete {}: {}", path_str, e);
            }
        }
    }

    /// Show an inline rename text edit UI at the bottom of the panel area
    fn show_rename_ui(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Rename:");
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.rename_buffer)
                    .desired_width(200.0)
            );

            if self.rename_focus_request {
                response.request_focus();
                self.rename_focus_request = false;
            }

            // Finish on Enter or focus lost
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.finish_rename();
            } else if response.lost_focus() {
                // Also finish on focus lost (clicked away)
                self.finish_rename();
            }

            if ui.button("OK").clicked() {
                self.finish_rename();
            }
            if ui.button("Cancel").clicked() {
                self.cancel_rename();
            }

            // Escape to cancel
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.cancel_rename();
            }
        });
    }

    fn open_asset(&mut self, state: &mut EditorState, path: &PathBuf) {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let asset_type = AssetType::from_extension(ext);

        match asset_type {
            AssetType::Scene => {
                if state.has_unsaved_changes() {
                    tracing::warn!("Unsaved changes - save before opening {}", path.display());
                    return;
                }
                if let Err(err) = state.load_scene(path) {
                    tracing::error!("Failed to load scene {}: {}", path.display(), err);
                }
            }
            AssetType::Material | AssetType::Shader => {
                state.request_panel_open(PanelType::MaterialGraph);
                tracing::info!("Opening material editor for {}", path.display());
            }
            AssetType::Animation => {
                state.request_panel_open(PanelType::Sequencer);
                tracing::info!("Opening sequencer for {}", path.display());
            }
            AssetType::Audio => {
                // Preview audio clip
                if state.audio_engine.preview_clip(path) {
                    tracing::info!("Playing audio preview: {}", path.display());
                } else {
                    tracing::warn!("Failed to play audio preview: {}", path.display());
                }
            }
            AssetType::Prefab => {
                // Set selected asset for drag-drop or inspector view
                state.selected_asset = Some(path.clone());
                tracing::info!("Selected prefab: {}", path.display());
            }
            _ => {
                tracing::info!("Open asset not implemented: {}", path.display());
            }
        }
    }
}

impl Default for AssetBrowserPanel {
    fn default() -> Self {
        Self::new()
    }
}
