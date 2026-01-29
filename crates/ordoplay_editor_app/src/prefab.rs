// SPDX-License-Identifier: MIT OR Apache-2.0
//! Prefab system for reusable entity templates.
//!
//! Prefabs are templates that can be instantiated multiple times in a scene.
//! They support:
//! - Property overrides (instances can differ from the source prefab)
//! - Nested prefabs (prefabs containing other prefab instances)
//! - Live updates (changes to prefab propagate to instances)


use crate::state::{EntityData, EntityId, Transform};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use uuid::Uuid;

/// Unique identifier for prefabs
pub type PrefabId = Uuid;

/// A prefab asset - a reusable entity template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prefab {
    /// Unique identifier
    pub id: PrefabId,
    /// Display name
    pub name: String,
    /// Root entity of the prefab (and its children)
    pub root: PrefabEntity,
    /// File path where this prefab is saved
    #[serde(skip)]
    pub path: Option<PathBuf>,
    /// Version for format compatibility
    pub version: u32,
}

/// An entity within a prefab
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabEntity {
    /// Local ID within the prefab (maps to instance IDs)
    pub local_id: u32,
    /// Entity name
    pub name: String,
    /// Transform relative to parent
    pub transform: Transform,
    /// Components attached to this entity
    pub components: Vec<crate::components::Component>,
    /// Child entities
    pub children: Vec<PrefabEntity>,
    /// If this is a nested prefab instance
    pub nested_prefab: Option<NestedPrefabRef>,
}

/// Reference to a nested prefab
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NestedPrefabRef {
    /// Path to the prefab asset
    pub prefab_path: PathBuf,
    /// Overrides applied to this nested instance
    pub overrides: Vec<PropertyOverride>,
}

/// A property override on a prefab instance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PropertyOverride {
    /// Path to the entity within the prefab (e.g., "0/2/1" for nested children)
    pub entity_path: String,
    /// Property path (e.g., "transform.position.x" or "components[0].intensity")
    pub property_path: String,
    /// Serialized override value
    pub value: serde_json::Value,
}

/// Instance of a prefab in a scene
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabInstance {
    /// The scene entity ID of this instance's root
    pub root_entity_id: EntityId,
    /// Path to the source prefab asset
    pub prefab_path: PathBuf,
    /// Cached prefab ID (for quick lookup)
    pub prefab_id: PrefabId,
    /// Property overrides applied to this instance
    pub overrides: Vec<PropertyOverride>,
    /// Mapping from prefab local IDs to scene entity IDs
    pub id_mapping: HashMap<u32, EntityId>,
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl Prefab {
    /// Current prefab format version
    pub const FORMAT_VERSION: u32 = 1;

    /// Create a new empty prefab
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            root: PrefabEntity {
                local_id: 0,
                name: "Root".to_string(),
                transform: Transform::default(),
                components: Vec::new(),
                children: Vec::new(),
                nested_prefab: None,
            },
            path: None,
            version: Self::FORMAT_VERSION,
        }
    }

    /// Create a prefab from existing entities
    pub fn from_entities(
        name: impl Into<String>,
        root_entity: &EntityData,
        all_entities: &HashMap<EntityId, EntityData>,
    ) -> Self {
        let mut local_id_counter = 0u32;
        let root = Self::entity_to_prefab_entity(
            root_entity,
            all_entities,
            &mut local_id_counter,
        );

        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            root,
            path: None,
            version: Self::FORMAT_VERSION,
        }
    }

    /// Convert an EntityData to a PrefabEntity recursively
    fn entity_to_prefab_entity(
        entity: &EntityData,
        all_entities: &HashMap<EntityId, EntityData>,
        local_id_counter: &mut u32,
    ) -> PrefabEntity {
        let local_id = *local_id_counter;
        *local_id_counter += 1;

        let children: Vec<PrefabEntity> = entity
            .children
            .iter()
            .filter_map(|child_id| all_entities.get(child_id))
            .map(|child| Self::entity_to_prefab_entity(child, all_entities, local_id_counter))
            .collect();

        PrefabEntity {
            local_id,
            name: entity.name.clone(),
            transform: entity.transform.clone(),
            components: entity.components.clone(),
            children,
            nested_prefab: None, // TODO: detect nested prefabs
        }
    }

    /// Instantiate this prefab, creating entity data
    pub fn instantiate(&self) -> (EntityData, HashMap<u32, EntityId>) {
        let mut id_mapping = HashMap::new();
        let root = self.instantiate_entity(&self.root, None, &mut id_mapping);
        (root, id_mapping)
    }

    /// Instantiate a prefab entity recursively
    fn instantiate_entity(
        &self,
        prefab_entity: &PrefabEntity,
        parent_id: Option<EntityId>,
        id_mapping: &mut HashMap<u32, EntityId>,
    ) -> EntityData {
        let entity_id = EntityId::new();
        id_mapping.insert(prefab_entity.local_id, entity_id);

        // Recursively instantiate children first so their IDs are in id_mapping
        let _children: Vec<EntityData> = prefab_entity
            .children
            .iter()
            .map(|child| self.instantiate_entity(child, Some(entity_id), id_mapping))
            .collect();

        // Extract child IDs from id_mapping using the children's local_ids
        let child_ids: Vec<EntityId> = prefab_entity
            .children
            .iter()
            .filter_map(|child| id_mapping.get(&child.local_id).copied())
            .collect();

        EntityData {
            name: prefab_entity.name.clone(),
            active: true,
            is_static: false,
            transform: prefab_entity.transform.clone(),
            parent: parent_id,
            children: child_ids,
            components: prefab_entity.components.clone(),
        }
    }

    /// Get all entities from instantiation as a flat list
    pub fn instantiate_flat(&self) -> (Vec<EntityData>, HashMap<u32, EntityId>) {
        let mut id_mapping = HashMap::new();
        let mut entities = Vec::new();
        self.instantiate_entity_flat(&self.root, None, &mut id_mapping, &mut entities);
        (entities, id_mapping)
    }

    fn instantiate_entity_flat(
        &self,
        prefab_entity: &PrefabEntity,
        parent_id: Option<EntityId>,
        id_mapping: &mut HashMap<u32, EntityId>,
        entities: &mut Vec<EntityData>,
    ) {
        let entity_id = EntityId::new();
        id_mapping.insert(prefab_entity.local_id, entity_id);

        // First collect child IDs
        let child_ids: Vec<EntityId> = prefab_entity
            .children
            .iter()
            .map(|_| EntityId::new())
            .collect();

        // Create entity with child IDs
        let entity = EntityData {
            name: prefab_entity.name.clone(),
            active: true,
            is_static: false,
            transform: prefab_entity.transform.clone(),
            parent: parent_id,
            children: child_ids.clone(),
            components: prefab_entity.components.clone(),
        };
        entities.push(entity);

        // Recursively create children
        for (i, child) in prefab_entity.children.iter().enumerate() {
            // Use the pre-generated child ID
            let child_entity_id = child_ids[i];
            id_mapping.insert(child.local_id, child_entity_id);
            self.instantiate_child_flat(child, entity_id, child_entity_id, id_mapping, entities);
        }
    }

    fn instantiate_child_flat(
        &self,
        prefab_entity: &PrefabEntity,
        parent_id: EntityId,
        entity_id: EntityId,
        id_mapping: &mut HashMap<u32, EntityId>,
        entities: &mut Vec<EntityData>,
    ) {
        // Collect child IDs
        let child_ids: Vec<EntityId> = prefab_entity
            .children
            .iter()
            .map(|_| EntityId::new())
            .collect();

        let entity = EntityData {
            name: prefab_entity.name.clone(),
            active: true,
            is_static: false,
            transform: prefab_entity.transform.clone(),
            parent: Some(parent_id),
            children: child_ids.clone(),
            components: prefab_entity.components.clone(),
        };
        entities.push(entity);

        // Recursively create children
        for (i, child) in prefab_entity.children.iter().enumerate() {
            let child_entity_id = child_ids[i];
            id_mapping.insert(child.local_id, child_entity_id);
            self.instantiate_child_flat(child, entity_id, child_entity_id, id_mapping, entities);
        }
    }

    /// Count total entities in this prefab
    pub fn entity_count(&self) -> usize {
        self.count_entities(&self.root)
    }

    fn count_entities(&self, entity: &PrefabEntity) -> usize {
        1 + entity.children.iter().map(|c| self.count_entities(c)).sum::<usize>()
    }

    /// Serialize to RON format
    pub fn to_ron(&self) -> Result<String, ron::Error> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
    }

    /// Deserialize from RON format
    pub fn from_ron(s: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(s)
    }

    /// Save prefab to file
    pub fn save(&self, path: &PathBuf) -> std::io::Result<()> {
        let ron_str = self.to_ron().map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })?;
        std::fs::write(path, ron_str)
    }

    /// Load prefab from file
    pub fn load(path: &PathBuf) -> std::io::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let mut prefab = Self::from_ron(&contents).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })?;
        prefab.path = Some(path.clone());
        Ok(prefab)
    }
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl PrefabInstance {
    /// Create a new prefab instance
    pub fn new(
        root_entity_id: EntityId,
        prefab_path: PathBuf,
        prefab_id: PrefabId,
        id_mapping: HashMap<u32, EntityId>,
    ) -> Self {
        Self {
            root_entity_id,
            prefab_path,
            prefab_id,
            overrides: Vec::new(),
            id_mapping,
        }
    }

    /// Check if a property is overridden
    pub fn is_overridden(&self, entity_path: &str, property_path: &str) -> bool {
        self.overrides.iter().any(|o| {
            o.entity_path == entity_path && o.property_path == property_path
        })
    }

    /// Add or update an override
    pub fn set_override(&mut self, override_: PropertyOverride) {
        // Remove existing override for same property
        self.overrides.retain(|o| {
            !(o.entity_path == override_.entity_path && o.property_path == override_.property_path)
        });
        self.overrides.push(override_);
    }

    /// Remove an override
    pub fn remove_override(&mut self, entity_path: &str, property_path: &str) {
        self.overrides.retain(|o| {
            !(o.entity_path == entity_path && o.property_path == property_path)
        });
    }

    /// Revert all overrides
    pub fn revert_all(&mut self) {
        self.overrides.clear();
    }

    /// Get all entity IDs that are part of this instance
    pub fn all_entity_ids(&self) -> HashSet<EntityId> {
        self.id_mapping.values().copied().collect()
    }

    /// Check if an entity ID belongs to this instance
    pub fn contains_entity(&self, entity_id: EntityId) -> bool {
        self.id_mapping.values().any(|&id| id == entity_id)
    }

    /// Get the local prefab ID for a scene entity ID
    pub fn get_local_id(&self, entity_id: EntityId) -> Option<u32> {
        self.id_mapping.iter()
            .find(|(_, &id)| id == entity_id)
            .map(|(&local_id, _)| local_id)
    }
}

/// Manager for prefabs in the editor
#[allow(dead_code)] // Intentionally kept for API completeness
pub struct PrefabManager {
    /// Loaded prefabs (path -> prefab)
    loaded_prefabs: HashMap<PathBuf, Prefab>,
    /// Active prefab instances in the scene
    instances: HashMap<EntityId, PrefabInstance>,
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl PrefabManager {
    /// Create a new prefab manager
    pub fn new() -> Self {
        Self {
            loaded_prefabs: HashMap::new(),
            instances: HashMap::new(),
        }
    }

    /// Load a prefab from disk
    pub fn load_prefab(&mut self, path: &PathBuf) -> std::io::Result<&Prefab> {
        if !self.loaded_prefabs.contains_key(path) {
            let prefab = Prefab::load(path)?;
            self.loaded_prefabs.insert(path.clone(), prefab);
        }
        Ok(self.loaded_prefabs.get(path).unwrap())
    }

    /// Get a loaded prefab
    pub fn get_prefab(&self, path: &PathBuf) -> Option<&Prefab> {
        self.loaded_prefabs.get(path)
    }

    /// Register a prefab instance
    pub fn register_instance(&mut self, instance: PrefabInstance) {
        self.instances.insert(instance.root_entity_id, instance);
    }

    /// Unregister a prefab instance
    pub fn unregister_instance(&mut self, root_entity_id: EntityId) {
        self.instances.remove(&root_entity_id);
    }

    /// Get prefab instance by root entity ID
    pub fn get_instance(&self, root_entity_id: EntityId) -> Option<&PrefabInstance> {
        self.instances.get(&root_entity_id)
    }

    /// Get mutable prefab instance
    pub fn get_instance_mut(&mut self, root_entity_id: EntityId) -> Option<&mut PrefabInstance> {
        self.instances.get_mut(&root_entity_id)
    }

    /// Find the prefab instance that contains an entity
    pub fn find_instance_containing(&self, entity_id: EntityId) -> Option<&PrefabInstance> {
        self.instances.values().find(|inst| inst.contains_entity(entity_id))
    }

    /// Check if an entity is part of any prefab instance
    pub fn is_prefab_entity(&self, entity_id: EntityId) -> bool {
        self.find_instance_containing(entity_id).is_some()
    }

    /// Check if an entity is the root of a prefab instance
    pub fn is_prefab_root(&self, entity_id: EntityId) -> bool {
        self.instances.contains_key(&entity_id)
    }

    /// Get all prefab instances
    pub fn all_instances(&self) -> impl Iterator<Item = &PrefabInstance> {
        self.instances.values()
    }

    /// Reload a prefab and update all instances
    pub fn reload_prefab(&mut self, path: &PathBuf) -> std::io::Result<()> {
        let prefab = Prefab::load(path)?;
        self.loaded_prefabs.insert(path.clone(), prefab);
        // Note: Updating instances would require access to EditorState
        // This would be done at a higher level
        Ok(())
    }

    /// Clear all loaded prefabs (for project unload)
    pub fn clear(&mut self) {
        self.loaded_prefabs.clear();
        self.instances.clear();
    }
}

impl Default for PrefabManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefab_creation() {
        let prefab = Prefab::new("Test Prefab");
        assert_eq!(prefab.name, "Test Prefab");
        assert_eq!(prefab.version, Prefab::FORMAT_VERSION);
        assert_eq!(prefab.entity_count(), 1);
    }

    #[test]
    fn test_prefab_instantiation() {
        let mut prefab = Prefab::new("Test");
        prefab.root.children.push(PrefabEntity {
            local_id: 1,
            name: "Child".to_string(),
            transform: Transform::default(),
            components: Vec::new(),
            children: Vec::new(),
            nested_prefab: None,
        });

        let (entities, mapping) = prefab.instantiate_flat();
        assert_eq!(entities.len(), 2);
        assert_eq!(mapping.len(), 2);
    }

    #[test]
    fn test_prefab_serialization() {
        let prefab = Prefab::new("Serialization Test");
        let ron = prefab.to_ron().unwrap();
        let loaded = Prefab::from_ron(&ron).unwrap();
        assert_eq!(loaded.name, prefab.name);
    }

    #[test]
    fn test_override_management() {
        let mut instance = PrefabInstance::new(
            EntityId::new(),
            PathBuf::from("test.prefab"),
            Uuid::new_v4(),
            HashMap::new(),
        );

        let override_ = PropertyOverride {
            entity_path: "0".to_string(),
            property_path: "transform.position.x".to_string(),
            value: serde_json::json!(5.0),
        };

        instance.set_override(override_);
        assert!(instance.is_overridden("0", "transform.position.x"));
        assert!(!instance.is_overridden("0", "transform.position.y"));

        instance.remove_override("0", "transform.position.x");
        assert!(!instance.is_overridden("0", "transform.position.x"));
    }
}
