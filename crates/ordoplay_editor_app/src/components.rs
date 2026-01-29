// SPDX-License-Identifier: MIT OR Apache-2.0
//! Component system for entity composition.
//!
//! This module defines the available components that can be attached to entities,
//! along with their serialization and default values.


use serde::{Deserialize, Serialize};

/// Unique identifier for component types
pub type ComponentTypeId = &'static str;

/// Registry of all available component types
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone)]
pub struct ComponentInfo {
    /// Unique type identifier
    pub type_id: ComponentTypeId,
    /// Display name for UI
    pub display_name: &'static str,
    /// Category for grouping in add menu
    pub category: &'static str,
    /// Description for tooltips
    pub description: &'static str,
    /// Factory function to create default instance
    pub create_default: fn() -> Component,
}

/// All component types that can be attached to entities
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Component {
    /// Mesh renderer for 3D models
    MeshRenderer(MeshRendererComponent),
    /// Light source
    Light(LightComponent),
    /// Camera
    Camera(CameraComponent),
    /// Rigidbody for physics
    Rigidbody(RigidbodyComponent),
    /// Box collider
    BoxCollider(BoxColliderComponent),
    /// Sphere collider
    SphereCollider(SphereColliderComponent),
    /// Capsule collider
    CapsuleCollider(CapsuleColliderComponent),
    /// Mesh collider
    MeshCollider(MeshColliderComponent),
    /// Physics material
    PhysicsMaterial(PhysicsMaterialComponent),
    /// Audio source
    AudioSource(AudioSourceComponent),
    /// Script attachment
    Script(ScriptComponent),
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl Component {
    /// Get the type ID for this component
    pub fn type_id(&self) -> ComponentTypeId {
        match self {
            Component::MeshRenderer(_) => "MeshRenderer",
            Component::Light(_) => "Light",
            Component::Camera(_) => "Camera",
            Component::Rigidbody(_) => "Rigidbody",
            Component::BoxCollider(_) => "BoxCollider",
            Component::SphereCollider(_) => "SphereCollider",
            Component::CapsuleCollider(_) => "CapsuleCollider",
            Component::MeshCollider(_) => "MeshCollider",
            Component::PhysicsMaterial(_) => "PhysicsMaterial",
            Component::AudioSource(_) => "AudioSource",
            Component::Script(_) => "Script",
        }
    }

    /// Get display name for this component type
    pub fn display_name(&self) -> &'static str {
        match self {
            Component::MeshRenderer(_) => "Mesh Renderer",
            Component::Light(_) => "Light",
            Component::Camera(_) => "Camera",
            Component::Rigidbody(_) => "Rigidbody",
            Component::BoxCollider(_) => "Box Collider",
            Component::SphereCollider(_) => "Sphere Collider",
            Component::CapsuleCollider(_) => "Capsule Collider",
            Component::MeshCollider(_) => "Mesh Collider",
            Component::PhysicsMaterial(_) => "Physics Material",
            Component::AudioSource(_) => "Audio Source",
            Component::Script(_) => "Script",
        }
    }
}

// ============================================================================
// Component Definitions
// ============================================================================

/// Mesh renderer component for displaying 3D models
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MeshRendererComponent {
    /// Path to the mesh asset
    pub mesh: String,
    /// Path to the material asset
    pub material: String,
    /// Whether to cast shadows
    pub cast_shadows: bool,
    /// Whether to receive shadows
    pub receive_shadows: bool,
}

impl Default for MeshRendererComponent {
    fn default() -> Self {
        Self {
            mesh: String::new(),
            material: String::new(),
            cast_shadows: true,
            receive_shadows: true,
        }
    }
}

/// Light types
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LightType {
    Directional,
    Point,
    Spot,
}

impl Default for LightType {
    fn default() -> Self {
        Self::Point
    }
}

/// Light component for illumination
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LightComponent {
    /// Type of light
    pub light_type: LightType,
    /// Light color (RGB, 0-1)
    pub color: [f32; 3],
    /// Light intensity
    pub intensity: f32,
    /// Range for point/spot lights
    pub range: f32,
    /// Spot angle in degrees (for spot lights)
    pub spot_angle: f32,
    /// Whether the light casts shadows
    pub cast_shadows: bool,
}

impl Default for LightComponent {
    fn default() -> Self {
        Self {
            light_type: LightType::Point,
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
            range: 10.0,
            spot_angle: 45.0,
            cast_shadows: true,
        }
    }
}

/// Camera component for rendering viewpoints
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CameraComponent {
    /// Field of view in degrees
    pub fov: f32,
    /// Near clipping plane
    pub near: f32,
    /// Far clipping plane
    pub far: f32,
    /// Clear color (RGBA)
    pub clear_color: [f32; 4],
    /// Whether this is the main camera
    pub is_main: bool,
}

impl Default for CameraComponent {
    fn default() -> Self {
        Self {
            fov: 60.0,
            near: 0.1,
            far: 1000.0,
            clear_color: [0.1, 0.1, 0.15, 1.0],
            is_main: false,
        }
    }
}

/// Rigidbody type
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum RigidbodyType {
    /// Dynamic body - fully simulated
    #[default]
    Dynamic,
    /// Kinematic body - moved by code, affects dynamic bodies
    Kinematic,
    /// Static body - never moves
    Static,
}

/// Collision detection mode
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CollisionDetection {
    /// Discrete collision detection (default, faster)
    #[default]
    Discrete,
    /// Continuous collision detection (better for fast objects)
    Continuous,
    /// Continuous dynamic (best quality, slowest)
    ContinuousDynamic,
}

/// Interpolation mode for smooth rendering
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum RigidbodyInterpolation {
    /// No interpolation
    #[default]
    None,
    /// Interpolate between previous and current position
    Interpolate,
    /// Extrapolate based on velocity
    Extrapolate,
}

/// Axis constraints for freezing position/rotation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RigidbodyConstraints {
    /// Freeze position on X axis
    pub freeze_position_x: bool,
    /// Freeze position on Y axis
    pub freeze_position_y: bool,
    /// Freeze position on Z axis
    pub freeze_position_z: bool,
    /// Freeze rotation on X axis
    pub freeze_rotation_x: bool,
    /// Freeze rotation on Y axis
    pub freeze_rotation_y: bool,
    /// Freeze rotation on Z axis
    pub freeze_rotation_z: bool,
}

/// Rigidbody component for physics simulation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RigidbodyComponent {
    /// Body type (dynamic, kinematic, static)
    pub body_type: RigidbodyType,
    /// Mass in kilograms
    pub mass: f32,
    /// Linear drag (air resistance)
    pub drag: f32,
    /// Angular drag (rotational resistance)
    pub angular_drag: f32,
    /// Whether to use gravity
    pub use_gravity: bool,
    /// Collision detection mode
    pub collision_detection: CollisionDetection,
    /// Interpolation mode
    pub interpolation: RigidbodyInterpolation,
    /// Axis constraints
    pub constraints: RigidbodyConstraints,
    /// Center of mass offset
    pub center_of_mass: [f32; 3],
    /// Initial linear velocity
    pub initial_velocity: [f32; 3],
    /// Initial angular velocity (degrees/sec)
    pub initial_angular_velocity: [f32; 3],
}

impl Default for RigidbodyComponent {
    fn default() -> Self {
        Self {
            body_type: RigidbodyType::Dynamic,
            mass: 1.0,
            drag: 0.0,
            angular_drag: 0.05,
            use_gravity: true,
            collision_detection: CollisionDetection::Discrete,
            interpolation: RigidbodyInterpolation::None,
            constraints: RigidbodyConstraints::default(),
            center_of_mass: [0.0, 0.0, 0.0],
            initial_velocity: [0.0, 0.0, 0.0],
            initial_angular_velocity: [0.0, 0.0, 0.0],
        }
    }
}

/// Box collider component
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoxColliderComponent {
    /// Size of the box (width, height, depth)
    pub size: [f32; 3],
    /// Center offset
    pub center: [f32; 3],
    /// Whether this is a trigger (no physics response)
    pub is_trigger: bool,
    /// Collision layer
    pub layer: u32,
}

impl Default for BoxColliderComponent {
    fn default() -> Self {
        Self {
            size: [1.0, 1.0, 1.0],
            center: [0.0, 0.0, 0.0],
            is_trigger: false,
            layer: 0,
        }
    }
}

/// Sphere collider component
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SphereColliderComponent {
    /// Radius of the sphere
    pub radius: f32,
    /// Center offset
    pub center: [f32; 3],
    /// Whether this is a trigger
    pub is_trigger: bool,
    /// Collision layer
    pub layer: u32,
}

impl Default for SphereColliderComponent {
    fn default() -> Self {
        Self {
            radius: 0.5,
            center: [0.0, 0.0, 0.0],
            is_trigger: false,
            layer: 0,
        }
    }
}

/// Capsule direction axis
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CapsuleDirection {
    /// Capsule along X axis
    X,
    /// Capsule along Y axis (default, standing capsule)
    #[default]
    Y,
    /// Capsule along Z axis
    Z,
}

/// Capsule collider component
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapsuleColliderComponent {
    /// Radius of the capsule
    pub radius: f32,
    /// Height of the capsule (including caps)
    pub height: f32,
    /// Direction axis of the capsule
    pub direction: CapsuleDirection,
    /// Center offset
    pub center: [f32; 3],
    /// Whether this is a trigger
    pub is_trigger: bool,
    /// Collision layer
    pub layer: u32,
}

impl Default for CapsuleColliderComponent {
    fn default() -> Self {
        Self {
            radius: 0.5,
            height: 2.0,
            direction: CapsuleDirection::Y,
            center: [0.0, 0.0, 0.0],
            is_trigger: false,
            layer: 0,
        }
    }
}

/// Mesh collider component
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MeshColliderComponent {
    /// Path to the mesh asset (uses same mesh as MeshRenderer if empty)
    pub mesh: String,
    /// Whether the mesh is convex (faster, but must be convex hull)
    pub convex: bool,
    /// Whether this is a trigger
    pub is_trigger: bool,
    /// Collision layer
    pub layer: u32,
}

impl Default for MeshColliderComponent {
    fn default() -> Self {
        Self {
            mesh: String::new(),
            convex: true,
            is_trigger: false,
            layer: 0,
        }
    }
}

/// Friction combine mode
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum FrictionCombine {
    /// Average of two values
    #[default]
    Average,
    /// Minimum of two values
    Minimum,
    /// Maximum of two values
    Maximum,
    /// Multiply two values
    Multiply,
}

/// Physics material component for surface properties
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhysicsMaterialComponent {
    /// Dynamic friction (0-1)
    pub dynamic_friction: f32,
    /// Static friction (0-1)
    pub static_friction: f32,
    /// Bounciness/restitution (0-1)
    pub bounciness: f32,
    /// How friction values are combined
    pub friction_combine: FrictionCombine,
    /// How bounciness values are combined
    pub bounce_combine: FrictionCombine,
}

impl Default for PhysicsMaterialComponent {
    fn default() -> Self {
        Self {
            dynamic_friction: 0.6,
            static_friction: 0.6,
            bounciness: 0.0,
            friction_combine: FrictionCombine::Average,
            bounce_combine: FrictionCombine::Average,
        }
    }
}

/// Audio source component for playing sounds
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioSourceComponent {
    /// Path to the audio clip
    pub clip: String,
    /// Volume (0-1)
    pub volume: f32,
    /// Pitch multiplier
    pub pitch: f32,
    /// Whether to loop
    pub loop_audio: bool,
    /// Whether to play on awake
    pub play_on_awake: bool,
    /// Whether to use 3D spatial audio
    pub spatial: bool,
    /// Min distance for 3D audio
    pub min_distance: f32,
    /// Max distance for 3D audio
    pub max_distance: f32,
}

impl Default for AudioSourceComponent {
    fn default() -> Self {
        Self {
            clip: String::new(),
            volume: 1.0,
            pitch: 1.0,
            loop_audio: false,
            play_on_awake: false,
            spatial: true,
            min_distance: 1.0,
            max_distance: 50.0,
        }
    }
}

/// Script component for attaching gameplay logic
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScriptComponent {
    /// Path to the script asset
    pub script: String,
    /// Enabled state
    pub enabled: bool,
}

impl Default for ScriptComponent {
    fn default() -> Self {
        Self {
            script: String::new(),
            enabled: true,
        }
    }
}

// ============================================================================
// Component Registry
// ============================================================================

/// Get all available component types for the add component menu
#[allow(dead_code)] // Intentionally kept for API completeness
pub fn get_component_registry() -> Vec<ComponentInfo> {
    vec![
        ComponentInfo {
            type_id: "MeshRenderer",
            display_name: "Mesh Renderer",
            category: "Rendering",
            description: "Renders a 3D mesh with a material",
            create_default: || Component::MeshRenderer(MeshRendererComponent::default()),
        },
        ComponentInfo {
            type_id: "Light",
            display_name: "Light",
            category: "Rendering",
            description: "Illuminates the scene",
            create_default: || Component::Light(LightComponent::default()),
        },
        ComponentInfo {
            type_id: "Camera",
            display_name: "Camera",
            category: "Rendering",
            description: "Renders the scene from this viewpoint",
            create_default: || Component::Camera(CameraComponent::default()),
        },
        ComponentInfo {
            type_id: "Rigidbody",
            display_name: "Rigidbody",
            category: "Physics",
            description: "Enables physics simulation",
            create_default: || Component::Rigidbody(RigidbodyComponent::default()),
        },
        ComponentInfo {
            type_id: "BoxCollider",
            display_name: "Box Collider",
            category: "Physics",
            description: "Box-shaped collision volume",
            create_default: || Component::BoxCollider(BoxColliderComponent::default()),
        },
        ComponentInfo {
            type_id: "SphereCollider",
            display_name: "Sphere Collider",
            category: "Physics",
            description: "Sphere-shaped collision volume",
            create_default: || Component::SphereCollider(SphereColliderComponent::default()),
        },
        ComponentInfo {
            type_id: "CapsuleCollider",
            display_name: "Capsule Collider",
            category: "Physics",
            description: "Capsule-shaped collision volume (good for characters)",
            create_default: || Component::CapsuleCollider(CapsuleColliderComponent::default()),
        },
        ComponentInfo {
            type_id: "MeshCollider",
            display_name: "Mesh Collider",
            category: "Physics",
            description: "Collision shape from mesh geometry",
            create_default: || Component::MeshCollider(MeshColliderComponent::default()),
        },
        ComponentInfo {
            type_id: "PhysicsMaterial",
            display_name: "Physics Material",
            category: "Physics",
            description: "Surface friction and bounciness properties",
            create_default: || Component::PhysicsMaterial(PhysicsMaterialComponent::default()),
        },
        ComponentInfo {
            type_id: "AudioSource",
            display_name: "Audio Source",
            category: "Audio",
            description: "Plays audio clips",
            create_default: || Component::AudioSource(AudioSourceComponent::default()),
        },
        ComponentInfo {
            type_id: "Script",
            display_name: "Script",
            category: "Scripting",
            description: "Attaches gameplay logic",
            create_default: || Component::Script(ScriptComponent::default()),
        },
    ]
}

/// Get components grouped by category
#[allow(dead_code)] // Intentionally kept for API completeness
pub fn get_components_by_category() -> Vec<(&'static str, Vec<ComponentInfo>)> {
    let registry = get_component_registry();
    let mut categories: std::collections::HashMap<&'static str, Vec<ComponentInfo>> =
        std::collections::HashMap::new();

    for info in registry {
        categories
            .entry(info.category)
            .or_default()
            .push(info);
    }

    // Return in a specific order
    let order = ["Rendering", "Physics", "Audio", "Scripting"];
    let mut result = Vec::new();

    for cat in order {
        if let Some(components) = categories.remove(cat) {
            result.push((cat, components));
        }
    }

    // Add any remaining categories
    for (cat, components) in categories {
        result.push((cat, components));
    }

    result
}
