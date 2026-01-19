// SPDX-License-Identifier: MIT OR Apache-2.0
//! Node definitions for the graph framework.

use crate::port::Port;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
    /// Create a new random node ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

/// Node type category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeCategory {
    /// Input nodes (constants, parameters)
    Input,
    /// Output nodes (result, preview)
    Output,
    /// Math operations
    Math,
    /// Texture operations
    Texture,
    /// Logic/flow control
    Logic,
    /// Utility nodes
    Utility,
    /// Custom/user-defined
    Custom,
}

/// Node type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeType {
    /// Unique type identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Category
    pub category: NodeCategory,
    /// Description
    pub description: String,
    /// Default input ports
    pub inputs: Vec<Port>,
    /// Default output ports
    pub outputs: Vec<Port>,
}

/// A node instance in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Unique instance ID
    pub id: NodeId,
    /// Node type ID
    pub node_type: String,
    /// Display name (can be customized)
    pub name: String,
    /// Position in the graph UI
    pub position: [f32; 2],
    /// Input ports
    pub inputs: Vec<Port>,
    /// Output ports
    pub outputs: Vec<Port>,
    /// Whether the node is collapsed in the UI
    pub collapsed: bool,
    /// Custom color (optional)
    pub color: Option<[u8; 3]>,
}

impl Node {
    /// Create a new node from a type definition
    pub fn new(node_type: &NodeType) -> Self {
        Self {
            id: NodeId::new(),
            node_type: node_type.id.clone(),
            name: node_type.name.clone(),
            position: [0.0, 0.0],
            inputs: node_type.inputs.clone(),
            outputs: node_type.outputs.clone(),
            collapsed: false,
            color: None,
        }
    }

    /// Set the position
    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.position = [x, y];
        self
    }

    /// Get an input port by index
    pub fn input(&self, index: usize) -> Option<&Port> {
        self.inputs.get(index)
    }

    /// Get an output port by index
    pub fn output(&self, index: usize) -> Option<&Port> {
        self.outputs.get(index)
    }

    /// Get a port by ID
    pub fn port(&self, port_id: &crate::port::PortId) -> Option<&Port> {
        self.inputs.iter().find(|p| p.id == *port_id)
            .or_else(|| self.outputs.iter().find(|p| p.id == *port_id))
    }

    /// Get all ports
    pub fn ports(&self) -> impl Iterator<Item = &Port> {
        self.inputs.iter().chain(self.outputs.iter())
    }
}

/// Registry of available node types
pub struct NodeRegistry {
    /// Registered node types by ID
    types: indexmap::IndexMap<String, NodeType>,
}

impl NodeRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            types: indexmap::IndexMap::new(),
        }
    }

    /// Register a node type
    pub fn register(&mut self, node_type: NodeType) {
        self.types.insert(node_type.id.clone(), node_type);
    }

    /// Get a node type by ID
    pub fn get(&self, id: &str) -> Option<&NodeType> {
        self.types.get(id)
    }

    /// Get all registered types
    pub fn types(&self) -> impl Iterator<Item = &NodeType> {
        self.types.values()
    }

    /// Get types by category
    pub fn types_in_category(&self, category: NodeCategory) -> impl Iterator<Item = &NodeType> {
        self.types.values().filter(move |t| t.category == category)
    }

    /// Create a node from a type ID
    pub fn create_node(&self, type_id: &str) -> Option<Node> {
        self.get(type_id).map(Node::new)
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
