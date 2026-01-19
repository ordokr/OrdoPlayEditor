// SPDX-License-Identifier: MIT OR Apache-2.0
//! Editor commands for undo/redo support.
//!
//! Commands encapsulate editor operations and integrate with
//! the ordoplay_editor::History system.

use crate::history::{HistoryError, Operation, OperationID, StateSnapshot};
use crate::state::{EditorState, EntityData, EntityId, Transform};
use serde::{Deserialize, Serialize};

/// Trait for editor commands that can be undone/redone
pub trait EditorCommand: Send + Sync {
    /// Get a description of this command
    fn description(&self) -> &str;

    /// Execute the command
    fn execute(&self, state: &mut EditorState) -> Result<(), CommandError>;

    /// Build before/after snapshots for undo/redo
    fn snapshots(&self, state: &EditorState) -> Result<(StateSnapshot, StateSnapshot), CommandError>;

    /// Create an operation for the undo system
    fn to_operation(&self, id: OperationID) -> Result<Operation, CommandError>;
}

/// Error type for command execution
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    /// History error
    #[error("History error: {0}")]
    History(#[from] HistoryError),

    /// Entity not found
    #[error("Entity not found: {0:?}")]
    EntityNotFound(EntityId),

    /// Invalid operation
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

/// Transform data for position/rotation/scale
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformData {
    /// Position (x, y, z)
    pub position: [f32; 3],
    /// Rotation quaternion (x, y, z, w)
    pub rotation: [f32; 4],
    /// Scale (x, y, z)
    pub scale: [f32; 3],
}

impl Default for TransformData {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

/// Command to transform entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformCommand {
    /// Entities being transformed
    pub entities: Vec<EntityId>,
    /// Transforms before the operation
    pub before: Vec<TransformData>,
    /// Transforms after the operation
    pub after: Vec<TransformData>,
    /// Description of the transform
    pub description: String,
}

impl TransformCommand {
    /// Create a new transform command
    pub fn new(
        entities: Vec<EntityId>,
        before: Vec<TransformData>,
        after: Vec<TransformData>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            entities,
            before,
            after,
            description: description.into(),
        }
    }
}

impl EditorCommand for TransformCommand {
    fn description(&self) -> &str {
        &self.description
    }

    fn execute(&self, state: &mut EditorState) -> Result<(), CommandError> {
        if self.entities.len() != self.after.len() {
            return Err(CommandError::InvalidOperation(
                "Transform data length mismatch".to_string(),
            ));
        }

        for (entity_id, transform) in self.entities.iter().zip(self.after.iter()) {
            let Some(entity) = state.scene.get_mut(entity_id) else {
                return Err(CommandError::EntityNotFound(*entity_id));
            };
            entity.transform = to_editor_transform(transform);
        }

        state.dirty = true;
        Ok(())
    }

    fn snapshots(&self, _state: &EditorState) -> Result<(StateSnapshot, StateSnapshot), CommandError> {
        if self.entities.len() != self.before.len() || self.entities.len() != self.after.len() {
            return Err(CommandError::InvalidOperation(
                "Transform data length mismatch".to_string(),
            ));
        }

        let before: Vec<_> = self.entities.iter().copied().zip(self.before.iter().cloned()).collect();
        let after: Vec<_> = self.entities.iter().copied().zip(self.after.iter().cloned()).collect();

        Ok((StateSnapshot::from_value(&before)?, StateSnapshot::from_value(&after)?))
    }

    fn to_operation(&self, id: OperationID) -> Result<Operation, CommandError> {
        let before = StateSnapshot::from_value(&self.before)?;
        let after = StateSnapshot::from_value(&self.after)?;
        Ok(Operation::new(id, self.description.clone(), before, after))
    }
}

/// Command to spawn an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnCommand {
    /// ID for the new entity
    pub entity_id: EntityId,
    /// Prefab or template path
    pub prefab_path: Option<String>,
    /// Initial transform
    pub transform: TransformData,
    /// Whether to select the spawned entity
    pub select: bool,
}

impl SpawnCommand {
    /// Create a new spawn command
    pub fn new(entity_id: EntityId, transform: TransformData) -> Self {
        Self {
            entity_id,
            prefab_path: None,
            transform,
            select: true,
        }
    }

    /// Set the prefab path
    pub fn with_prefab(mut self, path: impl Into<String>) -> Self {
        self.prefab_path = Some(path.into());
        self
    }
}

impl EditorCommand for SpawnCommand {
    fn description(&self) -> &str {
        "Spawn Entity"
    }

    fn execute(&self, state: &mut EditorState) -> Result<(), CommandError> {
        if state.scene.entities.contains_key(&self.entity_id) {
            return Err(CommandError::InvalidOperation(format!(
                "Entity already exists: {:?}",
                self.entity_id
            )));
        }

        let name = self
            .prefab_path
            .as_ref()
            .and_then(|path| std::path::Path::new(path).file_stem())
            .and_then(|name| name.to_str())
            .unwrap_or("New Entity");

        let mut data = EntityData::new(name);
        data.transform = to_editor_transform(&self.transform);

        if !state.scene.insert_entity(self.entity_id, data) {
            return Err(CommandError::InvalidOperation(
                "Failed to insert entity".to_string(),
            ));
        }

        if self.select {
            state.selection.clear();
            state.selection.add(self.entity_id);
        }

        state.dirty = true;
        Ok(())
    }

    fn snapshots(&self, _state: &EditorState) -> Result<(StateSnapshot, StateSnapshot), CommandError> {
        let data = self.entity_data();
        let before = StateSnapshot::from_value(&vec![self.entity_id])?;
        let after = StateSnapshot::from_value(&vec![(self.entity_id, data)])?;
        Ok((before, after))
    }

    fn to_operation(&self, id: OperationID) -> Result<Operation, CommandError> {
        // For spawn, "before" is empty, "after" is the entity data
        let before = StateSnapshot::new(vec![]);
        let after = StateSnapshot::from_value(self)?;
        Ok(Operation::new(
            id,
            self.description().to_string(),
            before,
            after,
        ))
    }
}

/// Command to delete entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteCommand {
    /// Entities to delete
    pub entities: Vec<EntityId>,
    /// Serialized entity data for undo
    pub entity_data: Vec<u8>,
}

impl DeleteCommand {
    /// Create a new delete command
    pub fn new(entities: Vec<EntityId>) -> Self {
        Self {
            entities,
            entity_data: vec![],
        }
    }
}

impl EditorCommand for DeleteCommand {
    fn description(&self) -> &str {
        "Delete Entities"
    }

    fn execute(&self, state: &mut EditorState) -> Result<(), CommandError> {
        if self.entities.is_empty() {
            return Ok(());
        }

        state.delete_entities(&self.entities);
        Ok(())
    }

    fn snapshots(&self, state: &EditorState) -> Result<(StateSnapshot, StateSnapshot), CommandError> {
        let ids = state.collect_with_descendants(&self.entities);
        let mut removed_entities = Vec::new();

        for id in &ids {
            let Some(entity) = state.scene.get(id).cloned() else {
                return Err(CommandError::EntityNotFound(*id));
            };
            removed_entities.push((*id, entity));
        }

        let before = StateSnapshot::from_value(&removed_entities)?;
        let after = StateSnapshot::from_value(&ids)?;
        Ok((before, after))
    }

    fn to_operation(&self, id: OperationID) -> Result<Operation, CommandError> {
        // For delete, "before" is entity data, "after" is empty
        let before = StateSnapshot::new(self.entity_data.clone());
        let after = StateSnapshot::new(vec![]);
        Ok(Operation::new(
            id,
            self.description().to_string(),
            before,
            after,
        ))
    }
}

/// Command to duplicate entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateCommand {
    /// Source entities
    pub source_entities: Vec<EntityId>,
    /// New entity IDs for duplicates
    pub new_entities: Vec<EntityId>,
    /// Whether to select the duplicates
    pub select: bool,
}

impl DuplicateCommand {
    /// Create a new duplicate command
    pub fn new(source_entities: Vec<EntityId>) -> Self {
        let new_entities = source_entities.iter().map(|_| EntityId::new()).collect();
        Self {
            source_entities,
            new_entities,
            select: true,
        }
    }
}

impl EditorCommand for DuplicateCommand {
    fn description(&self) -> &str {
        "Duplicate Entities"
    }

    fn execute(&self, state: &mut EditorState) -> Result<(), CommandError> {
        if self.source_entities.is_empty() {
            return Ok(());
        }

        if self.source_entities.len() != self.new_entities.len() {
            return Err(CommandError::InvalidOperation(
                "Duplicate data length mismatch".to_string(),
            ));
        }

        for (source_id, new_id) in self.source_entities.iter().zip(self.new_entities.iter()) {
            let Some(original) = state.scene.get(source_id).cloned() else {
                return Err(CommandError::EntityNotFound(*source_id));
            };

            let mut duplicate = original.clone();
            duplicate.name = format!("{} (Copy)", original.name);
            duplicate.children = Vec::new();

            if !state.scene.insert_entity(*new_id, duplicate.clone()) {
                return Err(CommandError::InvalidOperation(
                    "Failed to insert duplicate entity".to_string(),
                ));
            }

            if let Some(parent_id) = duplicate.parent {
                if let Some(parent) = state.scene.get_mut(&parent_id) {
                    if !parent.children.contains(new_id) {
                        parent.children.push(*new_id);
                    }
                }
            }
        }

        if self.select {
            state.selection.clear();
            for id in &self.new_entities {
                state.selection.add(*id);
            }
        }

        state.dirty = true;
        Ok(())
    }

    fn snapshots(&self, state: &EditorState) -> Result<(StateSnapshot, StateSnapshot), CommandError> {
        let duplicates = self.build_duplicates(state)?;
        let before = StateSnapshot::from_value(&self.new_entities)?;
        let after = StateSnapshot::from_value(&duplicates)?;
        Ok((before, after))
    }

    fn to_operation(&self, id: OperationID) -> Result<Operation, CommandError> {
        let before = StateSnapshot::new(vec![]);
        let after = StateSnapshot::from_value(self)?;
        Ok(Operation::new(
            id,
            self.description().to_string(),
            before,
            after,
        ))
    }
}

/// Command to edit a component property
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyEditCommand {
    /// Entity being edited
    pub entity: EntityId,
    /// Component type name
    pub component_type: String,
    /// Field path (e.g., "transform.position.x")
    pub field_path: String,
    /// Serialized old value
    pub old_value: Vec<u8>,
    /// Serialized new value
    pub new_value: Vec<u8>,
}

impl PropertyEditCommand {
    /// Create a new property edit command
    pub fn new(
        entity: EntityId,
        component_type: impl Into<String>,
        field_path: impl Into<String>,
        old_value: Vec<u8>,
        new_value: Vec<u8>,
    ) -> Self {
        Self {
            entity,
            component_type: component_type.into(),
            field_path: field_path.into(),
            old_value,
            new_value,
        }
    }
}

impl EditorCommand for PropertyEditCommand {
    fn description(&self) -> &str {
        "Edit Property"
    }

    fn execute(&self, state: &mut EditorState) -> Result<(), CommandError> {
        let Some(entity) = state.scene.get_mut(&self.entity) else {
            return Err(CommandError::EntityNotFound(self.entity));
        };

        let component = self.component_type.as_str();
        let field = self.field_path.as_str();

        if component.eq_ignore_ascii_case("Transform") || component.eq_ignore_ascii_case("transform") {
            apply_transform_edit(entity, field, &self.new_value)?;
            state.dirty = true;
            return Ok(());
        }

        if component.eq_ignore_ascii_case("Entity") && field.eq_ignore_ascii_case("name") {
            let name: String = bincode::deserialize(&self.new_value)?;
            entity.name = name;
            state.dirty = true;
            return Ok(());
        }

        Err(CommandError::InvalidOperation(format!(
            "Unsupported property edit: {}.{}",
            self.component_type, self.field_path
        )))
    }

    fn snapshots(&self, _state: &EditorState) -> Result<(StateSnapshot, StateSnapshot), CommandError> {
        let before = PropertyEditSnapshot::new(
            self.entity,
            self.component_type.clone(),
            self.field_path.clone(),
            self.old_value.clone(),
        );
        let after = PropertyEditSnapshot::new(
            self.entity,
            self.component_type.clone(),
            self.field_path.clone(),
            self.new_value.clone(),
        );

        Ok((
            StateSnapshot::from_value(&before)?,
            StateSnapshot::from_value(&after)?,
        ))
    }

    fn to_operation(&self, id: OperationID) -> Result<Operation, CommandError> {
        let before = StateSnapshot::new(self.old_value.clone());
        let after = StateSnapshot::new(self.new_value.clone());
        Ok(Operation::new(
            id,
            format!("Edit {}.{}", self.component_type, self.field_path),
            before,
            after,
        ))
    }
}

/// Command to reparent entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReparentCommand {
    /// Entities being reparented
    pub entities: Vec<EntityId>,
    /// Original parents
    pub old_parents: Vec<Option<EntityId>>,
    /// New parent (None for root)
    pub new_parent: Option<EntityId>,
}

impl ReparentCommand {
    /// Create a new reparent command
    pub fn new(
        entities: Vec<EntityId>,
        old_parents: Vec<Option<EntityId>>,
        new_parent: Option<EntityId>,
    ) -> Self {
        Self {
            entities,
            old_parents,
            new_parent,
        }
    }
}

impl EditorCommand for ReparentCommand {
    fn description(&self) -> &str {
        "Reparent Entities"
    }

    fn execute(&self, state: &mut EditorState) -> Result<(), CommandError> {
        if self.entities.is_empty() {
            return Ok(());
        }

        // First pass: collect old parents and validate entities exist
        let mut old_parents_to_update: Vec<EntityId> = Vec::new();
        for entity_id in &self.entities {
            let Some(entity) = state.scene.get(entity_id) else {
                return Err(CommandError::EntityNotFound(*entity_id));
            };
            if let Some(old_parent) = entity.parent {
                old_parents_to_update.push(old_parent);
            }
        }

        // Second pass: remove from old parents' children lists
        for old_parent_id in &old_parents_to_update {
            if let Some(parent) = state.scene.get_mut(old_parent_id) {
                parent.children.retain(|id| !self.entities.contains(id));
            }
        }

        // Third pass: update entity parent references
        for entity_id in &self.entities {
            if let Some(entity) = state.scene.get_mut(entity_id) {
                entity.parent = self.new_parent;
            }
        }

        // Fourth pass: add to new parent's children list
        if let Some(new_parent) = self.new_parent {
            if let Some(parent) = state.scene.get_mut(&new_parent) {
                for entity_id in &self.entities {
                    if !parent.children.contains(entity_id) {
                        parent.children.push(*entity_id);
                    }
                }
            }
        }

        state.dirty = true;
        Ok(())
    }

    fn snapshots(&self, _state: &EditorState) -> Result<(StateSnapshot, StateSnapshot), CommandError> {
        if self.entities.len() != self.old_parents.len() {
            return Err(CommandError::InvalidOperation(
                "Reparent data length mismatch".to_string(),
            ));
        }

        let before: Vec<_> = self.entities.iter().copied().zip(self.old_parents.iter().copied()).collect();
        let after: Vec<_> = self.entities.iter().copied().map(|id| (id, self.new_parent)).collect();
        Ok((StateSnapshot::from_value(&before)?, StateSnapshot::from_value(&after)?))
    }

    fn to_operation(&self, id: OperationID) -> Result<Operation, CommandError> {
        let before = StateSnapshot::from_value(&self.old_parents)?;
        let after = StateSnapshot::from_value(&self.new_parent)?;
        Ok(Operation::new(
            id,
            self.description().to_string(),
            before,
            after,
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyEditSnapshot {
    pub entity: EntityId,
    pub component_type: String,
    pub field_path: String,
    pub value: Vec<u8>,
}

impl PropertyEditSnapshot {
    pub fn new(
        entity: EntityId,
        component_type: String,
        field_path: String,
        value: Vec<u8>,
    ) -> Self {
        Self {
            entity,
            component_type,
            field_path,
            value,
        }
    }
}

fn to_editor_transform(data: &TransformData) -> Transform {
    Transform {
        position: data.position,
        rotation: [data.rotation[0], data.rotation[1], data.rotation[2]],
        scale: data.scale,
    }
}

fn apply_transform_edit(entity: &mut EntityData, field: &str, value: &[u8]) -> Result<(), CommandError> {
    match field {
        "position" => {
            let pos: [f32; 3] = bincode::deserialize(value)?;
            entity.transform.position = pos;
            Ok(())
        }
        "rotation" => {
            if let Ok(rot) = bincode::deserialize::<[f32; 3]>(value) {
                entity.transform.rotation = rot;
            } else {
                let rot: [f32; 4] = bincode::deserialize(value)?;
                entity.transform.rotation = [rot[0], rot[1], rot[2]];
            }
            Ok(())
        }
        "scale" => {
            let scale: [f32; 3] = bincode::deserialize(value)?;
            entity.transform.scale = scale;
            Ok(())
        }
        "position.x" => {
            let v: f32 = bincode::deserialize(value)?;
            entity.transform.position[0] = v;
            Ok(())
        }
        "position.y" => {
            let v: f32 = bincode::deserialize(value)?;
            entity.transform.position[1] = v;
            Ok(())
        }
        "position.z" => {
            let v: f32 = bincode::deserialize(value)?;
            entity.transform.position[2] = v;
            Ok(())
        }
        "rotation.x" => {
            let v: f32 = bincode::deserialize(value)?;
            entity.transform.rotation[0] = v;
            Ok(())
        }
        "rotation.y" => {
            let v: f32 = bincode::deserialize(value)?;
            entity.transform.rotation[1] = v;
            Ok(())
        }
        "rotation.z" => {
            let v: f32 = bincode::deserialize(value)?;
            entity.transform.rotation[2] = v;
            Ok(())
        }
        "scale.x" => {
            let v: f32 = bincode::deserialize(value)?;
            entity.transform.scale[0] = v;
            Ok(())
        }
        "scale.y" => {
            let v: f32 = bincode::deserialize(value)?;
            entity.transform.scale[1] = v;
            Ok(())
        }
        "scale.z" => {
            let v: f32 = bincode::deserialize(value)?;
            entity.transform.scale[2] = v;
            Ok(())
        }
        _ => Err(CommandError::InvalidOperation(format!(
            "Unsupported transform field: {field}"
        ))),
    }
}

impl SpawnCommand {
    fn entity_data(&self) -> EntityData {
        let name = self
            .prefab_path
            .as_ref()
            .and_then(|path| std::path::Path::new(path).file_stem())
            .and_then(|name| name.to_str())
            .unwrap_or("New Entity");

        let mut data = EntityData::new(name);
        data.transform = to_editor_transform(&self.transform);
        data
    }
}

impl DuplicateCommand {
    fn build_duplicates(&self, state: &EditorState) -> Result<Vec<(EntityId, EntityData)>, CommandError> {
        if self.source_entities.len() != self.new_entities.len() {
            return Err(CommandError::InvalidOperation(
                "Duplicate data length mismatch".to_string(),
            ));
        }

        let mut duplicates = Vec::new();
        for (source_id, new_id) in self.source_entities.iter().zip(self.new_entities.iter()) {
            let Some(original) = state.scene.get(source_id).cloned() else {
                return Err(CommandError::EntityNotFound(*source_id));
            };

            let mut duplicate = original.clone();
            duplicate.name = format!("{} (Copy)", original.name);
            duplicate.children = Vec::new();
            duplicates.push((*new_id, duplicate));
        }

        Ok(duplicates)
    }
}
