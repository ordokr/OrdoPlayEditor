// SPDX-License-Identifier: MIT OR Apache-2.0
//! Entity binding for tracks.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Entity ID for binding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub Uuid);

/// Binding of a track to an entity/component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityBinding {
    /// Target entity ID
    pub entity_id: EntityId,
    /// Component type name (optional)
    pub component: Option<String>,
    /// Property path within component
    pub property_path: Option<String>,
}

impl EntityBinding {
    /// Create a binding to an entity
    pub fn entity(entity_id: EntityId) -> Self {
        Self {
            entity_id,
            component: None,
            property_path: None,
        }
    }

    /// Create a binding to a component
    pub fn component(entity_id: EntityId, component: impl Into<String>) -> Self {
        Self {
            entity_id,
            component: Some(component.into()),
            property_path: None,
        }
    }

    /// Create a binding to a property
    pub fn property(
        entity_id: EntityId,
        component: impl Into<String>,
        property_path: impl Into<String>,
    ) -> Self {
        Self {
            entity_id,
            component: Some(component.into()),
            property_path: Some(property_path.into()),
        }
    }
}
