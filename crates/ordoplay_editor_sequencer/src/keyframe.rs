// SPDX-License-Identifier: MIT OR Apache-2.0
//! Keyframe definitions for the sequencer.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a keyframe
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyframeId(pub Uuid);

impl KeyframeId {
    /// Create a new random keyframe ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for KeyframeId {
    fn default() -> Self {
        Self::new()
    }
}

/// Interpolation mode between keyframes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum InterpolationMode {
    /// Constant (step)
    Constant,
    /// Linear interpolation
    #[default]
    Linear,
    /// Cubic bezier interpolation
    Bezier,
    /// Auto-smooth
    Auto,
}


/// Value stored in a keyframe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyframeValue {
    /// Float value
    Float(f32),
    /// 2D vector
    Vec2([f32; 2]),
    /// 3D vector
    Vec3([f32; 3]),
    /// 4D vector / quaternion
    Vec4([f32; 4]),
    /// Color (RGBA)
    Color([f32; 4]),
    /// Boolean
    Bool(bool),
    /// Event (string identifier)
    Event(String),
}

/// A keyframe in a track
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyframe {
    /// Unique keyframe ID
    pub id: KeyframeId,
    /// Time in seconds
    pub time: f32,
    /// Value at this keyframe
    pub value: KeyframeValue,
    /// Interpolation mode to next keyframe
    pub interpolation: InterpolationMode,
    /// In-tangent for bezier curves
    pub in_tangent: Option<[f32; 2]>,
    /// Out-tangent for bezier curves
    pub out_tangent: Option<[f32; 2]>,
}

impl Keyframe {
    /// Create a new keyframe
    pub fn new(time: f32, value: KeyframeValue) -> Self {
        Self {
            id: KeyframeId::new(),
            time,
            value,
            interpolation: InterpolationMode::Linear,
            in_tangent: None,
            out_tangent: None,
        }
    }

    /// Set interpolation mode
    pub fn with_interpolation(mut self, mode: InterpolationMode) -> Self {
        self.interpolation = mode;
        self
    }

    /// Set tangents for bezier interpolation
    pub fn with_tangents(mut self, in_tangent: [f32; 2], out_tangent: [f32; 2]) -> Self {
        self.in_tangent = Some(in_tangent);
        self.out_tangent = Some(out_tangent);
        self
    }
}

/// Interpolation utilities
pub struct Interpolation;

impl Interpolation {
    /// Linear interpolation between two floats
    pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }

    /// Cubic bezier interpolation
    pub fn bezier(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;

        p0 * mt3 + 3.0 * p1 * mt2 * t + 3.0 * p2 * mt * t2 + p3 * t3
    }

    /// Hermite spline interpolation (for auto-smooth)
    pub fn hermite(p0: f32, m0: f32, p1: f32, m1: f32, t: f32) -> f32 {
        let t2 = t * t;
        let t3 = t2 * t;

        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;

        h00 * p0 + h10 * m0 + h01 * p1 + h11 * m1
    }

    /// Interpolate Vec2
    pub fn lerp_vec2(a: [f32; 2], b: [f32; 2], t: f32) -> [f32; 2] {
        [Self::lerp(a[0], b[0], t), Self::lerp(a[1], b[1], t)]
    }

    /// Interpolate Vec3
    pub fn lerp_vec3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
        [
            Self::lerp(a[0], b[0], t),
            Self::lerp(a[1], b[1], t),
            Self::lerp(a[2], b[2], t),
        ]
    }

    /// Interpolate Vec4
    pub fn lerp_vec4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
        [
            Self::lerp(a[0], b[0], t),
            Self::lerp(a[1], b[1], t),
            Self::lerp(a[2], b[2], t),
            Self::lerp(a[3], b[3], t),
        ]
    }

    /// Spherical linear interpolation for quaternions
    pub fn slerp(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
        let mut dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];

        // Handle opposite quaternions
        let mut b = b;
        if dot < 0.0 {
            b = [-b[0], -b[1], -b[2], -b[3]];
            dot = -dot;
        }

        // Use lerp for very close quaternions
        if dot > 0.9995 {
            let result = Self::lerp_vec4(a, b, t);
            // Normalize
            let len = (result[0] * result[0] + result[1] * result[1]
                     + result[2] * result[2] + result[3] * result[3]).sqrt();
            return [result[0] / len, result[1] / len, result[2] / len, result[3] / len];
        }

        let theta_0 = dot.acos();
        let theta = theta_0 * t;
        let sin_theta = theta.sin();
        let sin_theta_0 = theta_0.sin();

        let s0 = (theta_0 - theta).cos() - dot * sin_theta / sin_theta_0;
        let s1 = sin_theta / sin_theta_0;

        [
            a[0] * s0 + b[0] * s1,
            a[1] * s0 + b[1] * s1,
            a[2] * s0 + b[2] * s1,
            a[3] * s0 + b[3] * s1,
        ]
    }
}

impl KeyframeValue {
    /// Interpolate between two keyframe values
    pub fn interpolate(&self, other: &KeyframeValue, t: f32, mode: InterpolationMode) -> Option<KeyframeValue> {
        match mode {
            InterpolationMode::Constant => Some(self.clone()),
            InterpolationMode::Linear | InterpolationMode::Auto => {
                match (self, other) {
                    (KeyframeValue::Float(a), KeyframeValue::Float(b)) => {
                        Some(KeyframeValue::Float(Interpolation::lerp(*a, *b, t)))
                    }
                    (KeyframeValue::Vec2(a), KeyframeValue::Vec2(b)) => {
                        Some(KeyframeValue::Vec2(Interpolation::lerp_vec2(*a, *b, t)))
                    }
                    (KeyframeValue::Vec3(a), KeyframeValue::Vec3(b)) => {
                        Some(KeyframeValue::Vec3(Interpolation::lerp_vec3(*a, *b, t)))
                    }
                    (KeyframeValue::Vec4(a), KeyframeValue::Vec4(b)) => {
                        Some(KeyframeValue::Vec4(Interpolation::slerp(*a, *b, t)))
                    }
                    (KeyframeValue::Color(a), KeyframeValue::Color(b)) => {
                        Some(KeyframeValue::Color(Interpolation::lerp_vec4(*a, *b, t)))
                    }
                    (KeyframeValue::Bool(a), KeyframeValue::Bool(_)) => {
                        Some(KeyframeValue::Bool(*a)) // No interpolation for bool
                    }
                    (KeyframeValue::Event(a), KeyframeValue::Event(_)) => {
                        Some(KeyframeValue::Event(a.clone())) // No interpolation for events
                    }
                    _ => None, // Mismatched types
                }
            }
            InterpolationMode::Bezier => {
                // Bezier uses same logic for now (tangents handled at track level)
                self.interpolate(other, t, InterpolationMode::Linear)
            }
        }
    }

    /// Get as float if possible
    pub fn as_float(&self) -> Option<f32> {
        match self {
            KeyframeValue::Float(v) => Some(*v),
            _ => None,
        }
    }

    /// Get as Vec3 if possible
    pub fn as_vec3(&self) -> Option<[f32; 3]> {
        match self {
            KeyframeValue::Vec3(v) => Some(*v),
            _ => None,
        }
    }

    /// Get as Vec4/quaternion if possible
    pub fn as_vec4(&self) -> Option<[f32; 4]> {
        match self {
            KeyframeValue::Vec4(v) => Some(*v),
            _ => None,
        }
    }

    /// Get as color if possible
    pub fn as_color(&self) -> Option<[f32; 4]> {
        match self {
            KeyframeValue::Color(v) => Some(*v),
            _ => None,
        }
    }
}
