// SPDX-License-Identifier: MIT OR Apache-2.0
//! Gameplay graph for visual scripting (Blueprint-like).
//!
//! Supports execution flow and data flow.

use crate::node::{NodeCategory, NodeRegistry, NodeType};
use crate::port::{Port, PortDirection, PortId, PortType};

/// Create the gameplay graph node registry
pub fn create_gameplay_registry() -> NodeRegistry {
    let mut registry = NodeRegistry::new();

    // Event nodes
    registry.register(NodeType {
        id: "event_begin_play".to_string(),
        name: "Event Begin Play".to_string(),
        category: NodeCategory::Input,
        description: "Triggered when gameplay starts".to_string(),
        inputs: vec![],
        outputs: vec![
            Port::new(PortId::new(), "Exec", PortType::Exec, PortDirection::Output),
        ],
    });

    registry.register(NodeType {
        id: "event_tick".to_string(),
        name: "Event Tick".to_string(),
        category: NodeCategory::Input,
        description: "Triggered every frame".to_string(),
        inputs: vec![],
        outputs: vec![
            Port::new(PortId::new(), "Exec", PortType::Exec, PortDirection::Output),
            Port::new(PortId::new(), "Delta Time", PortType::Float, PortDirection::Output),
        ],
    });

    // Flow control
    registry.register(NodeType {
        id: "branch".to_string(),
        name: "Branch".to_string(),
        category: NodeCategory::Logic,
        description: "If/else branching".to_string(),
        inputs: vec![
            Port::new(PortId::new(), "Exec", PortType::Exec, PortDirection::Input),
            Port::new(PortId::new(), "Condition", PortType::Bool, PortDirection::Input),
        ],
        outputs: vec![
            Port::new(PortId::new(), "True", PortType::Exec, PortDirection::Output),
            Port::new(PortId::new(), "False", PortType::Exec, PortDirection::Output),
        ],
    });

    // Print string (for debugging)
    registry.register(NodeType {
        id: "print_string".to_string(),
        name: "Print String".to_string(),
        category: NodeCategory::Utility,
        description: "Print a string to the console".to_string(),
        inputs: vec![
            Port::new(PortId::new(), "Exec", PortType::Exec, PortDirection::Input),
            Port::new(PortId::new(), "String", PortType::String, PortDirection::Input),
        ],
        outputs: vec![
            Port::new(PortId::new(), "Exec", PortType::Exec, PortDirection::Output),
        ],
    });

    registry
}
