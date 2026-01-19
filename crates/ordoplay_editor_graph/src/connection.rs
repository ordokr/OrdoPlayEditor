// SPDX-License-Identifier: MIT OR Apache-2.0
//! Connection (edge) definitions for the graph.

use crate::node::NodeId;
use crate::port::PortId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a connection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(pub Uuid);

impl ConnectionId {
    /// Create a new random connection ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new()
    }
}

/// A connection between two ports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    /// Unique connection ID
    pub id: ConnectionId,
    /// Source node ID
    pub from_node: NodeId,
    /// Source port ID
    pub from_port: PortId,
    /// Target node ID
    pub to_node: NodeId,
    /// Target port ID
    pub to_port: PortId,
}

impl Connection {
    /// Create a new connection
    pub fn new(
        from_node: NodeId,
        from_port: PortId,
        to_node: NodeId,
        to_port: PortId,
    ) -> Self {
        Self {
            id: ConnectionId::new(),
            from_node,
            from_port,
            to_node,
            to_port,
        }
    }

    /// Check if this connection involves a specific node
    pub fn involves_node(&self, node_id: NodeId) -> bool {
        self.from_node == node_id || self.to_node == node_id
    }

    /// Check if this connection involves a specific port
    pub fn involves_port(&self, port_id: PortId) -> bool {
        self.from_port == port_id || self.to_port == port_id
    }
}
