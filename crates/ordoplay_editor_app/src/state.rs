// SPDX-License-Identifier: MIT OR Apache-2.0
//! Editor state management.
//!
//! This module contains the core editor state including selection,
//! undo/redo history, and scene management.

use crate::history::{History, Operation, OperationGroup, OperationID, StateSnapshot};
use crate::tools::GizmoMode;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
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
    pub fn union(&mut self, other: &Selection) {
        for id in &other.entities {
            self.add(*id);
        }
    }

    /// Difference with another selection
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
    pub fn iter(&self) -> impl Iterator<Item = &EntityId> {
        self.entities.iter()
    }

    /// Get the first selected entity
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
}

impl EditorState {
    /// Create a new editor state
    pub fn new() -> Self {
        let mut scene = SceneData::new();

        // Add some test entities
        let cube = scene.add_entity(EntityData::new("Cube"));
        let sphere = scene.add_entity(EntityData {
            name: "Sphere".to_string(),
            transform: Transform {
                position: [3.0, 0.0, 0.0],
                ..Default::default()
            },
            ..Default::default()
        });
        let light = scene.add_entity(EntityData {
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
        // Serialize scene to RON format
        let ron_str = ron::ser::to_string_pretty(&self.scene, ron::ser::PrettyConfig::default())
            .map_err(|e| format!("Serialization error: {}", e))?;

        // Write to file
        std::fs::write(path, ron_str)
            .map_err(|e| format!("File write error: {}", e))?;

        self.scene_path = Some(path.to_path_buf());
        self.dirty = false;

        // Add to recent scenes
        self.add_to_recent(path.to_path_buf());

        tracing::info!("Saved scene to {:?}", path);
        Ok(())
    }

    /// Load a scene from a file
    pub fn load_scene(&mut self, path: &std::path::Path) -> Result<(), String> {
        // Read file contents
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("File read error: {}", e))?;

        // Deserialize from RON
        let scene: SceneData = ron::from_str(&content)
            .map_err(|e| format!("Deserialization error: {}", e))?;

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
    pub fn clear_recent_scenes(&mut self) {
        self.recent_scenes.clear();
    }

    /// Get the scene file name (for window title)
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

        // TODO: Create delete command with undo support
        tracing::info!("Deleting {} entities", self.selection.len());
        self.selection.clear();
        self.dirty = true;
    }

    /// Duplicate selected entities
    pub fn duplicate_selected(&mut self) {
        if self.selection.is_empty() {
            return;
        }

        // TODO: Create duplicate command with undo support
        tracing::info!("Duplicating {} entities", self.selection.len());
        self.dirty = true;
    }

    /// Mark the scene as modified
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Set entity transform with undo support
    pub fn set_transform(&mut self, entity_id: EntityId, new_transform: Transform, description: &str) {
        // Get old transform for undo
        let old_transform = match self.scene.get(&entity_id) {
            Some(data) => data.transform.clone(),
            None => return,
        };

        // Don't create operation if nothing changed
        if old_transform == new_transform {
            return;
        }

        // Apply the change
        if let Some(data) = self.scene.get_mut(&entity_id) {
            data.transform = new_transform.clone();
        }

        // Create undo operation
        if let (Ok(before), Ok(after)) = (
            StateSnapshot::from_value(&(entity_id, old_transform)),
            StateSnapshot::from_value(&(entity_id, new_transform)),
        ) {
            let op_id = self.history.begin_operation(description);
            let operation = Operation::new(op_id, description.to_string(), before, after);
            let mut group = OperationGroup::new(op_id, description.to_string());
            group.add_operation(operation);
            let _ = self.history.commit(group);
        }

        self.dirty = true;
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

        if let Some(data) = self.scene.get_mut(&entity_id) {
            data.name = new_name.clone();
        }

        // Create undo operation
        let description = format!("Rename {} to {}", old_name, new_name);
        if let (Ok(before), Ok(after)) = (
            StateSnapshot::from_value(&(entity_id, old_name)),
            StateSnapshot::from_value(&(entity_id, new_name)),
        ) {
            let op_id = self.history.begin_operation(&description);
            let operation = Operation::new(op_id, description.clone(), before, after);
            let mut group = OperationGroup::new(op_id, description);
            group.add_operation(operation);
            let _ = self.history.commit(group);
        }

        self.dirty = true;
    }
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}
