// SPDX-License-Identifier: MIT OR Apache-2.0
//! Editor state management.
//!
//! This module contains the core editor state including selection,
//! undo/redo history, and scene management.

use crate::commands::{EditorCommand, PropertyEditSnapshot, TransformData};
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
            pending_panels: Vec::new(),
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

        let ids = self.selection.entities.clone();
        self.delete_entities(&ids);
    }

    /// Duplicate selected entities
    pub fn duplicate_selected(&mut self) {
        if self.selection.is_empty() {
            return;
        }

        let ids = self.selection.entities.clone();
        let new_ids = self.duplicate_entities(&ids);
        if !new_ids.is_empty() {
            self.selection.clear();
            for id in new_ids {
                self.selection.add(id);
            }
        }
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

    /// Delete a set of entities (including their descendants) with undo support
    pub fn delete_entities(&mut self, ids: &[EntityId]) {
        let to_remove = self.collect_with_descendants(ids);
        if to_remove.is_empty() {
            return;
        }

        let mut removed_entities = Vec::new();
        for id in &to_remove {
            if let Some(entity) = self.scene.get(id).cloned() {
                removed_entities.push((*id, entity));
            }
        }

        self.remove_entities_by_id(&to_remove);
        self.selection.clear();
        self.dirty = true;

        let description = "Delete Entities";
        if let (Ok(before), Ok(after)) = (
            StateSnapshot::from_value(&removed_entities),
            StateSnapshot::from_value(&to_remove),
        ) {
            let op_id = self.history.begin_operation(description);
            let operation = Operation::new(op_id, description.to_string(), before, after);
            let mut group = OperationGroup::new(op_id, description.to_string());
            group.add_operation(operation);
            let _ = self.history.commit(group);
        }
    }

    /// Duplicate a set of entities with undo support
    pub fn duplicate_entities(&mut self, ids: &[EntityId]) -> Vec<EntityId> {
        let mut new_entities = Vec::new();
        let mut new_ids = Vec::new();

        for id in ids {
            let Some(original) = self.scene.get(id).cloned() else {
                continue;
            };

            let mut duplicate = original.clone();
            duplicate.name = format!("{} (Copy)", original.name);
            duplicate.children = Vec::new();

            let new_id = EntityId::new();
            if let Some(parent_id) = duplicate.parent {
                if let Some(parent) = self.scene.get_mut(&parent_id) {
                    if !parent.children.contains(&new_id) {
                        parent.children.push(new_id);
                    }
                }
            }

            self.scene.entities.insert(new_id, duplicate.clone());
            new_entities.push((new_id, duplicate));
            new_ids.push(new_id);
        }

        if new_ids.is_empty() {
            return Vec::new();
        }

        self.dirty = true;

        let description = "Duplicate Entities";
        if let (Ok(before), Ok(after)) = (
            StateSnapshot::from_value(&new_ids),
            StateSnapshot::from_value(&new_entities),
        ) {
            let op_id = self.history.begin_operation(description);
            let operation = Operation::new(op_id, description.to_string(), before, after);
            let mut group = OperationGroup::new(op_id, description.to_string());
            group.add_operation(operation);
            let _ = self.history.commit(group);
        }

        new_ids
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

        let mut before = Vec::new();
        let mut after = Vec::new();

        for (entity_id, new_transform) in entities.iter().copied().zip(transforms.iter().cloned()) {
            let Some(entity) = self.scene.get(&entity_id) else {
                continue;
            };
            before.push((entity_id, entity.transform.clone()));
            after.push((entity_id, new_transform.clone()));
        }

        if before.is_empty() || before == after {
            return;
        }

        for (entity_id, transform) in after.iter() {
            if let Some(entity) = self.scene.get_mut(entity_id) {
                entity.transform = transform.clone();
            }
        }

        if let (Ok(before_snapshot), Ok(after_snapshot)) = (
            StateSnapshot::from_value(&before),
            StateSnapshot::from_value(&after),
        ) {
            let op_id = self.history.begin_operation(description);
            let operation = Operation::new(op_id, description.to_string(), before_snapshot, after_snapshot);
            let mut group = OperationGroup::new(op_id, description.to_string());
            group.add_operation(operation);
            let _ = self.history.commit(group);
        }

        self.dirty = true;
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
