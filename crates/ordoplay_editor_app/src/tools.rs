// SPDX-License-Identifier: MIT OR Apache-2.0
//! Editor tools (gizmos, transform handles, etc.)

use serde::{Deserialize, Serialize};

/// Gizmo mode for transform operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GizmoMode {
    /// Translate (move) mode - W key
    #[default]
    Translate,
    /// Rotate mode - E key
    Rotate,
    /// Scale mode - R key
    Scale,
}

impl GizmoMode {
    /// Get the name of this mode
    pub fn name(&self) -> &'static str {
        match self {
            Self::Translate => "Translate",
            Self::Rotate => "Rotate",
            Self::Scale => "Scale",
        }
    }

    /// Get the hotkey for this mode
    pub fn hotkey(&self) -> char {
        match self {
            Self::Translate => 'W',
            Self::Rotate => 'E',
            Self::Scale => 'R',
        }
    }

    /// Get the icon for this mode
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Translate => "\u{f0b2}",  // arrows-alt
            Self::Rotate => "\u{f2f1}",     // sync
            Self::Scale => "\u{f424}",      // expand-arrows-alt
        }
    }
}

/// Axis constraint for gizmo operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AxisConstraint {
    /// No constraint - free movement
    #[default]
    None,
    /// Constrain to X axis
    X,
    /// Constrain to Y axis
    Y,
    /// Constrain to Z axis
    Z,
    /// Constrain to XY plane
    XY,
    /// Constrain to XZ plane
    XZ,
    /// Constrain to YZ plane
    YZ,
}

impl AxisConstraint {
    /// Get the axis mask as a Vec3 (1.0 = active, 0.0 = constrained)
    pub fn mask(&self) -> [f32; 3] {
        match self {
            Self::None => [1.0, 1.0, 1.0],
            Self::X => [1.0, 0.0, 0.0],
            Self::Y => [0.0, 1.0, 0.0],
            Self::Z => [0.0, 0.0, 1.0],
            Self::XY => [1.0, 1.0, 0.0],
            Self::XZ => [1.0, 0.0, 1.0],
            Self::YZ => [0.0, 1.0, 1.0],
        }
    }
}

/// State for an active gizmo operation
#[derive(Debug, Clone)]
pub struct GizmoOperation {
    /// Current gizmo mode
    pub mode: GizmoMode,
    /// Axis constraint
    pub constraint: AxisConstraint,
    /// Starting mouse position (screen space)
    pub start_pos: [f32; 2],
    /// Current mouse position (screen space)
    pub current_pos: [f32; 2],
    /// Accumulated delta
    pub delta: [f32; 3],
    /// Whether the operation is active
    pub active: bool,
}

impl GizmoOperation {
    /// Create a new inactive operation
    pub fn new(mode: GizmoMode) -> Self {
        Self {
            mode,
            constraint: AxisConstraint::None,
            start_pos: [0.0, 0.0],
            current_pos: [0.0, 0.0],
            delta: [0.0, 0.0, 0.0],
            active: false,
        }
    }

    /// Begin the operation
    pub fn begin(&mut self, pos: [f32; 2]) {
        self.start_pos = pos;
        self.current_pos = pos;
        self.delta = [0.0, 0.0, 0.0];
        self.active = true;
    }

    /// Update the operation with new mouse position
    pub fn update(&mut self, pos: [f32; 2]) {
        self.current_pos = pos;
    }

    /// End the operation
    pub fn end(&mut self) {
        self.active = false;
    }

    /// Cancel the operation
    pub fn cancel(&mut self) {
        self.active = false;
        self.delta = [0.0, 0.0, 0.0];
    }
}

/// Editor camera controls
#[derive(Debug, Clone)]
pub struct EditorCamera {
    /// Camera position
    pub position: [f32; 3],
    /// Camera target (look-at point)
    pub target: [f32; 3],
    /// Camera up vector
    pub up: [f32; 3],
    /// Field of view in degrees
    pub fov: f32,
    /// Near clip plane
    pub near: f32,
    /// Far clip plane
    pub far: f32,
    /// Orbit distance from target
    pub distance: f32,
    /// Orbit yaw angle in radians
    pub yaw: f32,
    /// Orbit pitch angle in radians
    pub pitch: f32,
    /// Movement speed
    pub move_speed: f32,
    /// Rotation speed
    pub rotate_speed: f32,
    /// Zoom speed
    pub zoom_speed: f32,
}

impl Default for EditorCamera {
    fn default() -> Self {
        Self {
            position: [5.0, 5.0, 5.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            fov: 60.0,
            near: 0.1,
            far: 10000.0,
            distance: 10.0,
            yaw: std::f32::consts::FRAC_PI_4,
            pitch: std::f32::consts::FRAC_PI_6,
            move_speed: 10.0,
            rotate_speed: 0.01,
            zoom_speed: 1.0,
        }
    }
}

impl EditorCamera {
    /// Create a new editor camera
    pub fn new() -> Self {
        Self::default()
    }

    /// Orbit the camera around the target
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        self.yaw += delta_x * self.rotate_speed;
        self.pitch += delta_y * self.rotate_speed;

        // Clamp pitch to avoid gimbal lock
        self.pitch = self.pitch.clamp(-std::f32::consts::FRAC_PI_2 + 0.01, std::f32::consts::FRAC_PI_2 - 0.01);

        self.update_position();
    }

    /// Pan the camera (move target)
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        // Calculate right and up vectors in camera space
        let right = self.get_right();
        let up = self.get_up();

        let pan_speed = self.distance * 0.001;
        let offset_x = right.map(|v| v * -delta_x * pan_speed);
        let offset_y = up.map(|v| v * delta_y * pan_speed);

        for i in 0..3 {
            self.target[i] += offset_x[i] + offset_y[i];
        }

        self.update_position();
    }

    /// Zoom the camera (change distance)
    pub fn zoom(&mut self, delta: f32) {
        self.distance *= 1.0 - delta * self.zoom_speed * 0.1;
        self.distance = self.distance.clamp(0.1, 10000.0);
        self.update_position();
    }

    /// Focus on a point
    pub fn focus(&mut self, target: [f32; 3], distance: Option<f32>) {
        self.target = target;
        if let Some(d) = distance {
            self.distance = d;
        }
        self.update_position();
    }

    /// Update camera position from orbit parameters
    fn update_position(&mut self) {
        let x = self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.cos();

        self.position = [
            self.target[0] + x,
            self.target[1] + y,
            self.target[2] + z,
        ];
    }

    /// Get the camera forward direction
    pub fn get_forward(&self) -> [f32; 3] {
        let dx = self.target[0] - self.position[0];
        let dy = self.target[1] - self.position[1];
        let dz = self.target[2] - self.position[2];
        let len = (dx * dx + dy * dy + dz * dz).sqrt();
        [dx / len, dy / len, dz / len]
    }

    /// Get the camera right direction
    pub fn get_right(&self) -> [f32; 3] {
        let forward = self.get_forward();
        // Cross product: up x forward
        [
            self.up[1] * forward[2] - self.up[2] * forward[1],
            self.up[2] * forward[0] - self.up[0] * forward[2],
            self.up[0] * forward[1] - self.up[1] * forward[0],
        ]
    }

    /// Get the camera up direction (orthogonalized)
    pub fn get_up(&self) -> [f32; 3] {
        let forward = self.get_forward();
        let right = self.get_right();
        // Cross product: forward x right
        [
            forward[1] * right[2] - forward[2] * right[1],
            forward[2] * right[0] - forward[0] * right[2],
            forward[0] * right[1] - forward[1] * right[0],
        ]
    }
}
