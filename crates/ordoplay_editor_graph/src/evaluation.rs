// SPDX-License-Identifier: MIT OR Apache-2.0
//! Graph evaluation and execution.

use crate::graph::Graph;
use crate::node::NodeId;
use crate::port::{PortId, PortValue};
use std::collections::HashMap;

/// Result of evaluating a node
#[derive(Debug, Clone)]
pub struct NodeOutput {
    /// Output values by port ID
    pub values: HashMap<PortId, PortValue>,
}

impl NodeOutput {
    /// Create a new empty output
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    /// Set an output value
    pub fn set(&mut self, port_id: PortId, value: PortValue) {
        self.values.insert(port_id, value);
    }

    /// Get an output value
    pub fn get(&self, port_id: &PortId) -> Option<&PortValue> {
        self.values.get(port_id)
    }
}

impl Default for NodeOutput {
    fn default() -> Self {
        Self::new()
    }
}

/// Context for graph evaluation
pub struct EvaluationContext<'a> {
    /// The graph being evaluated
    pub graph: &'a Graph,
    /// Cached node outputs
    outputs: HashMap<NodeId, NodeOutput>,
    /// Evaluation order
    order: Vec<NodeId>,
}

impl<'a> EvaluationContext<'a> {
    /// Create a new evaluation context
    pub fn new(graph: &'a Graph) -> Result<Self, EvaluationError> {
        let order = graph.topological_order()
            .map_err(|_| EvaluationError::CycleDetected)?;

        Ok(Self {
            graph,
            outputs: HashMap::new(),
            order,
        })
    }

    /// Get the input value for a port
    pub fn get_input(&self, node_id: NodeId, port_id: PortId) -> Option<&PortValue> {
        // Find the connection to this port
        let connection = self.graph.connections_to(port_id).next()?;

        // Get the output from the source node
        let source_output = self.outputs.get(&connection.from_node)?;
        source_output.get(&connection.from_port)
    }

    /// Get the default value for an input port
    pub fn get_default(&self, node_id: NodeId, port_id: PortId) -> Option<&PortValue> {
        let node = self.graph.node(node_id)?;
        let port = node.port(&port_id)?;
        port.default_value.as_ref()
    }

    /// Get input value or default
    pub fn get_input_or_default(&self, node_id: NodeId, port_id: PortId) -> Option<&PortValue> {
        self.get_input(node_id, port_id)
            .or_else(|| self.get_default(node_id, port_id))
    }

    /// Set the output for a node
    pub fn set_output(&mut self, node_id: NodeId, output: NodeOutput) {
        self.outputs.insert(node_id, output);
    }

    /// Get the evaluation order
    pub fn order(&self) -> &[NodeId] {
        &self.order
    }

    /// Get all outputs
    pub fn outputs(&self) -> &HashMap<NodeId, NodeOutput> {
        &self.outputs
    }
}

/// Trait for evaluating nodes
pub trait NodeEvaluator {
    /// Evaluate a node and produce outputs
    fn evaluate(&self, node_id: NodeId, ctx: &mut EvaluationContext) -> Result<NodeOutput, EvaluationError>;
}

/// Error during evaluation
#[derive(Debug, thiserror::Error)]
pub enum EvaluationError {
    /// Graph contains a cycle
    #[error("Graph contains a cycle")]
    CycleDetected,

    /// Node not found
    #[error("Node not found: {0:?}")]
    NodeNotFound(NodeId),

    /// Missing required input
    #[error("Missing required input: {0:?}")]
    MissingInput(PortId),

    /// Type mismatch
    #[error("Type mismatch")]
    TypeMismatch,

    /// Custom error
    #[error("{0}")]
    Custom(String),
}
