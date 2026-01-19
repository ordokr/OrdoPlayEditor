// SPDX-License-Identifier: MIT OR Apache-2.0
//! Port definitions for node inputs/outputs.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a port
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PortId(pub Uuid);

impl PortId {
    /// Create a new random port ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for PortId {
    fn default() -> Self {
        Self::new()
    }
}

/// Port direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortDirection {
    /// Input port
    Input,
    /// Output port
    Output,
}

/// Data type that can flow through ports
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PortType {
    /// Execution flow (for gameplay graphs)
    Exec,
    /// Boolean value
    Bool,
    /// Integer value
    Int,
    /// Floating point value
    Float,
    /// 2D vector
    Vector2,
    /// 3D vector
    Vector3,
    /// 4D vector / Color
    Vector4,
    /// Color (RGBA)
    Color,
    /// Matrix 4x4
    Mat4,
    /// Texture sampler
    Texture,
    /// Material reference
    Material,
    /// Entity reference
    Entity,
    /// String value
    String,
    /// Any type (for generic nodes)
    Any,
    /// Custom type
    Custom(String),
}

impl PortType {
    /// Get the color for this port type (for UI)
    pub fn color(&self) -> [u8; 3] {
        match self {
            Self::Exec => [200, 200, 200],
            Self::Bool => [200, 80, 80],
            Self::Int => [80, 200, 200],
            Self::Float => [80, 200, 80],
            Self::Vector2 => [200, 200, 80],
            Self::Vector3 => [200, 150, 80],
            Self::Vector4 => [200, 100, 200],
            Self::Color => [255, 200, 100],
            Self::Mat4 => [150, 100, 200],
            Self::Texture => [100, 150, 200],
            Self::Material => [200, 100, 150],
            Self::Entity => [150, 200, 150],
            Self::String => [200, 180, 150],
            Self::Any => [150, 150, 150],
            Self::Custom(_) => [128, 128, 128],
        }
    }

    /// Check if this type can connect to another type
    pub fn can_connect_to(&self, other: &PortType) -> bool {
        // Any type can connect to anything
        if matches!(self, Self::Any) || matches!(other, Self::Any) {
            return true;
        }

        // Same types can always connect
        if self == other {
            return true;
        }

        // Implicit conversions
        match (self, other) {
            // Numeric conversions
            (Self::Int, Self::Float) | (Self::Float, Self::Int) => true,
            // Vector conversions
            (Self::Float, Self::Vector2 | Self::Vector3 | Self::Vector4) => true,
            (Self::Vector2, Self::Vector3 | Self::Vector4) => true,
            (Self::Vector3, Self::Vector4) => true,
            // Color conversions
            (Self::Color, Self::Vector4) | (Self::Vector4, Self::Color) => true,
            // No other implicit conversions
            _ => false,
        }
    }
}

/// A port on a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Port {
    /// Unique port ID
    pub id: PortId,
    /// Port name
    pub name: String,
    /// Port direction
    pub direction: PortDirection,
    /// Data type
    pub port_type: PortType,
    /// Default value (for inputs)
    pub default_value: Option<PortValue>,
    /// Whether this port is required (for inputs)
    pub required: bool,
    /// Whether multiple connections are allowed
    pub multi_connect: bool,
}

impl Port {
    /// Create a new port
    pub fn new(
        id: PortId,
        name: impl Into<String>,
        port_type: PortType,
        direction: PortDirection,
    ) -> Self {
        let multi_connect = direction == PortDirection::Output;
        Self {
            id,
            name: name.into(),
            direction,
            port_type,
            default_value: None,
            required: false,
            multi_connect,
        }
    }

    /// Create a new input port
    pub fn input(name: impl Into<String>, port_type: PortType) -> Self {
        Self {
            id: PortId::new(),
            name: name.into(),
            direction: PortDirection::Input,
            port_type,
            default_value: None,
            required: false,
            multi_connect: false,
        }
    }

    /// Create a new output port
    pub fn output(name: impl Into<String>, port_type: PortType) -> Self {
        Self {
            id: PortId::new(),
            name: name.into(),
            direction: PortDirection::Output,
            port_type,
            default_value: None,
            required: false,
            multi_connect: true, // Outputs can have multiple connections by default
        }
    }

    /// Set the default value
    pub fn with_default(mut self, value: PortValue) -> Self {
        self.default_value = Some(value);
        self
    }

    /// Mark as required
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Check if a connection to another port is valid
    pub fn can_connect(&self, other: &Port) -> bool {
        // Must be opposite directions
        if self.direction == other.direction {
            return false;
        }

        // Check type compatibility
        self.port_type.can_connect_to(&other.port_type)
    }
}

/// Value that can be stored in a port
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PortValue {
    /// Boolean
    Bool(bool),
    /// Integer
    Int(i32),
    /// Float
    Float(f32),
    /// 2D vector
    Vector2([f32; 2]),
    /// 3D vector
    Vector3([f32; 3]),
    /// 4D vector
    Vector4([f32; 4]),
    /// Color
    Color([f32; 4]),
    /// String
    String(String),
}

impl PortValue {
    /// Get the port type for this value
    pub fn port_type(&self) -> PortType {
        match self {
            Self::Bool(_) => PortType::Bool,
            Self::Int(_) => PortType::Int,
            Self::Float(_) => PortType::Float,
            Self::Vector2(_) => PortType::Vector2,
            Self::Vector3(_) => PortType::Vector3,
            Self::Vector4(_) => PortType::Vector4,
            Self::Color(_) => PortType::Color,
            Self::String(_) => PortType::String,
        }
    }
}
