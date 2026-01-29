// SPDX-License-Identifier: MIT OR Apache-2.0
//! Physics simulation system for play mode.
//!
//! This module provides a simple physics simulation including:
//! - Rigidbody dynamics (gravity, velocity, forces)
//! - Collision detection (sphere-sphere, box-box, sphere-box)
//! - Collision response with friction and bounciness
//! - Constraint solving (position and rotation locks)


use crate::components::{
    BoxColliderComponent, CapsuleColliderComponent, Component,
    PhysicsMaterialComponent, RigidbodyComponent, RigidbodyType, SphereColliderComponent,
};
use crate::state::{EntityId, SceneData, Transform};
use std::collections::HashMap;

/// 3D Vector operations
#[derive(Debug, Clone, Copy, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn zero() -> Self {
        Self::default()
    }

    pub fn from_array(arr: [f32; 3]) -> Self {
        Self { x: arr[0], y: arr[1], z: arr[2] }
    }

    pub fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn length_squared(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 0.0001 {
            Self {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        } else {
            *self
        }
    }

    pub fn dot(&self, other: &Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(&self, other: &Vec3) -> Vec3 {
        Vec3 {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }
}

impl std::ops::Add for Vec3 {
    type Output = Vec3;
    fn add(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, rhs: Vec3) -> Vec3 {
        Vec3 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, rhs: f32) -> Vec3 {
        Vec3 {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

impl std::ops::Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3 {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

/// Physics body state during simulation
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone)]
pub struct PhysicsBody {
    /// Entity this body belongs to
    pub entity_id: EntityId,
    /// Body type
    pub body_type: RigidbodyType,
    /// Current position
    pub position: Vec3,
    /// Current rotation (euler angles in degrees)
    pub rotation: Vec3,
    /// Linear velocity
    pub velocity: Vec3,
    /// Angular velocity (degrees/sec)
    pub angular_velocity: Vec3,
    /// Mass
    pub mass: f32,
    /// Inverse mass (0 for static/kinematic)
    pub inv_mass: f32,
    /// Linear drag
    pub drag: f32,
    /// Angular drag
    pub angular_drag: f32,
    /// Use gravity
    pub use_gravity: bool,
    /// Position constraints
    pub freeze_position: [bool; 3],
    /// Rotation constraints
    pub freeze_rotation: [bool; 3],
    /// Accumulated force for this frame
    pub force: Vec3,
    /// Accumulated torque for this frame
    pub torque: Vec3,
}

impl PhysicsBody {
    pub fn from_entity(
        entity_id: EntityId,
        transform: &Transform,
        rigidbody: &RigidbodyComponent,
    ) -> Self {
        let inv_mass = match rigidbody.body_type {
            RigidbodyType::Dynamic => 1.0 / rigidbody.mass.max(0.001),
            RigidbodyType::Kinematic | RigidbodyType::Static => 0.0,
        };

        Self {
            entity_id,
            body_type: rigidbody.body_type,
            position: Vec3::from_array(transform.position),
            rotation: Vec3::from_array(transform.rotation),
            velocity: Vec3::from_array(rigidbody.initial_velocity),
            angular_velocity: Vec3::from_array(rigidbody.initial_angular_velocity),
            mass: rigidbody.mass,
            inv_mass,
            drag: rigidbody.drag,
            angular_drag: rigidbody.angular_drag,
            use_gravity: rigidbody.use_gravity,
            freeze_position: [
                rigidbody.constraints.freeze_position_x,
                rigidbody.constraints.freeze_position_y,
                rigidbody.constraints.freeze_position_z,
            ],
            freeze_rotation: [
                rigidbody.constraints.freeze_rotation_x,
                rigidbody.constraints.freeze_rotation_y,
                rigidbody.constraints.freeze_rotation_z,
            ],
            force: Vec3::zero(),
            torque: Vec3::zero(),
        }
    }

    /// Apply a force at the center of mass
    pub fn add_force(&mut self, force: Vec3) {
        self.force = self.force + force;
    }

    /// Apply a torque
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn add_torque(&mut self, torque: Vec3) {
        self.torque = self.torque + torque;
    }

    /// Clear accumulated forces
    pub fn clear_forces(&mut self) {
        self.force = Vec3::zero();
        self.torque = Vec3::zero();
    }

    /// Check if this body can move
    pub fn is_dynamic(&self) -> bool {
        matches!(self.body_type, RigidbodyType::Dynamic)
    }
}

/// Collider shape for collision detection
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone)]
pub enum ColliderShape {
    Sphere { radius: f32, center: Vec3 },
    Box { size: Vec3, center: Vec3 },
    Capsule { radius: f32, height: f32, center: Vec3, direction: u8 },
}

/// Physics collider
#[derive(Debug, Clone)]
pub struct PhysicsCollider {
    /// Entity this collider belongs to
    pub entity_id: EntityId,
    /// Shape of the collider
    pub shape: ColliderShape,
    /// Is this a trigger (no physics response)
    pub is_trigger: bool,
    /// Collision layer
    pub layer: u32,
    /// Physics material properties
    pub friction: f32,
    pub bounciness: f32,
}

impl PhysicsCollider {
    pub fn from_box(entity_id: EntityId, bc: &BoxColliderComponent, material: Option<&PhysicsMaterialComponent>) -> Self {
        let (friction, bounciness) = material
            .map(|m| (m.dynamic_friction, m.bounciness))
            .unwrap_or((0.6, 0.0));

        Self {
            entity_id,
            shape: ColliderShape::Box {
                size: Vec3::from_array(bc.size),
                center: Vec3::from_array(bc.center),
            },
            is_trigger: bc.is_trigger,
            layer: bc.layer,
            friction,
            bounciness,
        }
    }

    pub fn from_sphere(entity_id: EntityId, sc: &SphereColliderComponent, material: Option<&PhysicsMaterialComponent>) -> Self {
        let (friction, bounciness) = material
            .map(|m| (m.dynamic_friction, m.bounciness))
            .unwrap_or((0.6, 0.0));

        Self {
            entity_id,
            shape: ColliderShape::Sphere {
                radius: sc.radius,
                center: Vec3::from_array(sc.center),
            },
            is_trigger: sc.is_trigger,
            layer: sc.layer,
            friction,
            bounciness,
        }
    }

    pub fn from_capsule(entity_id: EntityId, cc: &CapsuleColliderComponent, material: Option<&PhysicsMaterialComponent>) -> Self {
        let (friction, bounciness) = material
            .map(|m| (m.dynamic_friction, m.bounciness))
            .unwrap_or((0.6, 0.0));

        use crate::components::CapsuleDirection;
        let direction = match cc.direction {
            CapsuleDirection::X => 0,
            CapsuleDirection::Y => 1,
            CapsuleDirection::Z => 2,
        };

        Self {
            entity_id,
            shape: ColliderShape::Capsule {
                radius: cc.radius,
                height: cc.height,
                center: Vec3::from_array(cc.center),
                direction,
            },
            is_trigger: cc.is_trigger,
            layer: cc.layer,
            friction,
            bounciness,
        }
    }
}

/// Contact point from collision detection
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone)]
pub struct Contact {
    /// Entity A
    pub entity_a: EntityId,
    /// Entity B
    pub entity_b: EntityId,
    /// Contact point in world space
    pub point: Vec3,
    /// Contact normal (from A to B)
    pub normal: Vec3,
    /// Penetration depth
    pub depth: f32,
    /// Combined friction
    pub friction: f32,
    /// Combined bounciness
    pub bounciness: f32,
}

/// Collision layer mask configuration
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone)]
pub struct CollisionLayers {
    /// Layer names
    pub names: Vec<String>,
    /// Collision matrix (layers[i] collides with layers[j])
    pub matrix: Vec<Vec<bool>>,
}

impl Default for CollisionLayers {
    fn default() -> Self {
        Self::new(8)
    }
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl CollisionLayers {
    pub fn new(num_layers: usize) -> Self {
        let mut names = Vec::with_capacity(num_layers);
        for i in 0..num_layers {
            names.push(format!("Layer {}", i));
        }

        // By default, all layers collide with all layers
        let matrix = vec![vec![true; num_layers]; num_layers];

        Self { names, matrix }
    }

    pub fn should_collide(&self, layer_a: u32, layer_b: u32) -> bool {
        let a = layer_a as usize;
        let b = layer_b as usize;
        if a < self.matrix.len() && b < self.matrix.len() {
            self.matrix[a][b]
        } else {
            true // Default to colliding if layer out of range
        }
    }
}

/// Physics world managing the simulation
#[allow(dead_code)] // Intentionally kept for API completeness
pub struct PhysicsWorld {
    /// Gravity vector
    pub gravity: Vec3,
    /// Fixed timestep for physics
    pub fixed_timestep: f32,
    /// All physics bodies
    pub bodies: HashMap<EntityId, PhysicsBody>,
    /// All colliders
    pub colliders: HashMap<EntityId, Vec<PhysicsCollider>>,
    /// Collision layer configuration
    pub collision_layers: CollisionLayers,
    /// Current contacts
    pub contacts: Vec<Contact>,
    /// Trigger enter events this frame
    pub trigger_enters: Vec<(EntityId, EntityId)>,
    /// Trigger exit events this frame
    pub trigger_exits: Vec<(EntityId, EntityId)>,
    /// Currently overlapping triggers
    active_triggers: std::collections::HashSet<(EntityId, EntityId)>,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl PhysicsWorld {
    pub fn new() -> Self {
        Self {
            gravity: Vec3::new(0.0, -9.81, 0.0),
            fixed_timestep: 1.0 / 60.0,
            bodies: HashMap::new(),
            colliders: HashMap::new(),
            collision_layers: CollisionLayers::default(),
            contacts: Vec::new(),
            trigger_enters: Vec::new(),
            trigger_exits: Vec::new(),
            active_triggers: std::collections::HashSet::new(),
        }
    }

    /// Initialize the physics world from scene data
    pub fn initialize_from_scene(&mut self, scene: &SceneData, gravity: [f32; 3]) {
        self.clear();
        self.gravity = Vec3::from_array(gravity);
        self.load_entities(scene);
    }

    /// Initialize the physics world with collision layer settings
    pub fn initialize_with_settings(
        &mut self,
        scene: &SceneData,
        gravity: [f32; 3],
        collision_layers: &crate::project::CollisionLayerSettings,
    ) {
        self.clear();
        self.gravity = Vec3::from_array(gravity);

        // Copy collision layer settings
        self.collision_layers = CollisionLayers {
            names: collision_layers.layer_names.clone(),
            matrix: collision_layers.layer_matrix.clone(),
        };

        self.load_entities(scene);
    }

    /// Load physics entities from scene data
    fn load_entities(&mut self, scene: &SceneData) {
        for (entity_id, entity_data) in scene.entities.iter() {
            // Find rigidbody component
            let rigidbody = entity_data.components.iter().find_map(|c| {
                if let Component::Rigidbody(rb) = c {
                    Some(rb)
                } else {
                    None
                }
            });

            // Create physics body if rigidbody exists
            if let Some(rb) = rigidbody {
                let body = PhysicsBody::from_entity(*entity_id, &entity_data.transform, rb);
                self.bodies.insert(*entity_id, body);
            }

            // Find physics material
            let physics_material = entity_data.components.iter().find_map(|c| {
                if let Component::PhysicsMaterial(pm) = c {
                    Some(pm)
                } else {
                    None
                }
            });

            // Create colliders
            let mut entity_colliders = Vec::new();

            for component in &entity_data.components {
                match component {
                    Component::BoxCollider(bc) => {
                        entity_colliders.push(PhysicsCollider::from_box(*entity_id, bc, physics_material));
                    }
                    Component::SphereCollider(sc) => {
                        entity_colliders.push(PhysicsCollider::from_sphere(*entity_id, sc, physics_material));
                    }
                    Component::CapsuleCollider(cc) => {
                        entity_colliders.push(PhysicsCollider::from_capsule(*entity_id, cc, physics_material));
                    }
                    _ => {}
                }
            }

            if !entity_colliders.is_empty() {
                self.colliders.insert(*entity_id, entity_colliders);
            }
        }

        tracing::info!(
            "Physics world initialized: {} bodies, {} colliders",
            self.bodies.len(),
            self.colliders.len()
        );
    }

    /// Clear all physics state
    pub fn clear(&mut self) {
        self.bodies.clear();
        self.colliders.clear();
        self.contacts.clear();
        self.trigger_enters.clear();
        self.trigger_exits.clear();
        self.active_triggers.clear();
    }

    /// Step the physics simulation
    pub fn step(&mut self, dt: f32) {
        // Clear per-frame data
        self.contacts.clear();
        self.trigger_enters.clear();
        self.trigger_exits.clear();

        // Apply gravity and integrate forces
        self.integrate_forces(dt);

        // Detect collisions
        self.detect_collisions();

        // Resolve collisions
        self.resolve_collisions();

        // Integrate velocities to positions
        self.integrate_velocities(dt);

        // Apply constraints
        self.apply_constraints();

        // Clear accumulated forces
        for body in self.bodies.values_mut() {
            body.clear_forces();
        }
    }

    fn integrate_forces(&mut self, dt: f32) {
        for body in self.bodies.values_mut() {
            if !body.is_dynamic() {
                continue;
            }

            // Apply gravity
            if body.use_gravity {
                let gravity_force = self.gravity * body.mass;
                body.add_force(gravity_force);
            }

            // Apply forces to velocity
            let acceleration = body.force * body.inv_mass;
            body.velocity = body.velocity + acceleration * dt;

            // Apply angular acceleration
            // Simplified: assuming uniform mass distribution
            let angular_acceleration = body.torque * body.inv_mass;
            body.angular_velocity = body.angular_velocity + angular_acceleration * dt;

            // Apply drag
            body.velocity = body.velocity * (1.0 - body.drag * dt).max(0.0);
            body.angular_velocity = body.angular_velocity * (1.0 - body.angular_drag * dt).max(0.0);
        }
    }

    fn integrate_velocities(&mut self, dt: f32) {
        for body in self.bodies.values_mut() {
            if !body.is_dynamic() {
                continue;
            }

            // Update position
            body.position = body.position + body.velocity * dt;

            // Update rotation
            body.rotation = body.rotation + body.angular_velocity * dt;

            // Normalize rotation to 0-360
            body.rotation.x %= 360.0;
            body.rotation.y %= 360.0;
            body.rotation.z %= 360.0;
        }
    }

    fn apply_constraints(&mut self) {
        for body in self.bodies.values_mut() {
            // Apply position freeze
            if body.freeze_position[0] {
                body.velocity.x = 0.0;
            }
            if body.freeze_position[1] {
                body.velocity.y = 0.0;
            }
            if body.freeze_position[2] {
                body.velocity.z = 0.0;
            }

            // Apply rotation freeze
            if body.freeze_rotation[0] {
                body.angular_velocity.x = 0.0;
            }
            if body.freeze_rotation[1] {
                body.angular_velocity.y = 0.0;
            }
            if body.freeze_rotation[2] {
                body.angular_velocity.z = 0.0;
            }
        }
    }

    fn detect_collisions(&mut self) {
        let entity_ids: Vec<EntityId> = self.colliders.keys().copied().collect();

        for i in 0..entity_ids.len() {
            for j in (i + 1)..entity_ids.len() {
                let id_a = entity_ids[i];
                let id_b = entity_ids[j];

                let colliders_a = self.colliders.get(&id_a).unwrap();
                let colliders_b = self.colliders.get(&id_b).unwrap();

                let pos_a = self.bodies.get(&id_a).map(|b| b.position).unwrap_or_default();
                let pos_b = self.bodies.get(&id_b).map(|b| b.position).unwrap_or_default();

                for col_a in colliders_a {
                    for col_b in colliders_b {
                        // Check collision layers
                        if !self.collision_layers.should_collide(col_a.layer, col_b.layer) {
                            continue;
                        }

                        // Test collision based on shape types
                        if let Some(contact) = self.test_collision(col_a, pos_a, col_b, pos_b) {
                            // Handle triggers
                            if col_a.is_trigger || col_b.is_trigger {
                                let pair = if id_a.0 < id_b.0 { (id_a, id_b) } else { (id_b, id_a) };
                                if !self.active_triggers.contains(&pair) {
                                    self.trigger_enters.push(pair);
                                    self.active_triggers.insert(pair);
                                }
                            } else {
                                self.contacts.push(contact);
                            }
                        }
                    }
                }
            }
        }
    }

    fn test_collision(
        &self,
        col_a: &PhysicsCollider,
        pos_a: Vec3,
        col_b: &PhysicsCollider,
        pos_b: Vec3,
    ) -> Option<Contact> {
        match (&col_a.shape, &col_b.shape) {
            (ColliderShape::Sphere { radius: r1, center: c1 }, ColliderShape::Sphere { radius: r2, center: c2 }) => {
                self.test_sphere_sphere(col_a.entity_id, pos_a, *c1, *r1, col_b.entity_id, pos_b, *c2, *r2, col_a, col_b)
            }
            (ColliderShape::Sphere { radius, center }, ColliderShape::Box { size, center: box_center }) => {
                self.test_sphere_box(col_a.entity_id, pos_a + *center, *radius, col_b.entity_id, pos_b + *box_center, *size, col_a, col_b)
            }
            (ColliderShape::Box { size, center: box_center }, ColliderShape::Sphere { radius, center }) => {
                // Swap order and negate normal
                self.test_sphere_box(col_b.entity_id, pos_b + *center, *radius, col_a.entity_id, pos_a + *box_center, *size, col_b, col_a)
                    .map(|mut c| {
                        std::mem::swap(&mut c.entity_a, &mut c.entity_b);
                        c.normal = -c.normal;
                        c
                    })
            }
            (ColliderShape::Box { size: s1, center: c1 }, ColliderShape::Box { size: s2, center: c2 }) => {
                self.test_box_box(col_a.entity_id, pos_a + *c1, *s1, col_b.entity_id, pos_b + *c2, *s2, col_a, col_b)
            }
            // Simplified: treat capsule as sphere for now
            (ColliderShape::Capsule { radius, center, .. }, ColliderShape::Sphere { radius: r2, center: c2 }) => {
                self.test_sphere_sphere(col_a.entity_id, pos_a, *center, *radius, col_b.entity_id, pos_b, *c2, *r2, col_a, col_b)
            }
            (ColliderShape::Sphere { radius: r1, center: c1 }, ColliderShape::Capsule { radius, center, .. }) => {
                self.test_sphere_sphere(col_a.entity_id, pos_a, *c1, *r1, col_b.entity_id, pos_b, *center, *radius, col_a, col_b)
            }
            _ => None, // Other combinations not yet implemented
        }
    }

    fn test_sphere_sphere(
        &self,
        id_a: EntityId, pos_a: Vec3, center_a: Vec3, radius_a: f32,
        id_b: EntityId, pos_b: Vec3, center_b: Vec3, radius_b: f32,
        col_a: &PhysicsCollider, col_b: &PhysicsCollider,
    ) -> Option<Contact> {
        let world_a = pos_a + center_a;
        let world_b = pos_b + center_b;
        let diff = world_b - world_a;
        let dist_sq = diff.length_squared();
        let min_dist = radius_a + radius_b;

        if dist_sq < min_dist * min_dist {
            let dist = dist_sq.sqrt();
            let normal = if dist > 0.0001 {
                diff * (1.0 / dist)
            } else {
                Vec3::new(0.0, 1.0, 0.0)
            };

            Some(Contact {
                entity_a: id_a,
                entity_b: id_b,
                point: world_a + normal * radius_a,
                normal,
                depth: min_dist - dist,
                friction: (col_a.friction + col_b.friction) * 0.5,
                bounciness: (col_a.bounciness + col_b.bounciness) * 0.5,
            })
        } else {
            None
        }
    }

    fn test_sphere_box(
        &self,
        sphere_id: EntityId, sphere_pos: Vec3, sphere_radius: f32,
        box_id: EntityId, box_pos: Vec3, box_size: Vec3,
        col_a: &PhysicsCollider, col_b: &PhysicsCollider,
    ) -> Option<Contact> {
        // Find closest point on box to sphere center
        let half_size = box_size * 0.5;
        let local_sphere = sphere_pos - box_pos;

        let closest = Vec3 {
            x: local_sphere.x.clamp(-half_size.x, half_size.x),
            y: local_sphere.y.clamp(-half_size.y, half_size.y),
            z: local_sphere.z.clamp(-half_size.z, half_size.z),
        };

        let diff = local_sphere - closest;
        let dist_sq = diff.length_squared();

        if dist_sq < sphere_radius * sphere_radius {
            let dist = dist_sq.sqrt();
            let normal = if dist > 0.0001 {
                diff * (1.0 / dist)
            } else {
                Vec3::new(0.0, 1.0, 0.0)
            };

            Some(Contact {
                entity_a: sphere_id,
                entity_b: box_id,
                point: box_pos + closest,
                normal,
                depth: sphere_radius - dist,
                friction: (col_a.friction + col_b.friction) * 0.5,
                bounciness: (col_a.bounciness + col_b.bounciness) * 0.5,
            })
        } else {
            None
        }
    }

    fn test_box_box(
        &self,
        id_a: EntityId, pos_a: Vec3, size_a: Vec3,
        id_b: EntityId, pos_b: Vec3, size_b: Vec3,
        col_a: &PhysicsCollider, col_b: &PhysicsCollider,
    ) -> Option<Contact> {
        // AABB collision test
        let half_a = size_a * 0.5;
        let half_b = size_b * 0.5;

        let min_a = pos_a - half_a;
        let max_a = pos_a + half_a;
        let min_b = pos_b - half_b;
        let max_b = pos_b + half_b;

        // Check overlap on each axis
        if max_a.x < min_b.x || min_a.x > max_b.x { return None; }
        if max_a.y < min_b.y || min_a.y > max_b.y { return None; }
        if max_a.z < min_b.z || min_a.z > max_b.z { return None; }

        // Calculate overlap on each axis
        let overlap_x = (max_a.x.min(max_b.x) - min_a.x.max(min_b.x)).max(0.0);
        let overlap_y = (max_a.y.min(max_b.y) - min_a.y.max(min_b.y)).max(0.0);
        let overlap_z = (max_a.z.min(max_b.z) - min_a.z.max(min_b.z)).max(0.0);

        // Find axis with minimum overlap (separation axis)
        let (depth, normal) = if overlap_x <= overlap_y && overlap_x <= overlap_z {
            let sign = if pos_b.x > pos_a.x { 1.0 } else { -1.0 };
            (overlap_x, Vec3::new(sign, 0.0, 0.0))
        } else if overlap_y <= overlap_z {
            let sign = if pos_b.y > pos_a.y { 1.0 } else { -1.0 };
            (overlap_y, Vec3::new(0.0, sign, 0.0))
        } else {
            let sign = if pos_b.z > pos_a.z { 1.0 } else { -1.0 };
            (overlap_z, Vec3::new(0.0, 0.0, sign))
        };

        let contact_point = (pos_a + pos_b) * 0.5;

        Some(Contact {
            entity_a: id_a,
            entity_b: id_b,
            point: contact_point,
            normal,
            depth,
            friction: (col_a.friction + col_b.friction) * 0.5,
            bounciness: (col_a.bounciness + col_b.bounciness) * 0.5,
        })
    }

    fn resolve_collisions(&mut self) {
        for contact in &self.contacts {
            let body_a = self.bodies.get(&contact.entity_a);
            let body_b = self.bodies.get(&contact.entity_b);

            let inv_mass_a = body_a.map(|b| b.inv_mass).unwrap_or(0.0);
            let inv_mass_b = body_b.map(|b| b.inv_mass).unwrap_or(0.0);
            let total_inv_mass = inv_mass_a + inv_mass_b;

            if total_inv_mass <= 0.0 {
                continue; // Both objects are static
            }

            let vel_a = body_a.map(|b| b.velocity).unwrap_or(Vec3::zero());
            let vel_b = body_b.map(|b| b.velocity).unwrap_or(Vec3::zero());
            let relative_vel = vel_a - vel_b;
            let normal_vel = relative_vel.dot(&contact.normal);

            // Only resolve if objects are moving towards each other
            if normal_vel > 0.0 {
                continue;
            }

            // Calculate impulse magnitude
            let restitution = contact.bounciness;
            let j = -(1.0 + restitution) * normal_vel / total_inv_mass;
            let impulse = contact.normal * j;

            // Apply impulse
            if let Some(body) = self.bodies.get_mut(&contact.entity_a) {
                if body.is_dynamic() {
                    body.velocity = body.velocity + impulse * inv_mass_a;
                }
            }
            if let Some(body) = self.bodies.get_mut(&contact.entity_b) {
                if body.is_dynamic() {
                    body.velocity = body.velocity - impulse * inv_mass_b;
                }
            }

            // Position correction (prevent sinking)
            let correction_amount = contact.depth * 0.8; // Baumgarte stabilization
            let correction = contact.normal * (correction_amount / total_inv_mass);

            if let Some(body) = self.bodies.get_mut(&contact.entity_a) {
                if body.is_dynamic() {
                    body.position = body.position + correction * inv_mass_a;
                }
            }
            if let Some(body) = self.bodies.get_mut(&contact.entity_b) {
                if body.is_dynamic() {
                    body.position = body.position - correction * inv_mass_b;
                }
            }
        }
    }

    /// Apply physics results back to the scene
    pub fn sync_to_scene(&self, scene: &mut SceneData) {
        for (entity_id, body) in &self.bodies {
            if let Some(entity) = scene.get_mut(entity_id) {
                entity.transform.position = body.position.to_array();
                entity.transform.rotation = body.rotation.to_array();
            }
        }
    }

    /// Get the body for an entity
    pub fn get_body(&self, entity_id: EntityId) -> Option<&PhysicsBody> {
        self.bodies.get(&entity_id)
    }

    /// Get mutable body for an entity
    pub fn get_body_mut(&mut self, entity_id: EntityId) -> Option<&mut PhysicsBody> {
        self.bodies.get_mut(&entity_id)
    }

    /// Apply a force to a body
    pub fn apply_force(&mut self, entity_id: EntityId, force: [f32; 3]) {
        if let Some(body) = self.bodies.get_mut(&entity_id) {
            body.add_force(Vec3::from_array(force));
        }
    }

    /// Apply an impulse (immediate velocity change) to a body
    pub fn apply_impulse(&mut self, entity_id: EntityId, impulse: [f32; 3]) {
        if let Some(body) = self.bodies.get_mut(&entity_id) {
            if body.is_dynamic() {
                let impulse_vec = Vec3::from_array(impulse);
                body.velocity = body.velocity + impulse_vec * body.inv_mass;
            }
        }
    }

    /// Set velocity directly
    pub fn set_velocity(&mut self, entity_id: EntityId, velocity: [f32; 3]) {
        if let Some(body) = self.bodies.get_mut(&entity_id) {
            body.velocity = Vec3::from_array(velocity);
        }
    }

    /// Generate debug lines for colliders
    pub fn generate_collider_debug_lines(&self) -> Vec<DebugLine> {
        let mut lines = Vec::new();

        for (entity_id, colliders) in &self.colliders {
            let pos = self.bodies.get(entity_id).map(|b| b.position).unwrap_or_default();

            for collider in colliders {
                let color = if collider.is_trigger {
                    [1.0, 1.0, 0.0, 0.5] // Yellow for triggers
                } else {
                    [0.0, 1.0, 0.0, 0.8] // Green for colliders
                };

                match &collider.shape {
                    ColliderShape::Sphere { radius, center } => {
                        let world_center = pos + *center;
                        // Generate circle approximation
                        let segments = 16;
                        for axis in 0..3 {
                            for i in 0..segments {
                                let angle1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
                                let angle2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;

                                let (p1, p2) = match axis {
                                    0 => (
                                        Vec3::new(0.0, angle1.cos() * radius, angle1.sin() * radius),
                                        Vec3::new(0.0, angle2.cos() * radius, angle2.sin() * radius),
                                    ),
                                    1 => (
                                        Vec3::new(angle1.cos() * radius, 0.0, angle1.sin() * radius),
                                        Vec3::new(angle2.cos() * radius, 0.0, angle2.sin() * radius),
                                    ),
                                    _ => (
                                        Vec3::new(angle1.cos() * radius, angle1.sin() * radius, 0.0),
                                        Vec3::new(angle2.cos() * radius, angle2.sin() * radius, 0.0),
                                    ),
                                };

                                lines.push(DebugLine {
                                    start: (world_center + p1).to_array(),
                                    end: (world_center + p2).to_array(),
                                    color,
                                });
                            }
                        }
                    }
                    ColliderShape::Box { size, center } => {
                        let world_center = pos + *center;
                        let half = *size * 0.5;

                        // 12 edges of a box
                        let corners = [
                            Vec3::new(-half.x, -half.y, -half.z),
                            Vec3::new( half.x, -half.y, -half.z),
                            Vec3::new( half.x,  half.y, -half.z),
                            Vec3::new(-half.x,  half.y, -half.z),
                            Vec3::new(-half.x, -half.y,  half.z),
                            Vec3::new( half.x, -half.y,  half.z),
                            Vec3::new( half.x,  half.y,  half.z),
                            Vec3::new(-half.x,  half.y,  half.z),
                        ];

                        let edges = [
                            (0, 1), (1, 2), (2, 3), (3, 0), // Bottom
                            (4, 5), (5, 6), (6, 7), (7, 4), // Top
                            (0, 4), (1, 5), (2, 6), (3, 7), // Vertical
                        ];

                        for (a, b) in edges {
                            lines.push(DebugLine {
                                start: (world_center + corners[a]).to_array(),
                                end: (world_center + corners[b]).to_array(),
                                color,
                            });
                        }
                    }
                    ColliderShape::Capsule { radius, height, center, direction } => {
                        let world_center = pos + *center;
                        let half_height = (height - radius * 2.0).max(0.0) * 0.5;

                        // Simplified: draw as cylinder + spheres
                        let segments = 12;
                        let axis = match direction {
                            0 => Vec3::new(1.0, 0.0, 0.0),
                            1 => Vec3::new(0.0, 1.0, 0.0),
                            _ => Vec3::new(0.0, 0.0, 1.0),
                        };

                        let top = world_center + axis * half_height;
                        let bottom = world_center - axis * half_height;

                        // Draw circles at top and bottom
                        for i in 0..segments {
                            let angle1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
                            let angle2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;

                            let (offset1, offset2) = match direction {
                                0 => (
                                    Vec3::new(0.0, angle1.cos() * radius, angle1.sin() * radius),
                                    Vec3::new(0.0, angle2.cos() * radius, angle2.sin() * radius),
                                ),
                                1 => (
                                    Vec3::new(angle1.cos() * radius, 0.0, angle1.sin() * radius),
                                    Vec3::new(angle2.cos() * radius, 0.0, angle2.sin() * radius),
                                ),
                                _ => (
                                    Vec3::new(angle1.cos() * radius, angle1.sin() * radius, 0.0),
                                    Vec3::new(angle2.cos() * radius, angle2.sin() * radius, 0.0),
                                ),
                            };

                            // Top circle
                            lines.push(DebugLine {
                                start: (top + offset1).to_array(),
                                end: (top + offset2).to_array(),
                                color,
                            });

                            // Bottom circle
                            lines.push(DebugLine {
                                start: (bottom + offset1).to_array(),
                                end: (bottom + offset2).to_array(),
                                color,
                            });
                        }

                        // Vertical lines
                        for i in 0..4 {
                            let angle = (i as f32 / 4.0) * std::f32::consts::TAU;
                            let offset = match direction {
                                0 => Vec3::new(0.0, angle.cos() * radius, angle.sin() * radius),
                                1 => Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius),
                                _ => Vec3::new(angle.cos() * radius, angle.sin() * radius, 0.0),
                            };

                            lines.push(DebugLine {
                                start: (top + offset).to_array(),
                                end: (bottom + offset).to_array(),
                                color,
                            });
                        }
                    }
                }
            }
        }

        lines
    }

    /// Generate debug lines for velocities
    pub fn generate_velocity_debug_lines(&self) -> Vec<DebugLine> {
        let mut lines = Vec::new();

        for body in self.bodies.values() {
            let vel_length = body.velocity.length();
            if vel_length > 0.01 {
                let end = body.position + body.velocity.normalize() * vel_length.min(5.0);
                lines.push(DebugLine {
                    start: body.position.to_array(),
                    end: end.to_array(),
                    color: [0.0, 0.5, 1.0, 1.0], // Blue
                });
            }
        }

        lines
    }

    /// Generate debug lines for contact points
    pub fn generate_contact_debug_lines(&self) -> Vec<DebugLine> {
        let mut lines = Vec::new();

        for contact in &self.contacts {
            let end = contact.point + contact.normal * 0.5;
            lines.push(DebugLine {
                start: contact.point.to_array(),
                end: end.to_array(),
                color: [1.0, 0.0, 0.0, 1.0], // Red
            });
        }

        lines
    }
}

/// Debug line for visualization
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone, Copy)]
pub struct DebugLine {
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub color: [f32; 4],
}
