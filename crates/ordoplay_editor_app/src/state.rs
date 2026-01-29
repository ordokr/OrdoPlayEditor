// SPDX-License-Identifier: MIT OR Apache-2.0
//! Editor state management.
//!
//! This module contains the core editor state including selection,
//! undo/redo history, and scene management.


use crate::commands::{
    DeleteCommand, DuplicateCommand, EditorCommand, PropertyEditCommand, PropertyEditGroupCommand,
    PropertyEditSnapshot, ReparentCommand, SpawnCommand, TransformCommand, TransformData,
};
use crate::history::{History, HistoryError, Operation, OperationGroup, StateSnapshot};
use crate::panel_types::PanelType;
use crate::tools::GizmoMode;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use uuid::Uuid;

/// Maximum number of recent scenes to track
const MAX_RECENT_SCENES: usize = 10;

/// Unique identifier for entities in the editor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub Uuid);

impl EntityId {
    /// Create a new random entity ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for EntityId {
    fn default() -> Self {
        Self::new()
    }
}

/// Selection mode for multi-select operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectMode {
    /// Replace current selection
    #[default]
    Set,
    /// Add to current selection (Shift+Click)
    Add,
    /// Remove from current selection (Ctrl+Click)
    #[allow(dead_code)] // Intentionally kept for API completeness
    Remove,
    /// Toggle in current selection (Ctrl+Shift+Click)
    Toggle,
}

/// Entity selection state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Selection {
    /// Currently selected entities
    pub entities: Vec<EntityId>,
}

impl Selection {
    /// Create a new empty selection
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a selection with the given entities
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn with_entities(entities: impl Into<Vec<EntityId>>) -> Self {
        Self {
            entities: entities.into(),
        }
    }

    /// Check if an entity is selected
    pub fn contains(&self, id: &EntityId) -> bool {
        self.entities.contains(id)
    }

    /// Add an entity to the selection (idempotent)
    pub fn add(&mut self, id: EntityId) {
        if !self.contains(&id) {
            self.entities.push(id);
        }
    }

    /// Remove an entity from the selection
    pub fn remove(&mut self, id: &EntityId) {
        self.entities.retain(|e| e != id);
    }

    /// Toggle an entity in the selection
    pub fn toggle(&mut self, id: EntityId) {
        if self.contains(&id) {
            self.remove(&id);
        } else {
            self.add(id);
        }
    }

    /// Union with another selection
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn union(&mut self, other: &Selection) {
        for id in &other.entities {
            self.add(*id);
        }
    }

    /// Difference with another selection
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn difference(&mut self, other: &Selection) {
        for id in &other.entities {
            self.remove(id);
        }
    }

    /// Clear the selection
    pub fn clear(&mut self) {
        self.entities.clear();
    }

    /// Check if the selection is empty
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Get the number of selected entities
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Iterate over selected entities
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn iter(&self) -> impl Iterator<Item = &EntityId> {
        self.entities.iter()
    }

    /// Get the first selected entity
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn first(&self) -> Option<&EntityId> {
        self.entities.first()
    }

    /// Get the primary (last) selected entity
    pub fn primary(&self) -> Option<&EntityId> {
        self.entities.last()
    }
}

/// Transform component data
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transform {
    /// Position (x, y, z)
    pub position: [f32; 3],
    /// Rotation in euler angles (degrees)
    pub rotation: [f32; 3],
    /// Scale
    pub scale: [f32; 3],
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

/// Entity data stored in the editor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityData {
    /// Entity name
    pub name: String,
    /// Whether the entity is active
    pub active: bool,
    /// Whether the entity is static (won't move at runtime)
    pub is_static: bool,
    /// Transform component
    pub transform: Transform,
    /// Parent entity (if any)
    pub parent: Option<EntityId>,
    /// Child entities
    pub children: Vec<EntityId>,
    /// Components attached to this entity
    #[serde(default)]
    pub components: Vec<crate::components::Component>,
}

impl Default for EntityData {
    fn default() -> Self {
        Self {
            name: "Entity".to_string(),
            active: true,
            is_static: false,
            transform: Transform::default(),
            parent: None,
            children: Vec::new(),
            components: Vec::new(),
        }
    }
}

impl EntityData {
    /// Create a new entity with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

/// Current scene file format version
pub const SCENE_FORMAT_VERSION: u32 = 1;

/// Scene file format with versioning and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneFile {
    /// Format version for compatibility checking
    pub version: u32,
    /// Scene name/title
    pub name: String,
    /// Scene description (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Creation timestamp (ISO 8601)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    /// Last modified timestamp (ISO 8601)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
    /// The actual scene data
    pub scene: SceneData,
}

impl SceneFile {
    /// Create a new scene file with default metadata
    pub fn new(name: impl Into<String>) -> Self {
        let now = Self::timestamp_now();
        Self {
            version: SCENE_FORMAT_VERSION,
            name: name.into(),
            description: None,
            created: Some(now.clone()),
            modified: Some(now),
            scene: SceneData::default(),
        }
    }

    /// Wrap existing scene data in a file format
    pub fn from_scene(name: impl Into<String>, scene: SceneData) -> Self {
        let now = Self::timestamp_now();
        Self {
            version: SCENE_FORMAT_VERSION,
            name: name.into(),
            description: None,
            created: Some(now.clone()),
            modified: Some(now),
            scene,
        }
    }

    /// Update the modified timestamp
    pub fn touch(&mut self) {
        self.modified = Some(Self::timestamp_now());
    }

    /// Get current timestamp in ISO 8601 format
    fn timestamp_now() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        // Simple ISO 8601 timestamp (without chrono dependency)
        let secs = duration.as_secs();
        let days = secs / 86400;
        let time = secs % 86400;
        let hours = time / 3600;
        let mins = (time % 3600) / 60;
        let secs = time % 60;
        // Approximate date calculation (not accounting for leap years precisely)
        let years = 1970 + days / 365;
        let remaining_days = days % 365;
        let months = remaining_days / 30 + 1;
        let day = remaining_days % 30 + 1;
        format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            years, months.min(12), day.min(31), hours, mins, secs)
    }
}

impl Default for SceneFile {
    fn default() -> Self {
        Self::new("Untitled Scene")
    }
}

/// Scene data containing all entities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SceneData {
    /// All entities in the scene
    pub entities: IndexMap<EntityId, EntityData>,
}

impl SceneData {
    /// Create a new empty scene
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an entity to the scene
    pub fn add_entity(&mut self, data: EntityData) -> EntityId {
        let id = EntityId::new();
        self.entities.insert(id, data);
        id
    }

    /// Insert an entity with a specific ID
    pub fn insert_entity(&mut self, id: EntityId, data: EntityData) -> bool {
        self.entities.insert(id, data).is_none()
    }

    /// Get an entity by ID
    pub fn get(&self, id: &EntityId) -> Option<&EntityData> {
        self.entities.get(id)
    }

    /// Get a mutable reference to an entity by ID
    pub fn get_mut(&mut self, id: &EntityId) -> Option<&mut EntityData> {
        self.entities.get_mut(id)
    }

    /// Remove an entity from the scene
    pub fn remove(&mut self, id: &EntityId) -> Option<EntityData> {
        self.entities.shift_remove(id)
    }

    /// Get all root entities (no parent)
    pub fn root_entities(&self) -> Vec<EntityId> {
        self.entities
            .iter()
            .filter(|(_, data)| data.parent.is_none())
            .map(|(id, _)| *id)
            .collect()
    }
}

/// Main editor state
pub struct EditorState {
    /// Current entity selection
    pub selection: Selection,

    /// Scene data
    pub scene: SceneData,

    /// Undo/redo history
    pub history: History,

    /// Current gizmo mode
    pub gizmo_mode: GizmoMode,

    /// Current scene file path
    pub scene_path: Option<PathBuf>,

    /// Whether the scene has unsaved changes
    pub dirty: bool,

    /// Current select mode (for multi-select)
    pub select_mode: SelectMode,

    /// Coordinate space (local vs world)
    pub use_world_space: bool,

    /// Snap to grid enabled
    pub snap_enabled: bool,

    /// Grid snap size
    pub snap_size: f32,

    /// Rotation snap angle in degrees
    pub rotation_snap: f32,

    /// Scale snap increment
    pub scale_snap: f32,

    /// Recent scenes list
    pub recent_scenes: VecDeque<PathBuf>,

    /// Panels requested to open
    pending_panels: Vec<PanelType>,

    /// Prefab manager for prefab instances
    pub prefab_manager: crate::prefab::PrefabManager,

    /// Currently selected asset path in asset browser
    pub selected_asset: Option<PathBuf>,

    /// Entity to create a prefab from (shows dialog when Some)
    pub show_create_prefab_dialog: Option<EntityId>,

    /// Currently editing prefab (path and backup of main scene)
    pub editing_prefab: Option<PrefabEditingState>,

    /// Project manager for project settings
    pub project_manager: crate::project::ProjectManager,

    /// Build manager for project builds
    pub build_manager: crate::build::BuildManager,

    /// Play mode manager for in-editor preview
    pub play_mode: crate::play_mode::PlayModeManager,

    /// Physics world for simulation
    pub physics_world: crate::physics::PhysicsWorld,

    /// Physics debug visualization settings
    pub physics_debug: PhysicsDebugSettings,

    /// Audio engine for playback
    pub audio_engine: crate::audio::AudioEngine,
}

/// Physics debug visualization settings
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone)]
pub struct PhysicsDebugSettings {
    /// Show collider shapes
    pub show_colliders: bool,
    /// Show rigidbody velocities
    pub show_velocities: bool,
    /// Show contact points
    pub show_contacts: bool,
    /// Show collision layer info
    pub show_layers: bool,
    /// Collider wireframe color
    pub collider_color: [f32; 4],
    /// Trigger volume color
    pub trigger_color: [f32; 4],
    /// Velocity arrow color
    pub velocity_color: [f32; 4],
    /// Contact point color
    pub contact_color: [f32; 4],
}

impl Default for PhysicsDebugSettings {
    fn default() -> Self {
        Self {
            show_colliders: false,
            show_velocities: false,
            show_contacts: false,
            show_layers: false,
            collider_color: [0.0, 1.0, 0.0, 0.8], // Green
            trigger_color: [1.0, 1.0, 0.0, 0.5],  // Yellow
            velocity_color: [0.0, 0.0, 1.0, 1.0], // Blue
            contact_color: [1.0, 0.0, 0.0, 1.0],  // Red
        }
    }
}

/// State for editing a prefab
pub struct PrefabEditingState {
    /// Path to the prefab being edited
    pub prefab_path: PathBuf,
    /// Backup of the main scene before entering prefab edit mode
    pub scene_backup: SceneData,
    /// Backup of selection before entering prefab edit mode
    pub selection_backup: Selection,
    /// Whether the prefab has unsaved changes
    pub prefab_dirty: bool,
}

impl EditorState {
    /// Create a new editor state
    pub fn new() -> Self {
        let mut scene = SceneData::new();

        // Add some test entities
        let cube = scene.add_entity(EntityData::new("Cube"));
        let _sphere = scene.add_entity(EntityData {
            name: "Sphere".to_string(),
            transform: Transform {
                position: [3.0, 0.0, 0.0],
                ..Default::default()
            },
            ..Default::default()
        });
        let _light = scene.add_entity(EntityData {
            name: "Directional Light".to_string(),
            transform: Transform {
                position: [0.0, 10.0, 0.0],
                rotation: [45.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            },
            ..Default::default()
        });

        // Select the cube by default
        let mut selection = Selection::new();
        selection.add(cube);

        Self {
            selection,
            scene,
            history: History::new(),
            gizmo_mode: GizmoMode::Translate,
            scene_path: None,
            dirty: false,
            select_mode: SelectMode::Set,
            use_world_space: true,
            snap_enabled: false,
            snap_size: 1.0,
            rotation_snap: 15.0,
            scale_snap: 0.1,
            recent_scenes: VecDeque::new(),
            pending_panels: Vec::new(),
            prefab_manager: crate::prefab::PrefabManager::new(),
            selected_asset: None,
            show_create_prefab_dialog: None,
            editing_prefab: None,
            project_manager: crate::project::ProjectManager::new(),
            build_manager: crate::build::BuildManager::new(),
            play_mode: crate::play_mode::PlayModeManager::new(),
            physics_world: crate::physics::PhysicsWorld::new(),
            physics_debug: PhysicsDebugSettings::default(),
            audio_engine: crate::audio::AudioEngine::new(),
        }
    }

    /// Create a new scene
    pub fn new_scene(&mut self) {
        // TODO: Confirm unsaved changes
        self.selection.clear();
        self.scene = SceneData::new();
        self.history.clear();
        self.scene_path = None;
        self.dirty = false;
        tracing::info!("Created new scene");
    }

    /// Save the current scene to a file
    pub fn save_scene(&mut self) -> Result<(), String> {
        if let Some(path) = &self.scene_path.clone() {
            self.save_scene_to_path(path)
        } else {
            Err("No scene path set".to_string())
        }
    }

    /// Save the current scene to a specific path
    pub fn save_scene_to_path(&mut self, path: &std::path::Path) -> Result<(), String> {
        // Extract scene name from path or use existing
        let name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled Scene")
            .to_string();

        // Create scene file with versioning
        let mut scene_file = SceneFile::from_scene(name, self.scene.clone());
        scene_file.touch(); // Update modified timestamp

        // Configure RON pretty printing
        let config = ron::ser::PrettyConfig::default()
            .struct_names(true)
            .enumerate_arrays(false);

        // Serialize scene file to RON format
        let ron_str = ron::ser::to_string_pretty(&scene_file, config)
            .map_err(|e| format!("Serialization error: {}", e))?;

        // Write to file
        std::fs::write(path, ron_str)
            .map_err(|e| format!("File write error: {}", e))?;

        self.scene_path = Some(path.to_path_buf());
        self.dirty = false;

        // Add to recent scenes
        self.add_to_recent(path.to_path_buf());

        tracing::info!("Saved scene v{} to {:?}", SCENE_FORMAT_VERSION, path);
        Ok(())
    }

    /// Load a scene from a file
    pub fn load_scene(&mut self, path: &std::path::Path) -> Result<(), String> {
        // Read file contents
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("File read error: {}", e))?;

        // Try to load as new versioned format first
        let scene = if let Ok(scene_file) = ron::from_str::<SceneFile>(&content) {
            // Check version compatibility
            if scene_file.version > SCENE_FORMAT_VERSION {
                return Err(format!(
                    "Scene file version {} is newer than supported version {}. Please update the editor.",
                    scene_file.version, SCENE_FORMAT_VERSION
                ));
            }
            if scene_file.version < SCENE_FORMAT_VERSION {
                tracing::info!(
                    "Upgrading scene from v{} to v{}",
                    scene_file.version, SCENE_FORMAT_VERSION
                );
            }
            tracing::info!("Loaded scene '{}' v{}", scene_file.name, scene_file.version);
            scene_file.scene
        } else {
            // Fall back to legacy format (raw SceneData)
            tracing::info!("Loading legacy scene format (pre-v1)");
            ron::from_str::<SceneData>(&content)
                .map_err(|e| format!("Deserialization error: {}", e))?
        };

        // Update state
        self.scene = scene;
        self.selection.clear();
        self.history.clear();
        self.scene_path = Some(path.to_path_buf());
        self.dirty = false;

        // Add to recent scenes
        self.add_to_recent(path.to_path_buf());

        tracing::info!("Loaded scene from {:?}", path);
        Ok(())
    }

    /// Check if scene has unsaved changes
    pub fn has_unsaved_changes(&self) -> bool {
        self.dirty
    }

    /// Add a scene to the recent scenes list
    pub fn add_to_recent(&mut self, path: PathBuf) {
        // Remove if already in list (to move it to the front)
        self.recent_scenes.retain(|p| p != &path);

        // Add to front
        self.recent_scenes.push_front(path);

        // Trim to max size
        while self.recent_scenes.len() > MAX_RECENT_SCENES {
            self.recent_scenes.pop_back();
        }
    }

    /// Clear the recent scenes list
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn clear_recent_scenes(&mut self) {
        self.recent_scenes.clear();
    }

    /// Get the scene file name (for window title)
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn scene_name(&self) -> String {
        if let Some(path) = &self.scene_path {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled")
                .to_string()
        } else {
            "Untitled".to_string()
        }
    }

    /// Select entities based on current select mode
    pub fn select(&mut self, entities: &[EntityId]) {
        match self.select_mode {
            SelectMode::Set => {
                self.selection.clear();
                for id in entities {
                    self.selection.add(*id);
                }
            }
            SelectMode::Add => {
                for id in entities {
                    self.selection.add(*id);
                }
            }
            SelectMode::Remove => {
                for id in entities {
                    self.selection.remove(id);
                }
            }
            SelectMode::Toggle => {
                for id in entities {
                    self.selection.toggle(*id);
                }
            }
        }
    }

    /// Delete selected entities
    pub fn delete_selected(&mut self) {
        if self.selection.is_empty() {
            return;
        }

        let ids = self.selection.entities.clone();
        self.delete_entities_with_command(&ids);
    }

    /// Duplicate selected entities
    pub fn duplicate_selected(&mut self) {
        if self.selection.is_empty() {
            return;
        }

        let ids = self.selection.entities.clone();
        let _ = self.duplicate_entities(&ids);
    }

    /// Mark the scene as modified
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Spawn a new entity using the command pipeline
    pub fn spawn_entity_with_command(
        &mut self,
        name: impl Into<String>,
        parent: Option<EntityId>,
        select: bool,
    ) -> Option<EntityId> {
        let entity_id = EntityId::new();
        let mut command = SpawnCommand::new(entity_id, TransformData::default())
            .with_name(name)
            .with_select(select);
        if let Some(parent_id) = parent {
            command = command.with_parent(parent_id);
        }

        if let Err(err) = self.execute_command(&command) {
            tracing::warn!("Spawn command failed: {}", err);
            return None;
        }

        Some(entity_id)
    }

    /// Set entity transform with undo support
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn set_transform(&mut self, entity_id: EntityId, new_transform: Transform, description: &str) {
        let old_transform = match self.scene.get(&entity_id) {
            Some(data) => data.transform.clone(),
            None => return,
        };

        self.set_transform_with_before(entity_id, old_transform, new_transform, description);
    }

    /// Set entity transform with a provided "before" snapshot
    pub fn set_transform_with_before(
        &mut self,
        entity_id: EntityId,
        before: Transform,
        after: Transform,
        description: &str,
    ) {
        if before == after {
            return;
        }

        let command = TransformCommand::new(
            vec![entity_id],
            vec![TransformData::from(before)],
            vec![TransformData::from(after)],
            description,
        );

        if let Err(err) = self.execute_command(&command) {
            tracing::warn!("Transform command failed: {}", err);
        }
    }

    /// Set entity name with undo support
    pub fn set_entity_name(&mut self, entity_id: EntityId, new_name: String) {
        let old_name = match self.scene.get(&entity_id) {
            Some(data) => data.name.clone(),
            None => return,
        };

        if old_name == new_name {
            return;
        }

        let Ok(old_value) = bincode::serialize(&old_name) else {
            tracing::warn!("Failed to serialize old entity name");
            return;
        };
        let Ok(new_value) = bincode::serialize(&new_name) else {
            tracing::warn!("Failed to serialize new entity name");
            return;
        };

        let command = PropertyEditCommand::new(entity_id, "Entity", "name", old_value, new_value);
        if let Err(err) = self.execute_command(&command) {
            tracing::warn!("Rename command failed: {}", err);
        }
    }

    /// Set entity active flag with undo support
    pub fn set_entity_active(&mut self, entity_id: EntityId, active: bool) {
        let old_value = match self.scene.get(&entity_id) {
            Some(data) => data.active,
            None => return,
        };

        if old_value == active {
            return;
        }

        let Ok(old_value) = bincode::serialize(&old_value) else {
            tracing::warn!("Failed to serialize entity active state");
            return;
        };
        let Ok(new_value) = bincode::serialize(&active) else {
            tracing::warn!("Failed to serialize entity active state");
            return;
        };

        let command = PropertyEditCommand::new(entity_id, "Entity", "active", old_value, new_value);
        if let Err(err) = self.execute_command(&command) {
            tracing::warn!("Active toggle failed: {}", err);
        }
    }

    /// Set entity static flag with undo support
    pub fn set_entity_static(&mut self, entity_id: EntityId, is_static: bool) {
        let old_value = match self.scene.get(&entity_id) {
            Some(data) => data.is_static,
            None => return,
        };

        if old_value == is_static {
            return;
        }

        let Ok(old_value) = bincode::serialize(&old_value) else {
            tracing::warn!("Failed to serialize entity static state");
            return;
        };
        let Ok(new_value) = bincode::serialize(&is_static) else {
            tracing::warn!("Failed to serialize entity static state");
            return;
        };

        let command = PropertyEditCommand::new(entity_id, "Entity", "is_static", old_value, new_value);
        if let Err(err) = self.execute_command(&command) {
            tracing::warn!("Static toggle failed: {}", err);
        }
    }

    /// Set active flag for multiple entities as a single undo operation
    pub fn set_entities_active_bulk(&mut self, entities: &[EntityId], active: bool) {
        let mut edits = Vec::new();

        for entity_id in entities {
            let Some(data) = self.scene.get(entity_id) else {
                continue;
            };
            if data.active == active {
                continue;
            }

            let Ok(old_value) = bincode::serialize(&data.active) else {
                continue;
            };
            let Ok(new_value) = bincode::serialize(&active) else {
                continue;
            };

            edits.push(PropertyEditCommand::new(
                *entity_id,
                "Entity",
                "active",
                old_value,
                new_value,
            ));
        }

        if edits.is_empty() {
            return;
        }

        let command = PropertyEditGroupCommand::new("Set Active", edits);
        if let Err(err) = self.execute_command(&command) {
            tracing::warn!("Bulk active toggle failed: {}", err);
        }
    }

    /// Set static flag for multiple entities as a single undo operation
    pub fn set_entities_static_bulk(&mut self, entities: &[EntityId], is_static: bool) {
        let mut edits = Vec::new();

        for entity_id in entities {
            let Some(data) = self.scene.get(entity_id) else {
                continue;
            };
            if data.is_static == is_static {
                continue;
            }

            let Ok(old_value) = bincode::serialize(&data.is_static) else {
                continue;
            };
            let Ok(new_value) = bincode::serialize(&is_static) else {
                continue;
            };

            edits.push(PropertyEditCommand::new(
                *entity_id,
                "Entity",
                "is_static",
                old_value,
                new_value,
            ));
        }

        if edits.is_empty() {
            return;
        }

        let command = PropertyEditGroupCommand::new("Set Static", edits);
        if let Err(err) = self.execute_command(&command) {
            tracing::warn!("Bulk static toggle failed: {}", err);
        }
    }

    /// Add a component to an entity with undo support
    pub fn add_component(&mut self, entity_id: EntityId, component: crate::components::Component) {
        use crate::commands::AddComponentCommand;

        let command = AddComponentCommand::new(entity_id, component);
        if let Err(err) = self.execute_command(&command) {
            tracing::warn!("Add component failed: {}", err);
        }
    }

    /// Remove a component from an entity with undo support
    pub fn remove_component(&mut self, entity_id: EntityId, component_index: usize) {
        use crate::commands::RemoveComponentCommand;

        // Get the component to be removed for undo
        let Some(entity) = self.scene.get(&entity_id) else {
            tracing::warn!("Entity not found: {:?}", entity_id);
            return;
        };

        if component_index >= entity.components.len() {
            tracing::warn!("Component index {} out of bounds", component_index);
            return;
        }

        let removed_component = entity.components[component_index].clone();
        let command = RemoveComponentCommand::new(entity_id, component_index, removed_component);

        if let Err(err) = self.execute_command(&command) {
            tracing::warn!("Remove component failed: {}", err);
        }
    }

    /// Check if entity has a component of the given type
    pub fn has_component(&self, entity_id: EntityId, type_id: &str) -> bool {
        self.scene
            .get(&entity_id)
            .map(|e| e.components.iter().any(|c| c.type_id() == type_id))
            .unwrap_or(false)
    }

    /// Request a panel to be opened by the UI
    pub fn request_panel_open(&mut self, panel: PanelType) {
        self.pending_panels.push(panel);
    }

    /// Take pending panel open requests
    pub fn take_pending_panels(&mut self) -> Vec<PanelType> {
        std::mem::take(&mut self.pending_panels)
    }

    /// Undo the last operation and apply it to the scene
    pub fn undo(&mut self) -> Result<(), HistoryError> {
        let group = self.history.undo()?;
        self.apply_operation_group(&group, HistoryDirection::Undo);
        self.dirty = true;
        Ok(())
    }

    /// Redo the last operation and apply it to the scene
    pub fn redo(&mut self) -> Result<(), HistoryError> {
        let group = self.history.redo()?;
        self.apply_operation_group(&group, HistoryDirection::Redo);
        self.dirty = true;
        Ok(())
    }

    /// Delete a set of entities (including their descendants)
    pub fn delete_entities(&mut self, ids: &[EntityId]) {
        let to_remove = self.collect_with_descendants(ids);
        if to_remove.is_empty() {
            return;
        }

        self.remove_entities_by_id(&to_remove);
        self.selection.clear();
        self.dirty = true;
    }

    /// Delete a set of entities via commands (undo/redo)
    pub fn delete_entities_with_command(&mut self, ids: &[EntityId]) {
        if ids.is_empty() {
            return;
        }

        let command = DeleteCommand::new(ids.to_vec());
        if let Err(err) = self.execute_command(&command) {
            tracing::warn!("Delete command failed: {}", err);
        }
    }

    /// Duplicate a set of entities with undo support
    pub fn duplicate_entities(&mut self, ids: &[EntityId]) -> Vec<EntityId> {
        if ids.is_empty() {
            return Vec::new();
        }

        let command = DuplicateCommand::new(ids.to_vec());
        let new_ids = command.new_entities.clone();
        if let Err(err) = self.execute_command(&command) {
            tracing::warn!("Duplicate command failed: {}", err);
            return Vec::new();
        }

        new_ids
    }

    /// Reparent entities via commands (undo/redo)
    pub fn reparent_entities_with_command(
        &mut self,
        entities: &[EntityId],
        new_parent: Option<EntityId>,
    ) {
        if entities.is_empty() {
            return;
        }

        if let Some(parent_id) = new_parent {
            if !self.scene.entities.contains_key(&parent_id) {
                tracing::warn!("Cannot reparent to missing parent {:?}", parent_id);
                return;
            }
        }

        let mut ids = Vec::new();
        let mut old_parents = Vec::new();

        for entity_id in entities {
            let Some(entity) = self.scene.get(entity_id) else {
                continue;
            };
            if Some(*entity_id) == new_parent || entity.parent == new_parent {
                continue;
            }
            ids.push(*entity_id);
            old_parents.push(entity.parent);
        }

        if ids.is_empty() {
            return;
        }

        let command = ReparentCommand::new(ids, old_parents, new_parent);
        if let Err(err) = self.execute_command(&command) {
            tracing::warn!("Reparent command failed: {}", err);
        }
    }

    fn apply_operation_group(&mut self, group: &OperationGroup, direction: HistoryDirection) {
        let ops: Box<dyn Iterator<Item = &Operation>> = match direction {
            HistoryDirection::Undo => Box::new(group.operations.iter().rev()),
            HistoryDirection::Redo => Box::new(group.operations.iter()),
        };

        for operation in ops {
            let snapshot = match direction {
                HistoryDirection::Undo => &operation.before,
                HistoryDirection::Redo => &operation.after,
            };
            let _ = self.apply_snapshot(snapshot);
        }
    }

    fn apply_snapshot(&mut self, snapshot: &StateSnapshot) -> bool {
        if snapshot.data.is_empty() {
            return false;
        }

        if let Ok((entity_id, transform)) = snapshot.to_value::<(EntityId, Transform)>() {
            if let Some(entity) = self.scene.get_mut(&entity_id) {
                entity.transform = transform;
                return true;
            }
        }

        if let Ok(pairs) = snapshot.to_value::<Vec<(EntityId, Transform)>>() {
            if !pairs.is_empty() {
                self.apply_transform_pairs(pairs);
                return true;
            }
        }

        if let Ok(pairs) = snapshot.to_value::<Vec<(EntityId, TransformData)>>() {
            if !pairs.is_empty() {
                self.apply_transform_data_pairs(pairs);
                return true;
            }
        }

        if let Ok((entity_id, name)) = snapshot.to_value::<(EntityId, String)>() {
            if let Some(entity) = self.scene.get_mut(&entity_id) {
                entity.name = name;
                return true;
            }
        }

        if let Ok(edit) = snapshot.to_value::<PropertyEditSnapshot>() {
            if self.apply_property_snapshot(edit) {
                return true;
            }
        }

        if let Ok(edits) = snapshot.to_value::<Vec<PropertyEditSnapshot>>() {
            if !edits.is_empty() {
                let mut applied = false;
                for edit in edits {
                    applied |= self.apply_property_snapshot(edit);
                }
                return applied;
            }
        }

        if let Ok(pairs) = snapshot.to_value::<Vec<(EntityId, Option<EntityId>)>>() {
            if !pairs.is_empty() {
                for (entity_id, parent) in pairs {
                    self.set_entity_parent(entity_id, parent);
                }
                return true;
            }
        }

        if let Ok(entities) = snapshot.to_value::<Vec<(EntityId, EntityData)>>() {
            if !entities.is_empty() {
                self.restore_entities(entities);
                return true;
            }
        }

        if let Ok(ids) = snapshot.to_value::<Vec<EntityId>>() {
            if !ids.is_empty() {
                self.remove_entities_by_id(&ids);
                return true;
            }
        }

        false
    }

    pub(crate) fn collect_with_descendants(&self, ids: &[EntityId]) -> Vec<EntityId> {
        let mut visited = HashSet::new();
        let mut collected = Vec::new();

        for id in ids {
            self.collect_descendants(*id, &mut collected, &mut visited);
        }

        collected
    }

    fn collect_descendants(
        &self,
        id: EntityId,
        collected: &mut Vec<EntityId>,
        visited: &mut HashSet<EntityId>,
    ) {
        if !visited.insert(id) {
            return;
        }

        collected.push(id);

        if let Some(entity) = self.scene.get(&id) {
            for child_id in &entity.children {
                self.collect_descendants(*child_id, collected, visited);
            }
        }
    }

    fn remove_entities_by_id(&mut self, ids: &[EntityId]) {
        for id in ids {
            if let Some(entity) = self.scene.remove(id) {
                if let Some(parent_id) = entity.parent {
                    if let Some(parent) = self.scene.get_mut(&parent_id) {
                        parent.children.retain(|child| child != id);
                    }
                }
            }
            self.selection.remove(id);
        }
    }

    fn restore_entities(&mut self, entities: Vec<(EntityId, EntityData)>) {
        for (id, data) in entities.iter() {
            self.scene.entities.insert(*id, data.clone());
        }

        for (id, data) in entities {
            self.attach_to_parent(id, data.parent);
        }
    }

    fn apply_transform_pairs(&mut self, pairs: Vec<(EntityId, Transform)>) {
        for (entity_id, transform) in pairs {
            if let Some(entity) = self.scene.get_mut(&entity_id) {
                entity.transform = transform;
            }
        }
    }

    fn apply_transform_data_pairs(&mut self, pairs: Vec<(EntityId, TransformData)>) {
        for (entity_id, transform) in pairs {
            if let Some(entity) = self.scene.get_mut(&entity_id) {
                entity.transform = Transform {
                    position: transform.position,
                    rotation: [transform.rotation[0], transform.rotation[1], transform.rotation[2]],
                    scale: transform.scale,
                };
            }
        }
    }

    fn apply_property_snapshot(&mut self, snapshot: PropertyEditSnapshot) -> bool {
        let Some(entity) = self.scene.get_mut(&snapshot.entity) else {
            return false;
        };

        let component = snapshot.component_type.as_str();
        let field = snapshot.field_path.as_str();

        if component.eq_ignore_ascii_case("Transform") || component.eq_ignore_ascii_case("transform") {
            if let Ok(value) = bincode::deserialize::<[f32; 3]>(&snapshot.value) {
                match field {
                    "position" => entity.transform.position = value,
                    "rotation" => entity.transform.rotation = value,
                    "scale" => entity.transform.scale = value,
                    "position.x" => entity.transform.position[0] = value[0],
                    "position.y" => entity.transform.position[1] = value[1],
                    "position.z" => entity.transform.position[2] = value[2],
                    "rotation.x" => entity.transform.rotation[0] = value[0],
                    "rotation.y" => entity.transform.rotation[1] = value[1],
                    "rotation.z" => entity.transform.rotation[2] = value[2],
                    "scale.x" => entity.transform.scale[0] = value[0],
                    "scale.y" => entity.transform.scale[1] = value[1],
                    "scale.z" => entity.transform.scale[2] = value[2],
                    _ => return false,
                }
                return true;
            }

            if let Ok(value) = bincode::deserialize::<f32>(&snapshot.value) {
                match field {
                    "position.x" => entity.transform.position[0] = value,
                    "position.y" => entity.transform.position[1] = value,
                    "position.z" => entity.transform.position[2] = value,
                    "rotation.x" => entity.transform.rotation[0] = value,
                    "rotation.y" => entity.transform.rotation[1] = value,
                    "rotation.z" => entity.transform.rotation[2] = value,
                    "scale.x" => entity.transform.scale[0] = value,
                    "scale.y" => entity.transform.scale[1] = value,
                    "scale.z" => entity.transform.scale[2] = value,
                    _ => return false,
                }
                return true;
            }
        }

        if component.eq_ignore_ascii_case("Entity") && field.eq_ignore_ascii_case("name") {
            if let Ok(name) = bincode::deserialize::<String>(&snapshot.value) {
                entity.name = name;
                return true;
            }
        }

        if component.eq_ignore_ascii_case("Entity") && field.eq_ignore_ascii_case("active") {
            if let Ok(active) = bincode::deserialize::<bool>(&snapshot.value) {
                entity.active = active;
                return true;
            }
        }

        if component.eq_ignore_ascii_case("Entity") && field.eq_ignore_ascii_case("is_static") {
            if let Ok(is_static) = bincode::deserialize::<bool>(&snapshot.value) {
                entity.is_static = is_static;
                return true;
            }
        }

        false
    }

    fn set_entity_parent(&mut self, entity_id: EntityId, new_parent: Option<EntityId>) {
        // First pass: check if entity exists and get old parent
        let old_parent = {
            let Some(entity) = self.scene.get(&entity_id) else {
                return;
            };
            if entity.parent == new_parent {
                return;
            }
            entity.parent
        };

        // Second pass: remove from old parent's children
        if let Some(old_parent_id) = old_parent {
            if let Some(parent) = self.scene.get_mut(&old_parent_id) {
                parent.children.retain(|id| id != &entity_id);
            }
        }

        // Third pass: update entity's parent reference
        if let Some(entity) = self.scene.get_mut(&entity_id) {
            entity.parent = new_parent;
        }

        // Fourth pass: add to new parent's children
        self.attach_to_parent(entity_id, new_parent);
    }

    fn attach_to_parent(&mut self, entity_id: EntityId, parent_id: Option<EntityId>) {
        if let Some(parent_id) = parent_id {
            if let Some(parent) = self.scene.get_mut(&parent_id) {
                if !parent.children.contains(&entity_id) {
                    parent.children.push(entity_id);
                }
            }
        }
    }

    /// Execute an editor command and commit its undo/redo snapshot
    pub fn execute_command<C: EditorCommand>(&mut self, command: &C) -> Result<(), crate::commands::CommandError> {
        let (before, after) = command.snapshots(self)?;
        let op_id = self.history.begin_operation(command.description());
        command.execute(self)?;

        let operation = Operation::new(op_id, command.description().to_string(), before, after);
        let mut group = OperationGroup::new(op_id, command.description().to_string());
        group.add_operation(operation);
        self.history.commit(group)?;
        Ok(())
    }

    /// Set transforms for multiple entities as a single undo operation
    pub fn set_transforms_bulk(
        &mut self,
        entities: &[EntityId],
        transforms: &[Transform],
        description: &str,
    ) {
        if entities.is_empty() || entities.len() != transforms.len() {
            return;
        }

        let mut ids = Vec::new();
        let mut before = Vec::new();
        let mut after = Vec::new();

        for (entity_id, new_transform) in entities.iter().copied().zip(transforms.iter().cloned()) {
            let Some(entity) = self.scene.get(&entity_id) else {
                continue;
            };
            if entity.transform == new_transform {
                continue;
            }
            ids.push(entity_id);
            before.push(TransformData::from(entity.transform.clone()));
            after.push(TransformData::from(new_transform));
        }

        if ids.is_empty() {
            return;
        }

        let command = TransformCommand::new(ids, before, after, description);
        if let Err(err) = self.execute_command(&command) {
            tracing::warn!("Bulk transform failed: {}", err);
        }
    }

    /// Set transforms for multiple entities with pre-captured before values
    /// Useful when live preview has already been applied and we want to commit to undo history
    pub fn set_transforms_bulk_with_before(
        &mut self,
        entities: &[EntityId],
        before_transforms: &[Transform],
        after_transforms: &[Transform],
        description: &str,
    ) {
        if entities.is_empty()
            || entities.len() != before_transforms.len()
            || entities.len() != after_transforms.len()
        {
            return;
        }

        let mut ids = Vec::new();
        let mut before = Vec::new();
        let mut after = Vec::new();

        for i in 0..entities.len() {
            let entity_id = entities[i];
            let before_transform = &before_transforms[i];
            let after_transform = &after_transforms[i];

            // Skip if no actual change
            if before_transform == after_transform {
                continue;
            }

            // Ensure entity still exists
            if self.scene.get(&entity_id).is_none() {
                continue;
            }

            ids.push(entity_id);
            before.push(TransformData::from(before_transform.clone()));
            after.push(TransformData::from(after_transform.clone()));
        }

        if ids.is_empty() {
            return;
        }

        let command = TransformCommand::new(ids, before, after, description);
        if let Err(err) = self.execute_command(&command) {
            tracing::warn!("Bulk transform with before failed: {}", err);
        }
    }

    /// Unpack a prefab instance (one level only)
    /// This removes the prefab link but keeps the entities as regular entities
    pub fn unpack_prefab(&mut self, root_entity_id: EntityId) {
        if !self.prefab_manager.is_prefab_root(root_entity_id) {
            tracing::warn!("Entity {:?} is not a prefab root", root_entity_id);
            return;
        }

        // Simply unregister the instance - entities remain in the scene
        self.prefab_manager.unregister_instance(root_entity_id);
        self.dirty = true;
        tracing::info!("Unpacked prefab instance {:?}", root_entity_id);
    }

    /// Unpack a prefab instance completely (including nested prefabs)
    pub fn unpack_prefab_completely(&mut self, root_entity_id: EntityId) {
        if !self.prefab_manager.is_prefab_root(root_entity_id) {
            tracing::warn!("Entity {:?} is not a prefab root", root_entity_id);
            return;
        }

        // Get all entity IDs in this instance
        let entity_ids: Vec<EntityId> = if let Some(instance) = self.prefab_manager.get_instance(root_entity_id) {
            instance.all_entity_ids().into_iter().collect()
        } else {
            return;
        };

        // Unregister the main instance
        self.prefab_manager.unregister_instance(root_entity_id);

        // Find and unregister any nested prefab instances
        let nested_roots: Vec<EntityId> = entity_ids
            .iter()
            .filter(|id| **id != root_entity_id && self.prefab_manager.is_prefab_root(**id))
            .copied()
            .collect();

        for nested_root in nested_roots {
            self.prefab_manager.unregister_instance(nested_root);
        }

        self.dirty = true;
        tracing::info!("Unpacked prefab instance {:?} completely", root_entity_id);
    }

    /// Check if we're currently editing a prefab
    pub fn is_editing_prefab(&self) -> bool {
        self.editing_prefab.is_some()
    }

    /// Get the path of the prefab being edited
    pub fn editing_prefab_path(&self) -> Option<&PathBuf> {
        self.editing_prefab.as_ref().map(|s| &s.prefab_path)
    }

    /// Enter prefab editing mode
    pub fn enter_prefab_edit_mode(&mut self, prefab_path: &PathBuf) -> Result<(), String> {
        if self.editing_prefab.is_some() {
            return Err("Already editing a prefab".to_string());
        }

        // Load the prefab
        let prefab = crate::prefab::Prefab::load(prefab_path)
            .map_err(|e| format!("Failed to load prefab: {}", e))?;

        // Backup current state
        let editing_state = PrefabEditingState {
            prefab_path: prefab_path.clone(),
            scene_backup: self.scene.clone(),
            selection_backup: self.selection.clone(),
            prefab_dirty: false,
        };

        // Load prefab entities into the scene
        let (entities, _id_mapping) = prefab.instantiate_flat();

        // Clear scene and load prefab content
        self.scene = SceneData::new();
        self.selection.clear();
        self.history.clear();

        // Add prefab entities to scene
        for entity in entities {
            self.scene.add_entity(entity);
        }

        self.editing_prefab = Some(editing_state);
        tracing::info!("Entered prefab edit mode: {:?}", prefab_path);
        Ok(())
    }

    /// Exit prefab editing mode without saving
    pub fn exit_prefab_edit_mode(&mut self, save: bool) -> Result<(), String> {
        let Some(editing_state) = self.editing_prefab.take() else {
            return Err("Not in prefab edit mode".to_string());
        };

        if save && editing_state.prefab_dirty {
            self.save_prefab_from_scene(&editing_state.prefab_path)?;
        }

        // Restore the main scene
        self.scene = editing_state.scene_backup;
        self.selection = editing_state.selection_backup;
        self.history.clear();

        tracing::info!("Exited prefab edit mode");
        Ok(())
    }

    /// Save current scene content as a prefab
    pub fn save_prefab_from_scene(&mut self, path: &PathBuf) -> Result<(), String> {
        // Get all root entities
        let roots = self.scene.root_entities();
        if roots.is_empty() {
            return Err("No entities in scene to save as prefab".to_string());
        }

        // Use the first root as the prefab root
        let root_id = roots[0];
        let root_entity = self.scene.get(&root_id)
            .ok_or("Root entity not found")?;

        // Build entity map for hierarchy
        let entity_map: std::collections::HashMap<EntityId, EntityData> = self.scene.entities
            .iter()
            .map(|(id, data)| (*id, data.clone()))
            .collect();

        // Create prefab from entities
        let name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Prefab")
            .to_string();

        let prefab = crate::prefab::Prefab::from_entities(name, root_entity, &entity_map);

        // Save to disk
        prefab.save(path)
            .map_err(|e| format!("Failed to save prefab: {}", e))?;

        // Mark as clean
        if let Some(ref mut editing_state) = self.editing_prefab {
            editing_state.prefab_dirty = false;
        }

        tracing::info!("Saved prefab to {:?}", path);
        Ok(())
    }

    /// Mark the prefab as having unsaved changes
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn mark_prefab_dirty(&mut self) {
        if let Some(ref mut editing_state) = self.editing_prefab {
            editing_state.prefab_dirty = true;
        }
    }

    /// Check if the editing prefab has unsaved changes
    pub fn prefab_has_unsaved_changes(&self) -> bool {
        self.editing_prefab.as_ref().map(|s| s.prefab_dirty).unwrap_or(false)
    }

    /// Track a property override on a prefab instance
    /// Returns true if the entity is part of a prefab instance
    pub fn track_prefab_override(
        &mut self,
        entity_id: EntityId,
        property_path: &str,
        value: serde_json::Value,
    ) -> bool {
        // Find the prefab instance containing this entity
        let instance_root = {
            let Some(instance) = self.prefab_manager.find_instance_containing(entity_id) else {
                return false;
            };
            instance.root_entity_id
        };

        // Get the local ID for this entity within the prefab
        let local_id = {
            let Some(instance) = self.prefab_manager.get_instance(instance_root) else {
                return false;
            };
            match instance.get_local_id(entity_id) {
                Some(id) => id.to_string(),
                None => return false,
            }
        };

        // Create the override
        let override_ = crate::prefab::PropertyOverride {
            entity_path: local_id,
            property_path: property_path.to_string(),
            value: value.clone(),
        };

        // Add the override to the instance
        if let Some(instance) = self.prefab_manager.get_instance_mut(instance_root) {
            tracing::debug!(
                "Tracked override for entity {:?}: {} = {:?}",
                entity_id,
                property_path,
                value
            );
            instance.set_override(override_);
        }

        true
    }

    /// Check if a property is overridden on a prefab instance entity
    pub fn is_property_overridden(&self, entity_id: EntityId, property_path: &str) -> bool {
        let Some(instance) = self.prefab_manager.find_instance_containing(entity_id) else {
            return false;
        };

        let Some(local_id) = instance.get_local_id(entity_id) else {
            return false;
        };

        instance.is_overridden(&local_id.to_string(), property_path)
    }

    /// Revert a specific property override on a prefab instance entity
    pub fn revert_property_override(&mut self, entity_id: EntityId, property_path: &str) -> bool {
        // Find the instance and get needed data
        let (instance_root, local_id, prefab_path) = {
            let Some(instance) = self.prefab_manager.find_instance_containing(entity_id) else {
                return false;
            };
            let Some(local_id) = instance.get_local_id(entity_id) else {
                return false;
            };
            (instance.root_entity_id, local_id, instance.prefab_path.clone())
        };

        // Load the prefab to get the original value
        let prefab = match crate::prefab::Prefab::load(&prefab_path) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("Failed to load prefab for revert: {}", e);
                return false;
            }
        };

        // Find the entity in the prefab by local_id
        let prefab_entity = self.find_prefab_entity_by_local_id(&prefab.root, local_id);
        let Some(prefab_entity) = prefab_entity else {
            tracing::warn!("Could not find prefab entity with local_id {}", local_id);
            return false;
        };

        // Revert the property value
        let reverted = self.revert_property_from_prefab(entity_id, property_path, prefab_entity);

        // Remove the override from the instance
        if reverted {
            if let Some(instance) = self.prefab_manager.get_instance_mut(instance_root) {
                instance.remove_override(&local_id.to_string(), property_path);
            }
        }

        reverted
    }

    fn find_prefab_entity_by_local_id<'a>(
        &self,
        entity: &'a crate::prefab::PrefabEntity,
        local_id: u32,
    ) -> Option<&'a crate::prefab::PrefabEntity> {
        if entity.local_id == local_id {
            return Some(entity);
        }
        for child in &entity.children {
            if let Some(found) = self.find_prefab_entity_by_local_id(child, local_id) {
                return Some(found);
            }
        }
        None
    }

    fn revert_property_from_prefab(
        &mut self,
        entity_id: EntityId,
        property_path: &str,
        prefab_entity: &crate::prefab::PrefabEntity,
    ) -> bool {
        let Some(entity) = self.scene.get_mut(&entity_id) else {
            return false;
        };

        // Handle common property paths
        match property_path {
            "name" => {
                entity.name = prefab_entity.name.clone();
                true
            }
            "transform.position" | "transform.position.x" | "transform.position.y" | "transform.position.z" => {
                entity.transform.position = prefab_entity.transform.position;
                true
            }
            "transform.rotation" | "transform.rotation.x" | "transform.rotation.y" | "transform.rotation.z" => {
                entity.transform.rotation = prefab_entity.transform.rotation;
                true
            }
            "transform.scale" | "transform.scale.x" | "transform.scale.y" | "transform.scale.z" => {
                entity.transform.scale = prefab_entity.transform.scale;
                true
            }
            "active" => {
                entity.active = true; // Default for prefab entities
                true
            }
            "is_static" => {
                entity.is_static = false; // Default for prefab entities
                true
            }
            _ => {
                tracing::warn!("Revert not implemented for property: {}", property_path);
                false
            }
        }
    }

    /// Revert all overrides on a prefab instance entity
    pub fn revert_all_overrides(&mut self, entity_id: EntityId) -> bool {
        let Some(instance) = self.prefab_manager.find_instance_containing(entity_id) else {
            return false;
        };

        let root_id = instance.root_entity_id;
        let prefab_path = instance.prefab_path.clone();

        // Load the prefab
        let prefab = match crate::prefab::Prefab::load(&prefab_path) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("Failed to load prefab for revert: {}", e);
                return false;
            }
        };

        // Get the local ID for this entity
        let Some(instance) = self.prefab_manager.get_instance(root_id) else {
            return false;
        };
        let Some(local_id) = instance.get_local_id(entity_id) else {
            return false;
        };

        // Find the prefab entity
        let Some(prefab_entity) = self.find_prefab_entity_by_local_id(&prefab.root, local_id) else {
            return false;
        };

        // Revert all properties
        if let Some(entity) = self.scene.get_mut(&entity_id) {
            entity.name = prefab_entity.name.clone();
            entity.transform = prefab_entity.transform.clone();
            entity.components = prefab_entity.components.clone();
        }

        // Clear all overrides for this entity
        if let Some(instance) = self.prefab_manager.get_instance_mut(root_id) {
            instance.overrides.retain(|o| o.entity_path != local_id.to_string());
        }

        self.dirty = true;
        true
    }

    /// Get all overrides for an entity (if it's part of a prefab instance)
    pub fn get_entity_overrides(&self, entity_id: EntityId) -> Vec<String> {
        let Some(instance) = self.prefab_manager.find_instance_containing(entity_id) else {
            return Vec::new();
        };

        let Some(local_id) = instance.get_local_id(entity_id) else {
            return Vec::new();
        };

        let local_id_str = local_id.to_string();
        instance.overrides
            .iter()
            .filter(|o| o.entity_path == local_id_str)
            .map(|o| o.property_path.clone())
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
enum HistoryDirection {
    Undo,
    Redo,
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}
