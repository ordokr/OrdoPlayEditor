// SPDX-License-Identifier: MIT OR Apache-2.0
//! Material/shader graph for visual material authoring.
//!
//! Provides a comprehensive set of nodes for creating PBR materials
//! compatible with ordoplay_materialx for runtime compilation to WGSL.

use crate::node::{NodeCategory, NodeRegistry, NodeType};
use crate::port::{Port, PortDirection, PortId, PortType, PortValue};

/// Create the material graph node registry with all available node types
pub fn create_material_registry() -> NodeRegistry {
    let mut registry = NodeRegistry::new();

    // ========================================================================
    // Output Nodes
    // ========================================================================

    registry.register(NodeType {
        id: "material_output".to_string(),
        name: "Material Output".to_string(),
        category: NodeCategory::Output,
        description: "Final PBR material output".to_string(),
        inputs: vec![
            Port::input("Base Color", PortType::Color).with_default(PortValue::Color([0.8, 0.8, 0.8, 1.0])),
            Port::input("Metallic", PortType::Float).with_default(PortValue::Float(0.0)),
            Port::input("Roughness", PortType::Float).with_default(PortValue::Float(0.5)),
            Port::input("Normal", PortType::Vector3),
            Port::input("Emission", PortType::Color).with_default(PortValue::Color([0.0, 0.0, 0.0, 1.0])),
            Port::input("Emission Strength", PortType::Float).with_default(PortValue::Float(0.0)),
            Port::input("Opacity", PortType::Float).with_default(PortValue::Float(1.0)),
            Port::input("Ambient Occlusion", PortType::Float).with_default(PortValue::Float(1.0)),
        ],
        outputs: vec![],
    });

    // Unlit output for special materials
    registry.register(NodeType {
        id: "unlit_output".to_string(),
        name: "Unlit Output".to_string(),
        category: NodeCategory::Output,
        description: "Unlit material output (no lighting)".to_string(),
        inputs: vec![
            Port::input("Color", PortType::Color).with_default(PortValue::Color([1.0, 1.0, 1.0, 1.0])),
            Port::input("Opacity", PortType::Float).with_default(PortValue::Float(1.0)),
        ],
        outputs: vec![],
    });

    // ========================================================================
    // Input Nodes - Constants
    // ========================================================================

    registry.register(NodeType {
        id: "color_constant".to_string(),
        name: "Color".to_string(),
        category: NodeCategory::Input,
        description: "Constant color value".to_string(),
        inputs: vec![],
        outputs: vec![Port::output("Color", PortType::Color)],
    });

    registry.register(NodeType {
        id: "float_constant".to_string(),
        name: "Float".to_string(),
        category: NodeCategory::Input,
        description: "Constant float value".to_string(),
        inputs: vec![],
        outputs: vec![Port::output("Value", PortType::Float)],
    });

    registry.register(NodeType {
        id: "vector2_constant".to_string(),
        name: "Vector2".to_string(),
        category: NodeCategory::Input,
        description: "Constant 2D vector value".to_string(),
        inputs: vec![],
        outputs: vec![Port::output("Vector", PortType::Vector2)],
    });

    registry.register(NodeType {
        id: "vector3_constant".to_string(),
        name: "Vector3".to_string(),
        category: NodeCategory::Input,
        description: "Constant 3D vector value".to_string(),
        inputs: vec![],
        outputs: vec![Port::output("Vector", PortType::Vector3)],
    });

    registry.register(NodeType {
        id: "vector4_constant".to_string(),
        name: "Vector4".to_string(),
        category: NodeCategory::Input,
        description: "Constant 4D vector value".to_string(),
        inputs: vec![],
        outputs: vec![Port::output("Vector", PortType::Vector4)],
    });

    // ========================================================================
    // Input Nodes - Coordinates & Parameters
    // ========================================================================

    registry.register(NodeType {
        id: "uv_coord".to_string(),
        name: "UV Coordinates".to_string(),
        category: NodeCategory::Input,
        description: "Mesh UV coordinates".to_string(),
        inputs: vec![],
        outputs: vec![
            Port::output("UV", PortType::Vector2),
            Port::output("U", PortType::Float),
            Port::output("V", PortType::Float),
        ],
    });

    registry.register(NodeType {
        id: "vertex_color".to_string(),
        name: "Vertex Color".to_string(),
        category: NodeCategory::Input,
        description: "Per-vertex color attribute".to_string(),
        inputs: vec![],
        outputs: vec![
            Port::output("Color", PortType::Color),
            Port::output("R", PortType::Float),
            Port::output("G", PortType::Float),
            Port::output("B", PortType::Float),
            Port::output("A", PortType::Float),
        ],
    });

    registry.register(NodeType {
        id: "world_position".to_string(),
        name: "World Position".to_string(),
        category: NodeCategory::Input,
        description: "Fragment world position".to_string(),
        inputs: vec![],
        outputs: vec![
            Port::output("Position", PortType::Vector3),
            Port::output("X", PortType::Float),
            Port::output("Y", PortType::Float),
            Port::output("Z", PortType::Float),
        ],
    });

    registry.register(NodeType {
        id: "world_normal".to_string(),
        name: "World Normal".to_string(),
        category: NodeCategory::Input,
        description: "Fragment world normal".to_string(),
        inputs: vec![],
        outputs: vec![Port::output("Normal", PortType::Vector3)],
    });

    registry.register(NodeType {
        id: "view_direction".to_string(),
        name: "View Direction".to_string(),
        category: NodeCategory::Input,
        description: "Direction from fragment to camera".to_string(),
        inputs: vec![],
        outputs: vec![Port::output("Direction", PortType::Vector3)],
    });

    registry.register(NodeType {
        id: "time".to_string(),
        name: "Time".to_string(),
        category: NodeCategory::Input,
        description: "Shader time values".to_string(),
        inputs: vec![],
        outputs: vec![
            Port::output("Time", PortType::Float),
            Port::output("Sin Time", PortType::Float),
            Port::output("Cos Time", PortType::Float),
            Port::output("Delta Time", PortType::Float),
        ],
    });

    // ========================================================================
    // Texture Nodes
    // ========================================================================

    registry.register(NodeType {
        id: "texture_sample".to_string(),
        name: "Texture Sample".to_string(),
        category: NodeCategory::Texture,
        description: "Sample a 2D texture".to_string(),
        inputs: vec![
            Port::input("Texture", PortType::Texture),
            Port::input("UV", PortType::Vector2),
        ],
        outputs: vec![
            Port::output("Color", PortType::Color),
            Port::output("R", PortType::Float),
            Port::output("G", PortType::Float),
            Port::output("B", PortType::Float),
            Port::output("A", PortType::Float),
        ],
    });

    registry.register(NodeType {
        id: "normal_map".to_string(),
        name: "Normal Map".to_string(),
        category: NodeCategory::Texture,
        description: "Sample and decode a normal map".to_string(),
        inputs: vec![
            Port::input("Texture", PortType::Texture),
            Port::input("UV", PortType::Vector2),
            Port::input("Strength", PortType::Float).with_default(PortValue::Float(1.0)),
        ],
        outputs: vec![Port::output("Normal", PortType::Vector3)],
    });

    registry.register(NodeType {
        id: "triplanar_mapping".to_string(),
        name: "Triplanar Mapping".to_string(),
        category: NodeCategory::Texture,
        description: "Triplanar texture projection".to_string(),
        inputs: vec![
            Port::input("Texture", PortType::Texture),
            Port::input("Position", PortType::Vector3),
            Port::input("Normal", PortType::Vector3),
            Port::input("Blend", PortType::Float).with_default(PortValue::Float(1.0)),
            Port::input("Scale", PortType::Float).with_default(PortValue::Float(1.0)),
        ],
        outputs: vec![Port::output("Color", PortType::Color)],
    });

    // ========================================================================
    // Math Nodes - Basic Operations
    // ========================================================================

    registry.register(NodeType {
        id: "add".to_string(),
        name: "Add".to_string(),
        category: NodeCategory::Math,
        description: "Add two values".to_string(),
        inputs: vec![
            Port::input("A", PortType::Any),
            Port::input("B", PortType::Any),
        ],
        outputs: vec![Port::output("Result", PortType::Any)],
    });

    registry.register(NodeType {
        id: "subtract".to_string(),
        name: "Subtract".to_string(),
        category: NodeCategory::Math,
        description: "Subtract B from A".to_string(),
        inputs: vec![
            Port::input("A", PortType::Any),
            Port::input("B", PortType::Any),
        ],
        outputs: vec![Port::output("Result", PortType::Any)],
    });

    registry.register(NodeType {
        id: "multiply".to_string(),
        name: "Multiply".to_string(),
        category: NodeCategory::Math,
        description: "Multiply two values".to_string(),
        inputs: vec![
            Port::input("A", PortType::Any),
            Port::input("B", PortType::Any),
        ],
        outputs: vec![Port::output("Result", PortType::Any)],
    });

    registry.register(NodeType {
        id: "divide".to_string(),
        name: "Divide".to_string(),
        category: NodeCategory::Math,
        description: "Divide A by B".to_string(),
        inputs: vec![
            Port::input("A", PortType::Any),
            Port::input("B", PortType::Any),
        ],
        outputs: vec![Port::output("Result", PortType::Any)],
    });

    registry.register(NodeType {
        id: "power".to_string(),
        name: "Power".to_string(),
        category: NodeCategory::Math,
        description: "Raise A to the power of B".to_string(),
        inputs: vec![
            Port::input("Base", PortType::Float),
            Port::input("Exponent", PortType::Float).with_default(PortValue::Float(2.0)),
        ],
        outputs: vec![Port::output("Result", PortType::Float)],
    });

    registry.register(NodeType {
        id: "sqrt".to_string(),
        name: "Square Root".to_string(),
        category: NodeCategory::Math,
        description: "Square root of value".to_string(),
        inputs: vec![Port::input("Value", PortType::Float)],
        outputs: vec![Port::output("Result", PortType::Float)],
    });

    registry.register(NodeType {
        id: "abs".to_string(),
        name: "Absolute".to_string(),
        category: NodeCategory::Math,
        description: "Absolute value".to_string(),
        inputs: vec![Port::input("Value", PortType::Any)],
        outputs: vec![Port::output("Result", PortType::Any)],
    });

    registry.register(NodeType {
        id: "negate".to_string(),
        name: "Negate".to_string(),
        category: NodeCategory::Math,
        description: "Negate value (-x)".to_string(),
        inputs: vec![Port::input("Value", PortType::Any)],
        outputs: vec![Port::output("Result", PortType::Any)],
    });

    // ========================================================================
    // Math Nodes - Trigonometry
    // ========================================================================

    registry.register(NodeType {
        id: "sin".to_string(),
        name: "Sine".to_string(),
        category: NodeCategory::Math,
        description: "Sine of angle (radians)".to_string(),
        inputs: vec![Port::input("Angle", PortType::Float)],
        outputs: vec![Port::output("Result", PortType::Float)],
    });

    registry.register(NodeType {
        id: "cos".to_string(),
        name: "Cosine".to_string(),
        category: NodeCategory::Math,
        description: "Cosine of angle (radians)".to_string(),
        inputs: vec![Port::input("Angle", PortType::Float)],
        outputs: vec![Port::output("Result", PortType::Float)],
    });

    registry.register(NodeType {
        id: "tan".to_string(),
        name: "Tangent".to_string(),
        category: NodeCategory::Math,
        description: "Tangent of angle (radians)".to_string(),
        inputs: vec![Port::input("Angle", PortType::Float)],
        outputs: vec![Port::output("Result", PortType::Float)],
    });

    registry.register(NodeType {
        id: "atan2".to_string(),
        name: "Atan2".to_string(),
        category: NodeCategory::Math,
        description: "Two-argument arctangent".to_string(),
        inputs: vec![
            Port::input("Y", PortType::Float),
            Port::input("X", PortType::Float),
        ],
        outputs: vec![Port::output("Angle", PortType::Float)],
    });

    // ========================================================================
    // Math Nodes - Interpolation & Clamping
    // ========================================================================

    registry.register(NodeType {
        id: "lerp".to_string(),
        name: "Lerp".to_string(),
        category: NodeCategory::Math,
        description: "Linear interpolation between A and B".to_string(),
        inputs: vec![
            Port::input("A", PortType::Any),
            Port::input("B", PortType::Any),
            Port::input("T", PortType::Float).with_default(PortValue::Float(0.5)),
        ],
        outputs: vec![Port::output("Result", PortType::Any)],
    });

    registry.register(NodeType {
        id: "smoothstep".to_string(),
        name: "Smoothstep".to_string(),
        category: NodeCategory::Math,
        description: "Hermite interpolation between edges".to_string(),
        inputs: vec![
            Port::input("Edge0", PortType::Float).with_default(PortValue::Float(0.0)),
            Port::input("Edge1", PortType::Float).with_default(PortValue::Float(1.0)),
            Port::input("X", PortType::Float),
        ],
        outputs: vec![Port::output("Result", PortType::Float)],
    });

    registry.register(NodeType {
        id: "clamp".to_string(),
        name: "Clamp".to_string(),
        category: NodeCategory::Math,
        description: "Clamp value between min and max".to_string(),
        inputs: vec![
            Port::input("Value", PortType::Any),
            Port::input("Min", PortType::Any),
            Port::input("Max", PortType::Any),
        ],
        outputs: vec![Port::output("Result", PortType::Any)],
    });

    registry.register(NodeType {
        id: "saturate".to_string(),
        name: "Saturate".to_string(),
        category: NodeCategory::Math,
        description: "Clamp value between 0 and 1".to_string(),
        inputs: vec![Port::input("Value", PortType::Any)],
        outputs: vec![Port::output("Result", PortType::Any)],
    });

    registry.register(NodeType {
        id: "min".to_string(),
        name: "Minimum".to_string(),
        category: NodeCategory::Math,
        description: "Minimum of two values".to_string(),
        inputs: vec![
            Port::input("A", PortType::Any),
            Port::input("B", PortType::Any),
        ],
        outputs: vec![Port::output("Result", PortType::Any)],
    });

    registry.register(NodeType {
        id: "max".to_string(),
        name: "Maximum".to_string(),
        category: NodeCategory::Math,
        description: "Maximum of two values".to_string(),
        inputs: vec![
            Port::input("A", PortType::Any),
            Port::input("B", PortType::Any),
        ],
        outputs: vec![Port::output("Result", PortType::Any)],
    });

    registry.register(NodeType {
        id: "floor".to_string(),
        name: "Floor".to_string(),
        category: NodeCategory::Math,
        description: "Round down to nearest integer".to_string(),
        inputs: vec![Port::input("Value", PortType::Any)],
        outputs: vec![Port::output("Result", PortType::Any)],
    });

    registry.register(NodeType {
        id: "ceil".to_string(),
        name: "Ceiling".to_string(),
        category: NodeCategory::Math,
        description: "Round up to nearest integer".to_string(),
        inputs: vec![Port::input("Value", PortType::Any)],
        outputs: vec![Port::output("Result", PortType::Any)],
    });

    registry.register(NodeType {
        id: "fract".to_string(),
        name: "Fraction".to_string(),
        category: NodeCategory::Math,
        description: "Fractional part of value".to_string(),
        inputs: vec![Port::input("Value", PortType::Any)],
        outputs: vec![Port::output("Result", PortType::Any)],
    });

    registry.register(NodeType {
        id: "one_minus".to_string(),
        name: "One Minus".to_string(),
        category: NodeCategory::Math,
        description: "One minus value (1 - x)".to_string(),
        inputs: vec![Port::input("Value", PortType::Any)],
        outputs: vec![Port::output("Result", PortType::Any)],
    });

    registry.register(NodeType {
        id: "remap".to_string(),
        name: "Remap".to_string(),
        category: NodeCategory::Math,
        description: "Remap value from one range to another".to_string(),
        inputs: vec![
            Port::input("Value", PortType::Float),
            Port::input("In Min", PortType::Float).with_default(PortValue::Float(0.0)),
            Port::input("In Max", PortType::Float).with_default(PortValue::Float(1.0)),
            Port::input("Out Min", PortType::Float).with_default(PortValue::Float(0.0)),
            Port::input("Out Max", PortType::Float).with_default(PortValue::Float(1.0)),
        ],
        outputs: vec![Port::output("Result", PortType::Float)],
    });

    // ========================================================================
    // Vector Operations
    // ========================================================================

    registry.register(NodeType {
        id: "dot".to_string(),
        name: "Dot Product".to_string(),
        category: NodeCategory::Math,
        description: "Dot product of two vectors".to_string(),
        inputs: vec![
            Port::input("A", PortType::Vector3),
            Port::input("B", PortType::Vector3),
        ],
        outputs: vec![Port::output("Result", PortType::Float)],
    });

    registry.register(NodeType {
        id: "cross".to_string(),
        name: "Cross Product".to_string(),
        category: NodeCategory::Math,
        description: "Cross product of two 3D vectors".to_string(),
        inputs: vec![
            Port::input("A", PortType::Vector3),
            Port::input("B", PortType::Vector3),
        ],
        outputs: vec![Port::output("Result", PortType::Vector3)],
    });

    registry.register(NodeType {
        id: "normalize".to_string(),
        name: "Normalize".to_string(),
        category: NodeCategory::Math,
        description: "Normalize vector to unit length".to_string(),
        inputs: vec![Port::input("Vector", PortType::Any)],
        outputs: vec![Port::output("Result", PortType::Any)],
    });

    registry.register(NodeType {
        id: "length".to_string(),
        name: "Length".to_string(),
        category: NodeCategory::Math,
        description: "Length of vector".to_string(),
        inputs: vec![Port::input("Vector", PortType::Any)],
        outputs: vec![Port::output("Length", PortType::Float)],
    });

    registry.register(NodeType {
        id: "distance".to_string(),
        name: "Distance".to_string(),
        category: NodeCategory::Math,
        description: "Distance between two points".to_string(),
        inputs: vec![
            Port::input("A", PortType::Any),
            Port::input("B", PortType::Any),
        ],
        outputs: vec![Port::output("Distance", PortType::Float)],
    });

    registry.register(NodeType {
        id: "reflect".to_string(),
        name: "Reflect".to_string(),
        category: NodeCategory::Math,
        description: "Reflect vector about normal".to_string(),
        inputs: vec![
            Port::input("Vector", PortType::Vector3),
            Port::input("Normal", PortType::Vector3),
        ],
        outputs: vec![Port::output("Result", PortType::Vector3)],
    });

    // ========================================================================
    // Vector Composition
    // ========================================================================

    registry.register(NodeType {
        id: "split_vector2".to_string(),
        name: "Split Vector2".to_string(),
        category: NodeCategory::Utility,
        description: "Split Vector2 into components".to_string(),
        inputs: vec![Port::input("Vector", PortType::Vector2)],
        outputs: vec![
            Port::output("X", PortType::Float),
            Port::output("Y", PortType::Float),
        ],
    });

    registry.register(NodeType {
        id: "split_vector3".to_string(),
        name: "Split Vector3".to_string(),
        category: NodeCategory::Utility,
        description: "Split Vector3 into components".to_string(),
        inputs: vec![Port::input("Vector", PortType::Vector3)],
        outputs: vec![
            Port::output("X", PortType::Float),
            Port::output("Y", PortType::Float),
            Port::output("Z", PortType::Float),
        ],
    });

    registry.register(NodeType {
        id: "split_vector4".to_string(),
        name: "Split Vector4".to_string(),
        category: NodeCategory::Utility,
        description: "Split Vector4 into components".to_string(),
        inputs: vec![Port::input("Vector", PortType::Vector4)],
        outputs: vec![
            Port::output("X", PortType::Float),
            Port::output("Y", PortType::Float),
            Port::output("Z", PortType::Float),
            Port::output("W", PortType::Float),
        ],
    });

    registry.register(NodeType {
        id: "combine_vector2".to_string(),
        name: "Combine Vector2".to_string(),
        category: NodeCategory::Utility,
        description: "Combine components into Vector2".to_string(),
        inputs: vec![
            Port::input("X", PortType::Float).with_default(PortValue::Float(0.0)),
            Port::input("Y", PortType::Float).with_default(PortValue::Float(0.0)),
        ],
        outputs: vec![Port::output("Vector", PortType::Vector2)],
    });

    registry.register(NodeType {
        id: "combine_vector3".to_string(),
        name: "Combine Vector3".to_string(),
        category: NodeCategory::Utility,
        description: "Combine components into Vector3".to_string(),
        inputs: vec![
            Port::input("X", PortType::Float).with_default(PortValue::Float(0.0)),
            Port::input("Y", PortType::Float).with_default(PortValue::Float(0.0)),
            Port::input("Z", PortType::Float).with_default(PortValue::Float(0.0)),
        ],
        outputs: vec![Port::output("Vector", PortType::Vector3)],
    });

    registry.register(NodeType {
        id: "combine_vector4".to_string(),
        name: "Combine Vector4".to_string(),
        category: NodeCategory::Utility,
        description: "Combine components into Vector4".to_string(),
        inputs: vec![
            Port::input("X", PortType::Float).with_default(PortValue::Float(0.0)),
            Port::input("Y", PortType::Float).with_default(PortValue::Float(0.0)),
            Port::input("Z", PortType::Float).with_default(PortValue::Float(0.0)),
            Port::input("W", PortType::Float).with_default(PortValue::Float(0.0)),
        ],
        outputs: vec![Port::output("Vector", PortType::Vector4)],
    });

    // ========================================================================
    // Color Operations
    // ========================================================================

    registry.register(NodeType {
        id: "hsv_to_rgb".to_string(),
        name: "HSV to RGB".to_string(),
        category: NodeCategory::Utility,
        description: "Convert HSV to RGB color".to_string(),
        inputs: vec![
            Port::input("H", PortType::Float),
            Port::input("S", PortType::Float),
            Port::input("V", PortType::Float),
        ],
        outputs: vec![Port::output("RGB", PortType::Color)],
    });

    registry.register(NodeType {
        id: "rgb_to_hsv".to_string(),
        name: "RGB to HSV".to_string(),
        category: NodeCategory::Utility,
        description: "Convert RGB to HSV color".to_string(),
        inputs: vec![Port::input("RGB", PortType::Color)],
        outputs: vec![
            Port::output("H", PortType::Float),
            Port::output("S", PortType::Float),
            Port::output("V", PortType::Float),
        ],
    });

    registry.register(NodeType {
        id: "contrast".to_string(),
        name: "Contrast".to_string(),
        category: NodeCategory::Utility,
        description: "Adjust color contrast".to_string(),
        inputs: vec![
            Port::input("Color", PortType::Color),
            Port::input("Contrast", PortType::Float).with_default(PortValue::Float(1.0)),
        ],
        outputs: vec![Port::output("Result", PortType::Color)],
    });

    registry.register(NodeType {
        id: "saturation".to_string(),
        name: "Saturation".to_string(),
        category: NodeCategory::Utility,
        description: "Adjust color saturation".to_string(),
        inputs: vec![
            Port::input("Color", PortType::Color),
            Port::input("Saturation", PortType::Float).with_default(PortValue::Float(1.0)),
        ],
        outputs: vec![Port::output("Result", PortType::Color)],
    });

    registry.register(NodeType {
        id: "hue_shift".to_string(),
        name: "Hue Shift".to_string(),
        category: NodeCategory::Utility,
        description: "Shift hue of color".to_string(),
        inputs: vec![
            Port::input("Color", PortType::Color),
            Port::input("Shift", PortType::Float).with_default(PortValue::Float(0.0)),
        ],
        outputs: vec![Port::output("Result", PortType::Color)],
    });

    registry.register(NodeType {
        id: "blend".to_string(),
        name: "Blend".to_string(),
        category: NodeCategory::Utility,
        description: "Blend two colors with various modes".to_string(),
        inputs: vec![
            Port::input("Base", PortType::Color),
            Port::input("Blend", PortType::Color),
            Port::input("Opacity", PortType::Float).with_default(PortValue::Float(1.0)),
        ],
        outputs: vec![Port::output("Result", PortType::Color)],
    });

    // ========================================================================
    // UV Operations
    // ========================================================================

    registry.register(NodeType {
        id: "uv_tiling".to_string(),
        name: "UV Tiling".to_string(),
        category: NodeCategory::Utility,
        description: "Tile and offset UV coordinates".to_string(),
        inputs: vec![
            Port::input("UV", PortType::Vector2),
            Port::input("Tiling", PortType::Vector2).with_default(PortValue::Vector2([1.0, 1.0])),
            Port::input("Offset", PortType::Vector2).with_default(PortValue::Vector2([0.0, 0.0])),
        ],
        outputs: vec![Port::output("UV", PortType::Vector2)],
    });

    registry.register(NodeType {
        id: "uv_rotate".to_string(),
        name: "UV Rotate".to_string(),
        category: NodeCategory::Utility,
        description: "Rotate UV coordinates".to_string(),
        inputs: vec![
            Port::input("UV", PortType::Vector2),
            Port::input("Center", PortType::Vector2).with_default(PortValue::Vector2([0.5, 0.5])),
            Port::input("Angle", PortType::Float).with_default(PortValue::Float(0.0)),
        ],
        outputs: vec![Port::output("UV", PortType::Vector2)],
    });

    registry.register(NodeType {
        id: "parallax_mapping".to_string(),
        name: "Parallax Mapping".to_string(),
        category: NodeCategory::Utility,
        description: "Parallax offset mapping".to_string(),
        inputs: vec![
            Port::input("UV", PortType::Vector2),
            Port::input("Height", PortType::Float),
            Port::input("Depth", PortType::Float).with_default(PortValue::Float(0.02)),
        ],
        outputs: vec![Port::output("UV", PortType::Vector2)],
    });

    // ========================================================================
    // Procedural Patterns
    // ========================================================================

    registry.register(NodeType {
        id: "noise_perlin".to_string(),
        name: "Perlin Noise".to_string(),
        category: NodeCategory::Utility,
        description: "Generate Perlin noise".to_string(),
        inputs: vec![
            Port::input("UV", PortType::Vector2),
            Port::input("Scale", PortType::Float).with_default(PortValue::Float(5.0)),
            Port::input("Octaves", PortType::Int).with_default(PortValue::Int(4)),
        ],
        outputs: vec![Port::output("Value", PortType::Float)],
    });

    registry.register(NodeType {
        id: "noise_voronoi".to_string(),
        name: "Voronoi Noise".to_string(),
        category: NodeCategory::Utility,
        description: "Generate Voronoi cellular noise".to_string(),
        inputs: vec![
            Port::input("UV", PortType::Vector2),
            Port::input("Scale", PortType::Float).with_default(PortValue::Float(5.0)),
        ],
        outputs: vec![
            Port::output("Distance", PortType::Float),
            Port::output("Cell", PortType::Float),
        ],
    });

    registry.register(NodeType {
        id: "checkerboard".to_string(),
        name: "Checkerboard".to_string(),
        category: NodeCategory::Utility,
        description: "Generate checkerboard pattern".to_string(),
        inputs: vec![
            Port::input("UV", PortType::Vector2),
            Port::input("Scale", PortType::Float).with_default(PortValue::Float(2.0)),
            Port::input("Color A", PortType::Color).with_default(PortValue::Color([0.0, 0.0, 0.0, 1.0])),
            Port::input("Color B", PortType::Color).with_default(PortValue::Color([1.0, 1.0, 1.0, 1.0])),
        ],
        outputs: vec![Port::output("Color", PortType::Color)],
    });

    registry.register(NodeType {
        id: "gradient".to_string(),
        name: "Gradient".to_string(),
        category: NodeCategory::Utility,
        description: "Generate linear gradient".to_string(),
        inputs: vec![
            Port::input("UV", PortType::Vector2),
            Port::input("Color A", PortType::Color).with_default(PortValue::Color([0.0, 0.0, 0.0, 1.0])),
            Port::input("Color B", PortType::Color).with_default(PortValue::Color([1.0, 1.0, 1.0, 1.0])),
        ],
        outputs: vec![Port::output("Color", PortType::Color)],
    });

    // ========================================================================
    // Fresnel / Effects
    // ========================================================================

    registry.register(NodeType {
        id: "fresnel".to_string(),
        name: "Fresnel".to_string(),
        category: NodeCategory::Utility,
        description: "Fresnel effect based on view angle".to_string(),
        inputs: vec![
            Port::input("Normal", PortType::Vector3),
            Port::input("Power", PortType::Float).with_default(PortValue::Float(5.0)),
        ],
        outputs: vec![Port::output("Result", PortType::Float)],
    });

    registry
}

/// Material graph panel state for the editor
pub struct MaterialGraphPanel {
    /// The material graph being edited
    pub graph: crate::Graph,
    /// Graph editor UI state
    pub editor_state: crate::ui::GraphEditorState,
    /// Node registry
    pub registry: NodeRegistry,
    /// Material name
    pub name: String,
    /// Whether the material has been modified
    pub dirty: bool,
}

impl MaterialGraphPanel {
    /// Create a new material graph panel
    pub fn new() -> Self {
        let registry = create_material_registry();
        let mut graph = crate::Graph::new("New Material");

        // Add default output node
        if let Some(output_node) = registry.create_node("material_output") {
            graph.add_node(output_node.with_position(400.0, 100.0));
        }

        Self {
            graph,
            editor_state: crate::ui::GraphEditorState::new(),
            registry,
            name: "New Material".to_string(),
            dirty: false,
        }
    }

    /// Render the material graph panel UI
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        // Toolbar
        ui.horizontal(|ui| {
            if ui.button("New").clicked() {
                *self = Self::new();
            }
            if ui.button("Save").clicked() {
                self.dirty = false;
                // TODO: Save material asset
            }
            ui.separator();

            // Add node menu
            ui.menu_button("Add Node", |ui| {
                self.add_node_menu(ui);
            });

            ui.separator();

            if ui.button("Compile").clicked() {
                // TODO: Compile to WGSL
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.dirty {
                    ui.label(egui::RichText::new("*").color(egui::Color32::YELLOW));
                }
                ui.label(&self.name);
            });
        });

        ui.separator();

        // Graph editor
        self.editor_state.ui_with_registry(ui, &mut self.graph, Some(&self.registry));
    }

    fn add_node_menu(&mut self, ui: &mut egui::Ui) {
        use crate::node::NodeCategory;

        // Group nodes by category
        for category in [
            NodeCategory::Input,
            NodeCategory::Output,
            NodeCategory::Math,
            NodeCategory::Texture,
            NodeCategory::Utility,
        ] {
            let category_name = match category {
                NodeCategory::Input => "Input",
                NodeCategory::Output => "Output",
                NodeCategory::Math => "Math",
                NodeCategory::Texture => "Texture",
                NodeCategory::Utility => "Utility",
                NodeCategory::Logic => "Logic",
                NodeCategory::Custom => "Custom",
            };

            ui.menu_button(category_name, |ui| {
                for node_type in self.registry.types_in_category(category) {
                    if ui.button(&node_type.name).on_hover_text(&node_type.description).clicked() {
                        if let Some(node) = self.registry.create_node(&node_type.id) {
                            // Place node near center of view
                            let pos = [
                                -self.editor_state.pan.x + 100.0,
                                -self.editor_state.pan.y + 100.0,
                            ];
                            self.graph.add_node(node.with_position(pos[0], pos[1]));
                            self.dirty = true;
                        }
                        ui.close_menu();
                    }
                }
            });
        }
    }
}

impl Default for MaterialGraphPanel {
    fn default() -> Self {
        Self::new()
    }
}
