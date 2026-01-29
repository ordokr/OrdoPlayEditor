// SPDX-License-Identifier: MIT OR Apache-2.0
//! Editor commands for undo/redo support.
//!
//! Commands encapsulate editor operations and integrate with
//! the `ordoplay_editor::History` system.


use crate::history::{HistoryError, Operation, OperationID, StateSnapshot};
use crate::state::{EditorState, EntityData, EntityId, Transform};
use serde::{Deserialize, Serialize};

/// Trait for editor commands that can be undone/redone
#[allow(dead_code)] // Intentionally kept for API completeness
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

impl From<&Transform> for TransformData {
    fn from(transform: &Transform) -> Self {
        Self {
            position: transform.position,
            rotation: euler_to_quaternion(transform.rotation),
            scale: transform.scale,
        }
    }
}

/// Convert euler angles (degrees) to quaternion [x, y, z, w]
fn euler_to_quaternion(euler_deg: [f32; 3]) -> [f32; 4] {
    let half_x = (euler_deg[0] * std::f32::consts::PI / 180.0) * 0.5;
    let half_y = (euler_deg[1] * std::f32::consts::PI / 180.0) * 0.5;
    let half_z = (euler_deg[2] * std::f32::consts::PI / 180.0) * 0.5;

    let (sx, cx) = half_x.sin_cos();
    let (sy, cy) = half_y.sin_cos();
    let (sz, cz) = half_z.sin_cos();

    [
        sx * cy * cz - cx * sy * sz, // x
        cx * sy * cz + sx * cy * sz, // y
        cx * cy * sz - sx * sy * cz, // z
        cx * cy * cz + sx * sy * sz, // w
    ]
}

/// Convert quaternion [x, y, z, w] to euler angles (degrees)
fn quaternion_to_euler(q: [f32; 4]) -> [f32; 3] {
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);

    // Roll (X)
    let sinr_cosp = 2.0 * (w * x + y * z);
    let cosr_cosp = 1.0 - 2.0 * (x * x + y * y);
    let roll = sinr_cosp.atan2(cosr_cosp);

    // Pitch (Y)
    let sinp = 2.0 * (w * y - z * x);
    let pitch = if sinp.abs() >= 1.0 {
        std::f32::consts::FRAC_PI_2.copysign(sinp)
    } else {
        sinp.asin()
    };

    // Yaw (Z)
    let siny_cosp = 2.0 * (w * z + x * y);
    let cosy_cosp = 1.0 - 2.0 * (y * y + z * z);
    let yaw = siny_cosp.atan2(cosy_cosp);

    [
        roll * 180.0 / std::f32::consts::PI,
        pitch * 180.0 / std::f32::consts::PI,
        yaw * 180.0 / std::f32::consts::PI,
    ]
}

impl From<Transform> for TransformData {
    fn from(transform: Transform) -> Self {
        Self::from(&transform)
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
    /// Optional override name for the new entity
    pub name: Option<String>,
    /// Initial transform
    pub transform: TransformData,
    /// Optional parent for the new entity
    pub parent: Option<EntityId>,
    /// Whether to select the spawned entity
    pub select: bool,
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl SpawnCommand {
    /// Create a new spawn command
    pub fn new(entity_id: EntityId, transform: TransformData) -> Self {
        Self {
            entity_id,
            prefab_path: None,
            name: None,
            transform,
            parent: None,
            select: true,
        }
    }

    /// Set the prefab path
    pub fn with_prefab(mut self, path: impl Into<String>) -> Self {
        self.prefab_path = Some(path.into());
        self
    }

    /// Override the entity name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Attach the entity to a parent
    pub fn with_parent(mut self, parent: EntityId) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Set whether the new entity should be selected
    pub fn with_select(mut self, select: bool) -> Self {
        self.select = select;
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

        if let Some(parent_id) = self.parent {
            if !state.scene.entities.contains_key(&parent_id) {
                return Err(CommandError::EntityNotFound(parent_id));
            }
        }

        let data = self.entity_data();

        if !state.scene.insert_entity(self.entity_id, data) {
            return Err(CommandError::InvalidOperation(
                "Failed to insert entity".to_string(),
            ));
        }

        if let Some(parent_id) = self.parent {
            if let Some(parent) = state.scene.get_mut(&parent_id) {
                if !parent.children.contains(&self.entity_id) {
                    parent.children.push(self.entity_id);
                }
            }
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

    /// Recursively duplicate an entity and all its children, returning all (`new_id`, `entity_data`) pairs.
    /// `id_map` accumulates `old_id` -> `new_id` mappings.
    fn duplicate_recursive(
        source_id: EntityId,
        new_id: EntityId,
        state: &EditorState,
        id_map: &mut std::collections::HashMap<EntityId, EntityId>,
    ) -> Result<Vec<(EntityId, EntityData)>, CommandError> {
        let original = state.scene.get(&source_id)
            .ok_or(CommandError::EntityNotFound(source_id))?
            .clone();

        id_map.insert(source_id, new_id);

        // Recursively duplicate children first so id_map is populated
        let mut all_duplicates = Vec::new();
        let mut new_child_ids = Vec::new();
        for &child_id in &original.children {
            let new_child_id = EntityId::new();
            let child_dupes = Self::duplicate_recursive(child_id, new_child_id, state, id_map)?;
            new_child_ids.push(new_child_id);
            all_duplicates.extend(child_dupes);
        }

        let mut duplicate = original.clone();
        duplicate.name = format!("{} (Copy)", original.name);
        duplicate.children = new_child_ids;
        // Parent will be set by caller or kept as original's parent for top-level duplicates

        // Insert self at the front
        all_duplicates.insert(0, (new_id, duplicate));
        Ok(all_duplicates)
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

        let mut id_map = std::collections::HashMap::new();

        for (source_id, new_id) in self.source_entities.iter().zip(self.new_entities.iter()) {
            let all_duplicates = Self::duplicate_recursive(*source_id, *new_id, state, &mut id_map)?;

            for (dup_id, mut dup_data) in all_duplicates {
                // For top-level duplicates, keep original parent; for children, remap parent
                if dup_id == *new_id {
                    // Top-level: keep original parent, add to parent's children
                    if let Some(parent_id) = dup_data.parent {
                        if let Some(parent) = state.scene.get_mut(&parent_id) {
                            if !parent.children.contains(&dup_id) {
                                parent.children.push(dup_id);
                            }
                        }
                    }
                } else {
                    // Child: parent was already remapped via id_map in children list
                    // Update parent reference to the new parent ID
                    if let Some(old_parent) = dup_data.parent {
                        dup_data.parent = id_map.get(&old_parent).copied().or(Some(old_parent));
                    }
                }

                if !state.scene.insert_entity(dup_id, dup_data) {
                    return Err(CommandError::InvalidOperation(
                        "Failed to insert duplicate entity".to_string(),
                    ));
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

        if component.eq_ignore_ascii_case("Entity") && field.eq_ignore_ascii_case("active") {
            let active: bool = bincode::deserialize(&self.new_value)?;
            entity.active = active;
            state.dirty = true;
            return Ok(());
        }

        if component.eq_ignore_ascii_case("Entity") && field.eq_ignore_ascii_case("is_static") {
            let is_static: bool = bincode::deserialize(&self.new_value)?;
            entity.is_static = is_static;
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

/// Command to edit multiple properties as a single undoable operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyEditGroupCommand {
    pub description: String,
    pub edits: Vec<PropertyEditCommand>,
}

impl PropertyEditGroupCommand {
    pub fn new(description: impl Into<String>, edits: Vec<PropertyEditCommand>) -> Self {
        Self {
            description: description.into(),
            edits,
        }
    }
}

impl EditorCommand for PropertyEditGroupCommand {
    fn description(&self) -> &str {
        &self.description
    }

    fn execute(&self, state: &mut EditorState) -> Result<(), CommandError> {
        if self.edits.is_empty() {
            return Ok(());
        }

        for edit in &self.edits {
            edit.execute(state)?;
        }

        Ok(())
    }

    fn snapshots(&self, _state: &EditorState) -> Result<(StateSnapshot, StateSnapshot), CommandError> {
        if self.edits.is_empty() {
            return Err(CommandError::InvalidOperation(
                "No property edits provided".to_string(),
            ));
        }

        let mut before = Vec::new();
        let mut after = Vec::new();

        for edit in &self.edits {
            before.push(PropertyEditSnapshot::new(
                edit.entity,
                edit.component_type.clone(),
                edit.field_path.clone(),
                edit.old_value.clone(),
            ));
            after.push(PropertyEditSnapshot::new(
                edit.entity,
                edit.component_type.clone(),
                edit.field_path.clone(),
                edit.new_value.clone(),
            ));
        }

        Ok((
            StateSnapshot::from_value(&before)?,
            StateSnapshot::from_value(&after)?,
        ))
    }

    fn to_operation(&self, id: OperationID) -> Result<Operation, CommandError> {
        let mut before = Vec::new();
        let mut after = Vec::new();

        for edit in &self.edits {
            before.push(PropertyEditSnapshot::new(
                edit.entity,
                edit.component_type.clone(),
                edit.field_path.clone(),
                edit.old_value.clone(),
            ));
            after.push(PropertyEditSnapshot::new(
                edit.entity,
                edit.component_type.clone(),
                edit.field_path.clone(),
                edit.new_value.clone(),
            ));
        }

        Ok(Operation::new(
            id,
            self.description.clone(),
            StateSnapshot::from_value(&before)?,
            StateSnapshot::from_value(&after)?,
        ))
    }
}

fn to_editor_transform(data: &TransformData) -> Transform {
    Transform {
        position: data.position,
        rotation: quaternion_to_euler(data.rotation),
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
            .name
            .clone()
            .or_else(|| {
                self.prefab_path
                    .as_ref()
                    .and_then(|path| std::path::Path::new(path).file_stem())
                    .and_then(|name| name.to_str())
                    .map(ToString::to_string)
            })
            .unwrap_or_else(|| "New Entity".to_string());

        let mut data = EntityData::new(name);
        data.transform = to_editor_transform(&self.transform);
        data.parent = self.parent;
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

        let mut all_duplicates = Vec::new();
        let mut id_map = std::collections::HashMap::new();
        for (source_id, new_id) in self.source_entities.iter().zip(self.new_entities.iter()) {
            let dupes = Self::duplicate_recursive(*source_id, *new_id, state, &mut id_map)?;
            all_duplicates.extend(dupes);
        }

        Ok(all_duplicates)
    }
}

// ============================================================================
// Component Commands
// ============================================================================

use crate::components::Component;

/// Command to add a component to an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddComponentCommand {
    /// Target entity
    pub entity_id: EntityId,
    /// Component to add
    pub component: Component,
}

impl AddComponentCommand {
    /// Create a new add component command
    pub fn new(entity_id: EntityId, component: Component) -> Self {
        Self {
            entity_id,
            component,
        }
    }
}

impl EditorCommand for AddComponentCommand {
    fn description(&self) -> &str {
        "Add Component"
    }

    fn execute(&self, state: &mut EditorState) -> Result<(), CommandError> {
        let Some(entity) = state.scene.get_mut(&self.entity_id) else {
            return Err(CommandError::EntityNotFound(self.entity_id));
        };

        // Check if component of same type already exists
        let type_id = self.component.type_id();
        if entity.components.iter().any(|c| c.type_id() == type_id) {
            return Err(CommandError::InvalidOperation(format!(
                "Entity already has a {} component",
                self.component.display_name()
            )));
        }

        entity.components.push(self.component.clone());
        state.dirty = true;
        Ok(())
    }

    fn snapshots(&self, _state: &EditorState) -> Result<(StateSnapshot, StateSnapshot), CommandError> {
        // Before: no component, After: has component
        let before: Option<Component> = None;
        let after: Option<Component> = Some(self.component.clone());
        Ok((
            StateSnapshot::from_value(&(self.entity_id, before))?,
            StateSnapshot::from_value(&(self.entity_id, after))?,
        ))
    }

    fn to_operation(&self, id: OperationID) -> Result<Operation, CommandError> {
        let before: Option<Component> = None;
        let after: Option<Component> = Some(self.component.clone());
        Ok(Operation::new(
            id,
            format!("Add {}", self.component.display_name()),
            StateSnapshot::from_value(&before)?,
            StateSnapshot::from_value(&after)?,
        ))
    }
}

/// Command to remove a component from an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveComponentCommand {
    /// Target entity
    pub entity_id: EntityId,
    /// Index of component to remove
    pub component_index: usize,
    /// The component being removed (for undo)
    pub removed_component: Component,
}

impl RemoveComponentCommand {
    /// Create a new remove component command
    pub fn new(entity_id: EntityId, component_index: usize, removed_component: Component) -> Self {
        Self {
            entity_id,
            component_index,
            removed_component,
        }
    }
}

impl EditorCommand for RemoveComponentCommand {
    fn description(&self) -> &str {
        "Remove Component"
    }

    fn execute(&self, state: &mut EditorState) -> Result<(), CommandError> {
        let Some(entity) = state.scene.get_mut(&self.entity_id) else {
            return Err(CommandError::EntityNotFound(self.entity_id));
        };

        if self.component_index >= entity.components.len() {
            return Err(CommandError::InvalidOperation(format!(
                "Component index {} out of bounds",
                self.component_index
            )));
        }

        entity.components.remove(self.component_index);
        state.dirty = true;
        Ok(())
    }

    fn snapshots(&self, _state: &EditorState) -> Result<(StateSnapshot, StateSnapshot), CommandError> {
        // Before: has component, After: no component
        let before: Option<Component> = Some(self.removed_component.clone());
        let after: Option<Component> = None;
        Ok((
            StateSnapshot::from_value(&(self.entity_id, self.component_index, before))?,
            StateSnapshot::from_value(&(self.entity_id, self.component_index, after))?,
        ))
    }

    fn to_operation(&self, id: OperationID) -> Result<Operation, CommandError> {
        let before: Option<Component> = Some(self.removed_component.clone());
        let after: Option<Component> = None;
        Ok(Operation::new(
            id,
            format!("Remove {}", self.removed_component.display_name()),
            StateSnapshot::from_value(&before)?,
            StateSnapshot::from_value(&after)?,
        ))
    }
}

// ============================================================================
// Prefab Commands
// ============================================================================

use crate::prefab::Prefab;
use std::path::PathBuf;

/// Command to instantiate a prefab
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstantiatePrefabCommand {
    /// Path to the prefab asset
    pub prefab_path: PathBuf,
    /// Pre-generated entity IDs for the instantiated entities
    pub entity_ids: Vec<EntityId>,
    /// Parent entity (if any)
    pub parent: Option<EntityId>,
    /// Prefab data (serialized for undo)
    pub prefab_data: Vec<u8>,
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl InstantiatePrefabCommand {
    /// Create a new instantiate prefab command
    pub fn new(prefab: &Prefab, parent: Option<EntityId>) -> Self {
        let (entities, _mapping) = prefab.instantiate_flat();
        let entity_ids: Vec<EntityId> = entities.iter().map(|_| EntityId::new()).collect();
        let prefab_data = bincode::serialize(prefab).unwrap_or_default();

        Self {
            prefab_path: prefab.path.clone().unwrap_or_default(),
            entity_ids,
            parent,
            prefab_data,
        }
    }
}

impl EditorCommand for InstantiatePrefabCommand {
    fn description(&self) -> &str {
        "Instantiate Prefab"
    }

    fn execute(&self, state: &mut EditorState) -> Result<(), CommandError> {
        let prefab: Prefab = bincode::deserialize(&self.prefab_data)
            .map_err(|e| CommandError::InvalidOperation(format!("Failed to deserialize prefab: {}", e)))?;

        let (mut entities, id_mapping) = prefab.instantiate_flat();

        // Assign our pre-generated IDs and update parent references
        if entities.len() != self.entity_ids.len() {
            return Err(CommandError::InvalidOperation(
                "Entity count mismatch".to_string(),
            ));
        }

        // Build a mapping from old generated IDs to our pre-generated IDs
        let mut old_to_new: std::collections::HashMap<EntityId, EntityId> = std::collections::HashMap::new();
        for (i, old_id) in id_mapping.values().enumerate() {
            if i < self.entity_ids.len() {
                old_to_new.insert(*old_id, self.entity_ids[i]);
            }
        }

        // Insert entities with our IDs
        for (i, mut entity) in entities.drain(..).enumerate() {
            let new_id = self.entity_ids[i];

            // Update parent reference
            if i == 0 {
                // Root entity gets our specified parent
                entity.parent = self.parent;
            } else if let Some(old_parent) = entity.parent {
                // Update to use new ID
                entity.parent = old_to_new.get(&old_parent).copied();
            }

            // Update children references
            entity.children = entity.children.iter()
                .filter_map(|old_id| old_to_new.get(old_id).copied())
                .collect();

            state.scene.insert_entity(new_id, entity);
        }

        // Add root to parent's children
        if let Some(parent_id) = self.parent {
            if let Some(parent) = state.scene.get_mut(&parent_id) {
                if !self.entity_ids.is_empty() && !parent.children.contains(&self.entity_ids[0]) {
                    parent.children.push(self.entity_ids[0]);
                }
            }
        }

        // Select the root entity
        if !self.entity_ids.is_empty() {
            state.selection.clear();
            state.selection.add(self.entity_ids[0]);
        }

        state.dirty = true;
        Ok(())
    }

    fn snapshots(&self, _state: &EditorState) -> Result<(StateSnapshot, StateSnapshot), CommandError> {
        let before: Vec<EntityId> = Vec::new();
        let after = self.entity_ids.clone();
        Ok((
            StateSnapshot::from_value(&before)?,
            StateSnapshot::from_value(&after)?,
        ))
    }

    fn to_operation(&self, id: OperationID) -> Result<Operation, CommandError> {
        Ok(Operation::new(
            id,
            self.description().to_string(),
            StateSnapshot::from_value(&Vec::<EntityId>::new())?,
            StateSnapshot::from_value(&self.entity_ids)?,
        ))
    }
}

/// Command to create a prefab from selected entities
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePrefabCommand {
    /// Name for the new prefab
    pub name: String,
    /// Path to save the prefab
    pub path: PathBuf,
    /// Source entity IDs
    pub source_entities: Vec<EntityId>,
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl CreatePrefabCommand {
    /// Create a new create prefab command
    pub fn new(name: impl Into<String>, path: impl Into<PathBuf>, source_entities: Vec<EntityId>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            source_entities,
        }
    }

    /// Build the prefab from the current state
    pub fn build_prefab(&self, state: &EditorState) -> Result<Prefab, CommandError> {
        if self.source_entities.is_empty() {
            return Err(CommandError::InvalidOperation(
                "No entities selected for prefab".to_string(),
            ));
        }

        // Get the first entity as root
        let root_id = self.source_entities[0];
        let Some(root_entity) = state.scene.get(&root_id) else {
            return Err(CommandError::EntityNotFound(root_id));
        };

        // Build entity map
        let mut entities_map = std::collections::HashMap::new();
        for entity_id in &self.source_entities {
            if let Some(entity) = state.scene.get(entity_id) {
                entities_map.insert(*entity_id, entity.clone());
            }
        }

        let prefab = Prefab::from_entities(&self.name, root_entity, &entities_map);
        Ok(prefab)
    }
}

impl EditorCommand for CreatePrefabCommand {
    fn description(&self) -> &str {
        "Create Prefab"
    }

    fn execute(&self, state: &mut EditorState) -> Result<(), CommandError> {
        let prefab = self.build_prefab(state)?;

        // Save prefab to disk
        prefab.save(&self.path).map_err(|e| {
            CommandError::InvalidOperation(format!("Failed to save prefab: {}", e))
        })?;

        tracing::info!("Created prefab '{}' at {:?}", self.name, self.path);
        Ok(())
    }

    fn snapshots(&self, _state: &EditorState) -> Result<(StateSnapshot, StateSnapshot), CommandError> {
        // Creating a prefab doesn't modify scene state, just creates a file
        Ok((
            StateSnapshot::new(vec![]),
            StateSnapshot::from_value(&self.path)?,
        ))
    }

    fn to_operation(&self, id: OperationID) -> Result<Operation, CommandError> {
        Ok(Operation::new(
            id,
            format!("Create Prefab '{}'", self.name),
            StateSnapshot::new(vec![]),
            StateSnapshot::from_value(&self.path)?,
        ))
    }
}
