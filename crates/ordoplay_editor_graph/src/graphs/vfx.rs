// SPDX-License-Identifier: MIT OR Apache-2.0
//! VFX graph for particle and effect authoring.
//!
//! Supports particle systems and GPU simulation.

use crate::node::{NodeCategory, NodeRegistry, NodeType};
use crate::port::{Port, PortDirection, PortId, PortType};

/// Create the VFX graph node registry
pub fn create_vfx_registry() -> NodeRegistry {
    let mut registry = NodeRegistry::new();

    // Particle spawn
    registry.register(NodeType {
        id: "spawn_rate".to_string(),
        name: "Spawn Rate".to_string(),
        category: NodeCategory::Input,
        description: "Spawn particles at a rate".to_string(),
        inputs: vec![
            Port::new(PortId::new(), "Rate", PortType::Float, PortDirection::Input),
        ],
        outputs: vec![
            Port::new(PortId::new(), "Spawn", PortType::Exec, PortDirection::Output),
        ],
    });

    // Initialize position
    registry.register(NodeType {
        id: "init_position".to_string(),
        name: "Initialize Position".to_string(),
        category: NodeCategory::Utility,
        description: "Set initial particle position".to_string(),
        inputs: vec![
            Port::new(PortId::new(), "Position", PortType::Vector3, PortDirection::Input),
        ],
        outputs: vec![
            Port::new(PortId::new(), "Position", PortType::Vector3, PortDirection::Output),
        ],
    });

    // Initialize velocity
    registry.register(NodeType {
        id: "init_velocity".to_string(),
        name: "Initialize Velocity".to_string(),
        category: NodeCategory::Utility,
        description: "Set initial particle velocity".to_string(),
        inputs: vec![
            Port::new(PortId::new(), "Direction", PortType::Vector3, PortDirection::Input),
            Port::new(PortId::new(), "Speed", PortType::Float, PortDirection::Input),
        ],
        outputs: vec![
            Port::new(PortId::new(), "Velocity", PortType::Vector3, PortDirection::Output),
        ],
    });

    // Output
    registry.register(NodeType {
        id: "vfx_output".to_string(),
        name: "VFX Output".to_string(),
        category: NodeCategory::Output,
        description: "Final particle output".to_string(),
        inputs: vec![
            Port::new(PortId::new(), "Position", PortType::Vector3, PortDirection::Input),
            Port::new(PortId::new(), "Velocity", PortType::Vector3, PortDirection::Input),
            Port::new(PortId::new(), "Color", PortType::Color, PortDirection::Input),
            Port::new(PortId::new(), "Size", PortType::Float, PortDirection::Input),
        ],
        outputs: vec![],
    });

    registry
}
