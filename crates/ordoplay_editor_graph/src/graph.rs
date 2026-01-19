// SPDX-License-Identifier: MIT OR Apache-2.0
//! Graph data structure containing nodes and connections.

use crate::connection::{Connection, ConnectionId};
use crate::node::{Node, NodeId};
use crate::port::PortId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A node graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph {
    /// Graph name
    pub name: String,
    /// Nodes in the graph
    nodes: IndexMap<NodeId, Node>,
    /// Connections between nodes
    connections: IndexMap<ConnectionId, Connection>,
}

impl Graph {
    /// Create a new empty graph
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nodes: IndexMap::new(),
            connections: IndexMap::new(),
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: Node) -> NodeId {
        let id = node.id;
        self.nodes.insert(id, node);
        id
    }

    /// Remove a node and its connections
    pub fn remove_node(&mut self, node_id: NodeId) -> Option<Node> {
        // Remove connections involving this node
        self.connections.retain(|_, c| !c.involves_node(node_id));
        // Remove the node
        self.nodes.swap_remove(&node_id)
    }

    /// Get a node by ID
    pub fn node(&self, node_id: NodeId) -> Option<&Node> {
        self.nodes.get(&node_id)
    }

    /// Get a mutable node by ID
    pub fn node_mut(&mut self, node_id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(&node_id)
    }

    /// Get all nodes
    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values()
    }

    /// Get all node IDs
    pub fn node_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.keys().copied()
    }

    /// Get the number of nodes
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Add a connection between ports
    pub fn connect(
        &mut self,
        from_node: NodeId,
        from_port: PortId,
        to_node: NodeId,
        to_port: PortId,
    ) -> Result<ConnectionId, ConnectionError> {
        // Validate nodes exist
        let source_node = self.nodes.get(&from_node)
            .ok_or(ConnectionError::NodeNotFound(from_node))?;
        let target_node = self.nodes.get(&to_node)
            .ok_or(ConnectionError::NodeNotFound(to_node))?;

        // Validate ports exist
        let source_port = source_node.port(&from_port)
            .ok_or(ConnectionError::PortNotFound(from_port))?;
        let target_port = target_node.port(&to_port)
            .ok_or(ConnectionError::PortNotFound(to_port))?;

        // Validate connection is valid
        if !source_port.can_connect(target_port) {
            return Err(ConnectionError::IncompatiblePorts);
        }

        // Check for existing connection to this input (if not multi-connect)
        if !target_port.multi_connect {
            if self.connections.values().any(|c| c.to_port == to_port) {
                return Err(ConnectionError::PortAlreadyConnected(to_port));
            }
        }

        // Prevent self-loops
        if from_node == to_node {
            return Err(ConnectionError::SelfLoop);
        }

        // TODO: Check for cycles (if required by graph type)

        let connection = Connection::new(from_node, from_port, to_node, to_port);
        let id = connection.id;
        self.connections.insert(id, connection);
        Ok(id)
    }

    /// Remove a connection
    pub fn disconnect(&mut self, connection_id: ConnectionId) -> Option<Connection> {
        self.connections.swap_remove(&connection_id)
    }

    /// Get a connection by ID
    pub fn connection(&self, connection_id: ConnectionId) -> Option<&Connection> {
        self.connections.get(&connection_id)
    }

    /// Get all connections
    pub fn connections(&self) -> impl Iterator<Item = &Connection> {
        self.connections.values()
    }

    /// Get connections from a specific port
    pub fn connections_from(&self, port_id: PortId) -> impl Iterator<Item = &Connection> {
        self.connections.values().filter(move |c| c.from_port == port_id)
    }

    /// Get connections to a specific port
    pub fn connections_to(&self, port_id: PortId) -> impl Iterator<Item = &Connection> {
        self.connections.values().filter(move |c| c.to_port == port_id)
    }

    /// Get connections involving a node
    pub fn connections_for_node(&self, node_id: NodeId) -> impl Iterator<Item = &Connection> {
        self.connections.values().filter(move |c| c.involves_node(node_id))
    }

    /// Get the number of connections
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Get nodes in topological order (for evaluation)
    pub fn topological_order(&self) -> Result<Vec<NodeId>, CycleError> {
        let mut visited = std::collections::HashSet::new();
        let mut temp_mark = std::collections::HashSet::new();
        let mut order = Vec::new();

        for node_id in self.nodes.keys() {
            if !visited.contains(node_id) {
                self.visit(*node_id, &mut visited, &mut temp_mark, &mut order)?;
            }
        }

        order.reverse();
        Ok(order)
    }

    fn visit(
        &self,
        node_id: NodeId,
        visited: &mut std::collections::HashSet<NodeId>,
        temp_mark: &mut std::collections::HashSet<NodeId>,
        order: &mut Vec<NodeId>,
    ) -> Result<(), CycleError> {
        if temp_mark.contains(&node_id) {
            return Err(CycleError);
        }
        if visited.contains(&node_id) {
            return Ok(());
        }

        temp_mark.insert(node_id);

        // Visit all nodes that this node depends on
        for connection in self.connections_for_node(node_id) {
            if connection.to_node == node_id {
                self.visit(connection.from_node, visited, temp_mark, order)?;
            }
        }

        temp_mark.remove(&node_id);
        visited.insert(node_id);
        order.push(node_id);

        Ok(())
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new("Untitled")
    }
}

/// Error when creating a connection
#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    /// Node not found
    #[error("Node not found: {0:?}")]
    NodeNotFound(NodeId),

    /// Port not found
    #[error("Port not found: {0:?}")]
    PortNotFound(PortId),

    /// Incompatible port types
    #[error("Incompatible port types")]
    IncompatiblePorts,

    /// Port is already connected
    #[error("Port already connected: {0:?}")]
    PortAlreadyConnected(PortId),

    /// Self-loop not allowed
    #[error("Self-loop not allowed")]
    SelfLoop,
}

/// Error when graph contains a cycle
#[derive(Debug, thiserror::Error)]
#[error("Graph contains a cycle")]
pub struct CycleError;
