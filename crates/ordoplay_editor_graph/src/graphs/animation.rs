// SPDX-License-Identifier: MIT OR Apache-2.0
//! Animation state machine graph.
//!
//! Supports states, transitions, and blend trees.

use crate::node::{NodeCategory, NodeRegistry, NodeType};
use crate::port::{Port, PortDirection, PortId, PortType};

/// Create the animation graph node registry
pub fn create_animation_registry() -> NodeRegistry {
    let mut registry = NodeRegistry::new();

    // State node
    registry.register(NodeType {
        id: "animation_state".to_string(),
        name: "Animation State".to_string(),
        category: NodeCategory::Custom,
        description: "An animation state".to_string(),
        inputs: vec![
            Port::new(PortId::new(), "Enter", PortType::Exec, PortDirection::Input),
        ],
        outputs: vec![
            Port::new(PortId::new(), "Exit", PortType::Exec, PortDirection::Output),
            Port::new(PortId::new(), "Pose", PortType::Any, PortDirection::Output),
        ],
    });

    // Blend node
    registry.register(NodeType {
        id: "blend_poses".to_string(),
        name: "Blend Poses".to_string(),
        category: NodeCategory::Math,
        description: "Blend between two poses".to_string(),
        inputs: vec![
            Port::new(PortId::new(), "Pose A", PortType::Any, PortDirection::Input),
            Port::new(PortId::new(), "Pose B", PortType::Any, PortDirection::Input),
            Port::new(PortId::new(), "Alpha", PortType::Float, PortDirection::Input),
        ],
        outputs: vec![
            Port::new(PortId::new(), "Pose", PortType::Any, PortDirection::Output),
        ],
    });

    // Output pose
    registry.register(NodeType {
        id: "output_pose".to_string(),
        name: "Output Pose".to_string(),
        category: NodeCategory::Output,
        description: "Final animation output".to_string(),
        inputs: vec![
            Port::new(PortId::new(), "Pose", PortType::Any, PortDirection::Input),
        ],
        outputs: vec![],
    });

    registry
}
