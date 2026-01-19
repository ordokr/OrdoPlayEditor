// SPDX-License-Identifier: MIT OR Apache-2.0
//! Editor commands for undo/redo support.
//!
//! Commands encapsulate editor operations and integrate with
//! the ordoplay_editor::History system.

use crate::history::{HistoryError, Operation, OperationID, StateSnapshot};
use crate::state::EntityId;
use serde::{Deserialize, Serialize};

/// Trait for editor commands that can be undone/redone
pub trait EditorCommand: Send + Sync {
    /// Get a description of this command
    fn description(&self) -> &str;

    /// Execute the command
    fn execute(&self) -> Result<(), CommandError>;

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

    fn execute(&self) -> Result<(), CommandError> {
        // TODO: Apply transforms to entities
        Ok(())
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

    fn execute(&self) -> Result<(), CommandError> {
        // TODO: Spawn entity in scene
        Ok(())
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

    fn execute(&self) -> Result<(), CommandError> {
        // TODO: Delete entities from scene
        Ok(())
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

    fn execute(&self) -> Result<(), CommandError> {
        // TODO: Duplicate entities in scene
        Ok(())
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

    fn execute(&self) -> Result<(), CommandError> {
        // TODO: Apply property change
        Ok(())
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

    fn execute(&self) -> Result<(), CommandError> {
        // TODO: Reparent entities in hierarchy
        Ok(())
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
