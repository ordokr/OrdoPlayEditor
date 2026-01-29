// SPDX-License-Identifier: MIT OR Apache-2.0
//! Inspector panel - Component/property editor.

use crate::components::{
    get_components_by_category, Component, LightType,
};
use crate::state::{EditorState, EntityId, Transform};

/// The inspector panel for editing entity components
pub struct InspectorPanel {
    /// Sections that are expanded
    expanded_sections: std::collections::HashSet<String>,
    /// Temporary edit values (for tracking changes)
    editing_transform: Option<(EntityId, Transform)>,
    /// Start of the current transform edit (for undo)
    editing_transform_start: Option<(EntityId, Transform)>,
    /// Entity name being edited
    editing_name: Option<(EntityId, String)>,
    /// Multi-edit transform buffer (offset values when relative mode is on)
    multi_transform: Transform,
    /// Cached selection for multi-edit
    multi_selection: Vec<EntityId>,
    /// Whether multi-edit uses relative offsets
    multi_edit_relative: bool,
    /// Starting transforms for multi-edit (for undo)
    multi_edit_start_transforms: Vec<(EntityId, Transform)>,
    /// Whether we're currently dragging in multi-edit mode
    multi_edit_dragging: bool,
    /// Add component popup open state
    add_component_popup_open: bool,
    /// Search filter for add component popup
    add_component_search: String,
    /// Property search/filter text
    property_search: String,
}

impl InspectorPanel {
    /// Create a new inspector panel
    pub fn new() -> Self {
        let mut expanded = std::collections::HashSet::new();
        expanded.insert("Transform".to_string());
        expanded.insert("Info".to_string());

        Self {
            expanded_sections: expanded,
            editing_transform: None,
            editing_transform_start: None,
            editing_name: None,
            multi_transform: Transform::default(),
            multi_selection: Vec::new(),
            multi_edit_relative: true, // Default to relative mode
            multi_edit_start_transforms: Vec::new(),
            multi_edit_dragging: false,
            add_component_popup_open: false,
            add_component_search: String::new(),
            property_search: String::new(),
        }
    }

    /// Render the inspector panel
    pub fn ui(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        if state.selection.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No entity selected");
            });
            return;
        }

        let selection_count = state.selection.len();
        if selection_count > 1 {
            ui.label(format!("{} entities selected", selection_count));
            ui.separator();
            self.multi_edit_ui(ui, state);
            return;
        }

        // Single entity selected
        if let Some(entity_id) = state.selection.primary().copied() {
            // Clone entity data for display
            let entity_data = state.scene.get(&entity_id).cloned();

            if let Some(data) = entity_data {
                // Property search box
                ui.horizontal(|ui| {
                    ui.label("Filter:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.property_search)
                            .desired_width(ui.available_width() - 30.0)
                            .hint_text("Search properties..."),
                    );
                    if ui.small_button("X").on_hover_text("Clear filter").clicked() {
                        self.property_search.clear();
                    }
                });

                ui.separator();

                let search_filter = self.property_search.to_lowercase();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Entity header (always show)
                    if search_filter.is_empty() || "name active static".contains(&search_filter) {
                        self.entity_header(ui, state, entity_id, &data.name, data.active, data.is_static);
                        ui.separator();
                    }

                    // Transform component (show if matches filter)
                    if search_filter.is_empty()
                        || "transform position rotation scale".contains(&search_filter)
                        || search_filter.contains("x")
                        || search_filter.contains("y")
                        || search_filter.contains("z")
                    {
                        self.transform_section(ui, state, entity_id, &data.transform);
                    }

                    // Components section
                    self.components_section_filtered(ui, state, entity_id, &search_filter);
                });
            }
        }
    }

    fn entity_header(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut EditorState,
        entity_id: EntityId,
        name: &str,
        mut active: bool,
        mut is_static: bool,
    ) {
        // Check if this entity is part of a prefab instance
        let is_prefab_entity = state.prefab_manager.is_prefab_entity(entity_id);
        let is_prefab_root = state.prefab_manager.is_prefab_root(entity_id);

        // Show prefab indicator
        if is_prefab_entity {
            ui.horizontal(|ui| {
                let indicator_text = if is_prefab_root {
                    "\u{f1b2} Prefab Instance (Root)"
                } else {
                    "\u{f0c1} Prefab Instance (Child)"
                };
                ui.label(
                    egui::RichText::new(indicator_text)
                        .color(egui::Color32::from_rgb(100, 180, 255))
                        .small()
                );

                // Show overrides count
                let overrides = state.get_entity_overrides(entity_id);
                if !overrides.is_empty() {
                    ui.label(
                        egui::RichText::new(format!("({} overrides)", overrides.len()))
                            .color(egui::Color32::from_rgb(255, 180, 80))
                            .small()
                    );
                }
            });

            // Prefab actions
            ui.horizontal(|ui| {
                if ui.small_button("Revert All").on_hover_text("Revert all property changes to prefab values").clicked() {
                    state.revert_all_overrides(entity_id);
                }
                if is_prefab_root
                    && ui.small_button("Unpack").on_hover_text("Remove prefab link").clicked() {
                        state.unpack_prefab(entity_id);
                    }
            });
            ui.add_space(4.0);
        }

        // Initialize editing name if not set
        let edit_name = self.editing_name.get_or_insert_with(|| (entity_id, name.to_string()));

        // Reset if entity changed
        if edit_name.0 != entity_id {
            *edit_name = (entity_id, name.to_string());
        }

        // Check if name is overridden
        let name_overridden = is_prefab_entity && state.is_property_overridden(entity_id, "name");

        ui.horizontal(|ui| {
            let label_text = if name_overridden {
                egui::RichText::new("Entity:").strong().color(egui::Color32::from_rgb(255, 180, 80))
            } else {
                egui::RichText::new("Entity:")
            };
            let label_response = ui.label(label_text);

            if name_overridden {
                label_response.on_hover_text("Overridden from prefab");
            }

            let response = ui.text_edit_singleline(&mut edit_name.1);

            // Commit name change when focus is lost
            if response.lost_focus() && edit_name.1 != name {
                state.set_entity_name(entity_id, edit_name.1.clone());
                // Track override if this is a prefab instance
                if is_prefab_entity {
                    state.track_prefab_override(
                        entity_id,
                        "name",
                        serde_json::Value::String(edit_name.1.clone()),
                    );
                }
            }

            // Context menu for override
            if name_overridden {
                response.context_menu(|ui| {
                    if ui.button("Revert to Prefab Value").clicked() {
                        state.revert_property_override(entity_id, "name");
                        ui.close_menu();
                    }
                });
            }
        });

        // Check if active/static are overridden
        let active_overridden = is_prefab_entity && state.is_property_overridden(entity_id, "active");
        let static_overridden = is_prefab_entity && state.is_property_overridden(entity_id, "is_static");

        ui.horizontal(|ui| {
            let active_text = if active_overridden {
                egui::RichText::new("Active").strong().color(egui::Color32::from_rgb(255, 180, 80))
            } else {
                egui::RichText::new("Active")
            };
            let response = ui.checkbox(&mut active, active_text);
            if response.changed() {
                state.set_entity_active(entity_id, active);
                if is_prefab_entity {
                    state.track_prefab_override(
                        entity_id,
                        "active",
                        serde_json::Value::Bool(active),
                    );
                }
            }
            if active_overridden {
                // Context menu first (consumes response), then hover text
                response.clone().context_menu(|ui| {
                    if ui.button("Revert to Prefab Value").clicked() {
                        state.revert_property_override(entity_id, "active");
                        ui.close_menu();
                    }
                });
                response.on_hover_text("Overridden from prefab (right-click to revert)");
            }

            let static_text = if static_overridden {
                egui::RichText::new("Static").strong().color(egui::Color32::from_rgb(255, 180, 80))
            } else {
                egui::RichText::new("Static")
            };
            let response = ui.checkbox(&mut is_static, static_text);
            if response.changed() {
                state.set_entity_static(entity_id, is_static);
                if is_prefab_entity {
                    state.track_prefab_override(
                        entity_id,
                        "is_static",
                        serde_json::Value::Bool(is_static),
                    );
                }
            }
            if static_overridden {
                // Context menu first (consumes response), then hover text
                response.clone().context_menu(|ui| {
                    if ui.button("Revert to Prefab Value").clicked() {
                        state.revert_property_override(entity_id, "is_static");
                        ui.close_menu();
                    }
                });
                response.on_hover_text("Overridden from prefab (right-click to revert)");
            }
        });
    }

    fn transform_section(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut EditorState,
        entity_id: EntityId,
        current_transform: &Transform,
    ) {
        let expanded = self.expanded_sections.contains("Transform");

        // Check if entity is part of a prefab instance
        let is_prefab_entity = state.prefab_manager.is_prefab_entity(entity_id);

        // Check which properties are overridden
        let pos_overridden = is_prefab_entity && state.is_property_overridden(entity_id, "transform.position");
        let rot_overridden = is_prefab_entity && state.is_property_overridden(entity_id, "transform.rotation");
        let scale_overridden = is_prefab_entity && state.is_property_overridden(entity_id, "transform.scale");
        let any_overridden = pos_overridden || rot_overridden || scale_overridden;

        // Initialize editing transform if not set
        let edit_transform = self.editing_transform.get_or_insert_with(|| {
            (entity_id, current_transform.clone())
        });

        // Reset if entity changed
        if edit_transform.0 != entity_id {
            *edit_transform = (entity_id, current_transform.clone());
            self.editing_transform_start = None;
        }

        // Header with override indicator
        let header_text = if any_overridden {
            egui::RichText::new("Transform").strong().color(egui::Color32::from_rgb(255, 180, 80))
        } else {
            egui::RichText::new("Transform")
        };

        let header = egui::CollapsingHeader::new(header_text)
            .default_open(expanded)
            .show(ui, |ui| {
                let mut changed = false;
                let mut reset_position = false;
                let mut reset_rotation = false;
                let mut reset_scale = false;

                // Position with right-click context menu
                let pos_response = ui.horizontal(|ui| {
                    ui.label("Position");
                    let response = ui.add(egui::DragValue::new(&mut edit_transform.1.position[0]).speed(0.1).prefix("X: "));
                    if response.changed() {
                        changed = true;
                    }
                    let response = ui.add(egui::DragValue::new(&mut edit_transform.1.position[1]).speed(0.1).prefix("Y: "));
                    if response.changed() {
                        changed = true;
                    }
                    let response = ui.add(egui::DragValue::new(&mut edit_transform.1.position[2]).speed(0.1).prefix("Z: "));
                    if response.changed() {
                        changed = true;
                    }
                }).response;

                pos_response.context_menu(|ui| {
                    if ui.button("Reset Position to [0, 0, 0]").clicked() {
                        reset_position = true;
                        ui.close_menu();
                    }
                });

                // Rotation with right-click context menu
                let rot_response = ui.horizontal(|ui| {
                    ui.label("Rotation");
                    let response = ui.add(egui::DragValue::new(&mut edit_transform.1.rotation[0]).speed(1.0).prefix("X: ").suffix("°"));
                    if response.changed() {
                        changed = true;
                    }
                    let response = ui.add(egui::DragValue::new(&mut edit_transform.1.rotation[1]).speed(1.0).prefix("Y: ").suffix("°"));
                    if response.changed() {
                        changed = true;
                    }
                    let response = ui.add(egui::DragValue::new(&mut edit_transform.1.rotation[2]).speed(1.0).prefix("Z: ").suffix("°"));
                    if response.changed() {
                        changed = true;
                    }
                }).response;

                rot_response.context_menu(|ui| {
                    if ui.button("Reset Rotation to [0, 0, 0]").clicked() {
                        reset_rotation = true;
                        ui.close_menu();
                    }
                });

                // Scale with right-click context menu
                let scale_response = ui.horizontal(|ui| {
                    ui.label("Scale   ");
                    let response = ui.add(egui::DragValue::new(&mut edit_transform.1.scale[0]).speed(0.01).prefix("X: "));
                    if response.changed() {
                        changed = true;
                    }
                    let response = ui.add(egui::DragValue::new(&mut edit_transform.1.scale[1]).speed(0.01).prefix("Y: "));
                    if response.changed() {
                        changed = true;
                    }
                    let response = ui.add(egui::DragValue::new(&mut edit_transform.1.scale[2]).speed(0.01).prefix("Z: "));
                    if response.changed() {
                        changed = true;
                    }
                }).response;

                scale_response.context_menu(|ui| {
                    if ui.button("Reset Scale to [1, 1, 1]").clicked() {
                        reset_scale = true;
                        ui.close_menu();
                    }
                });

                // Reset all button
                ui.horizontal(|ui| {
                    if ui.small_button("Reset Transform").on_hover_text("Reset to default (position=0, rotation=0, scale=1)").clicked() {
                        reset_position = true;
                        reset_rotation = true;
                        reset_scale = true;
                    }
                });

                // Apply resets
                if reset_position || reset_rotation || reset_scale {
                    let before = current_transform.clone();
                    if reset_position {
                        edit_transform.1.position = [0.0, 0.0, 0.0];
                    }
                    if reset_rotation {
                        edit_transform.1.rotation = [0.0, 0.0, 0.0];
                    }
                    if reset_scale {
                        edit_transform.1.scale = [1.0, 1.0, 1.0];
                    }
                    // Commit to undo
                    state.set_transform_with_before(
                        entity_id,
                        before,
                        edit_transform.1.clone(),
                        "Reset transform",
                    );
                }

                // Commit changes when dragging ends
                if changed {
                    if self.editing_transform_start.is_none() {
                        self.editing_transform_start = Some((entity_id, current_transform.clone()));
                    }
                    // Apply live updates for visual feedback
                    if let Some(data) = state.scene.get_mut(&entity_id) {
                        data.transform = edit_transform.1.clone();
                    }
                }

                // Check for drag end to commit to undo history
                let commit = ui.input(|i| i.pointer.any_released() || i.key_pressed(egui::Key::Enter));
                if commit {
                    if let Some((start_id, start_transform)) = self.editing_transform_start.take() {
                        if start_id == entity_id && edit_transform.1 != start_transform {
                            state.set_transform_with_before(
                                entity_id,
                                start_transform,
                                edit_transform.1.clone(),
                                "Transform entity",
                            );
                        }
                    }
                }
            });

        if header.header_response.clicked() {
            if expanded {
                self.expanded_sections.remove("Transform");
            } else {
                self.expanded_sections.insert("Transform".to_string());
            }
        }
    }

    fn components_section_filtered(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut EditorState,
        entity_id: EntityId,
        search_filter: &str,
    ) {
        // Get components for this entity
        let components = state
            .scene
            .get(&entity_id)
            .map(|e| e.components.clone())
            .unwrap_or_default();

        // Track component to remove (deferred to avoid borrow issues)
        let mut remove_index: Option<usize> = None;

        // Render each component (filtered)
        for (index, component) in components.iter().enumerate() {
            let component_name = component.display_name();

            // Check if component matches search filter
            if !search_filter.is_empty() && !self.component_matches_filter(component, search_filter) {
                continue;
            }

            let header_id = format!("component_{}_{}", entity_id.0, index);
            let expanded = self.expanded_sections.contains(&header_id);

            ui.push_id(index, |ui| {
                let header = egui::CollapsingHeader::new(component_name)
                    .default_open(expanded)
                    .show(ui, |ui| {
                        // Component-specific UI
                        // Clone component for mutable editing
                        let mut component_mut = component.clone();
                        let changed = self.draw_component_ui(ui, &mut component_mut);
                        if changed {
                            if let Some(entity) = state.scene.get_mut(&entity_id) {
                                if index < entity.components.len() {
                                    entity.components[index] = component_mut;
                                    state.dirty = true;
                                }
                            }
                        }

                        // Remove button at the bottom
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.small_button("Remove Component").clicked() {
                                remove_index = Some(index);
                            }
                        });
                    });

                if header.header_response.clicked() {
                    if expanded {
                        self.expanded_sections.remove(&header_id);
                    } else {
                        self.expanded_sections.insert(header_id);
                    }
                }
            });
        }

        // Apply deferred remove
        if let Some(index) = remove_index {
            state.remove_component(entity_id, index);
        }

        // Add Component button (always show)
        if search_filter.is_empty() || "add component".contains(search_filter) {
            ui.separator();
            self.add_component_button(ui, state, entity_id);
        }
    }

    fn component_matches_filter(&self, component: &Component, filter: &str) -> bool {
        // Check component name
        if component.display_name().to_lowercase().contains(filter) {
            return true;
        }

        // Check component-specific properties
        match component {
            Component::MeshRenderer(m) => {
                "mesh material shadows cast receive".contains(filter)
                    || m.mesh.to_lowercase().contains(filter)
                    || m.material.to_lowercase().contains(filter)
            }
            Component::Light(_) => {
                "light color intensity range spot directional point".contains(filter)
            }
            Component::Camera(_) => {
                "camera fov near far main clear".contains(filter)
            }
            Component::Rigidbody(_) => {
                "rigidbody mass drag gravity kinematic physics".contains(filter)
            }
            Component::BoxCollider(_) => {
                "box collider size center trigger".contains(filter)
            }
            Component::SphereCollider(_) => {
                "sphere collider radius center trigger".contains(filter)
            }
            Component::CapsuleCollider(_) => {
                "capsule collider radius height direction center trigger".contains(filter)
            }
            Component::MeshCollider(_) => {
                "mesh collider convex trigger".contains(filter)
            }
            Component::PhysicsMaterial(_) => {
                "physics material friction bounciness".contains(filter)
            }
            Component::AudioSource(a) => {
                "audio source clip volume pitch loop spatial".contains(filter)
                    || a.clip.to_lowercase().contains(filter)
            }
            Component::Script(s) => {
                "script enabled".contains(filter)
                    || s.script.to_lowercase().contains(filter)
            }
        }
    }

    fn draw_component_ui(&self, ui: &mut egui::Ui, component: &mut Component) -> bool {
        let mut changed = false;
        match component {
            Component::MeshRenderer(mesh) => {
                ui.horizontal(|ui| {
                    ui.label("Mesh");
                    ui.label(if mesh.mesh.is_empty() { "(None)" } else { &mesh.mesh });
                });
                ui.horizontal(|ui| {
                    ui.label("Material");
                    ui.label(if mesh.material.is_empty() { "(None)" } else { &mesh.material });
                });
                ui.label(format!("Cast Shadows: {}", mesh.cast_shadows));
                ui.label(format!("Receive Shadows: {}", mesh.receive_shadows));
            }
            Component::Light(light) => {
                let type_str = match light.light_type {
                    LightType::Directional => "Directional",
                    LightType::Point => "Point",
                    LightType::Spot => "Spot",
                };
                ui.label(format!("Type: {}", type_str));
                ui.horizontal(|ui| {
                    ui.label("Color");
                    let mut color_bytes = [
                        (light.color[0] * 255.0) as u8,
                        (light.color[1] * 255.0) as u8,
                        (light.color[2] * 255.0) as u8,
                        255u8,
                    ];
                    if ui.color_edit_button_srgba_unmultiplied(&mut color_bytes).changed() {
                        light.color[0] = color_bytes[0] as f32 / 255.0;
                        light.color[1] = color_bytes[1] as f32 / 255.0;
                        light.color[2] = color_bytes[2] as f32 / 255.0;
                        changed = true;
                    }
                });
                ui.label(format!("Intensity: {:.2}", light.intensity));
                ui.label(format!("Range: {:.2}", light.range));
                if matches!(light.light_type, LightType::Spot) {
                    ui.label(format!("Spot Angle: {:.1}°", light.spot_angle));
                }
            }
            Component::Camera(camera) => {
                ui.label(format!("FOV: {:.1}°", camera.fov));
                ui.label(format!("Near: {:.3}", camera.near));
                ui.label(format!("Far: {:.1}", camera.far));
                ui.label(format!("Main Camera: {}", camera.is_main));
            }
            Component::Rigidbody(rb) => {
                use crate::components::RigidbodyType;
                let body_type_str = match rb.body_type {
                    RigidbodyType::Dynamic => "Dynamic",
                    RigidbodyType::Kinematic => "Kinematic",
                    RigidbodyType::Static => "Static",
                };
                ui.label(format!("Body Type: {}", body_type_str));
                ui.label(format!("Mass: {:.2} kg", rb.mass));
                ui.label(format!("Drag: {:.3}", rb.drag));
                ui.label(format!("Angular Drag: {:.3}", rb.angular_drag));
                ui.label(format!("Use Gravity: {}", rb.use_gravity));
            }
            Component::BoxCollider(bc) => {
                ui.label(format!("Size: [{:.2}, {:.2}, {:.2}]", bc.size[0], bc.size[1], bc.size[2]));
                ui.label(format!("Center: [{:.2}, {:.2}, {:.2}]", bc.center[0], bc.center[1], bc.center[2]));
                ui.label(format!("Is Trigger: {}", bc.is_trigger));
                ui.label(format!("Layer: {}", bc.layer));
            }
            Component::SphereCollider(sc) => {
                ui.label(format!("Radius: {:.2}", sc.radius));
                ui.label(format!("Center: [{:.2}, {:.2}, {:.2}]", sc.center[0], sc.center[1], sc.center[2]));
                ui.label(format!("Is Trigger: {}", sc.is_trigger));
                ui.label(format!("Layer: {}", sc.layer));
            }
            Component::CapsuleCollider(cc) => {
                use crate::components::CapsuleDirection;
                let dir_str = match cc.direction {
                    CapsuleDirection::X => "X-Axis",
                    CapsuleDirection::Y => "Y-Axis",
                    CapsuleDirection::Z => "Z-Axis",
                };
                ui.label(format!("Radius: {:.2}", cc.radius));
                ui.label(format!("Height: {:.2}", cc.height));
                ui.label(format!("Direction: {}", dir_str));
                ui.label(format!("Center: [{:.2}, {:.2}, {:.2}]", cc.center[0], cc.center[1], cc.center[2]));
                ui.label(format!("Is Trigger: {}", cc.is_trigger));
                ui.label(format!("Layer: {}", cc.layer));
            }
            Component::MeshCollider(mc) => {
                ui.label(format!("Mesh: {}", if mc.mesh.is_empty() { "(Uses MeshRenderer)" } else { &mc.mesh }));
                ui.label(format!("Convex: {}", mc.convex));
                ui.label(format!("Is Trigger: {}", mc.is_trigger));
                ui.label(format!("Layer: {}", mc.layer));
            }
            Component::PhysicsMaterial(pm) => {
                use crate::components::FrictionCombine;
                let friction_combine_str = match pm.friction_combine {
                    FrictionCombine::Average => "Average",
                    FrictionCombine::Minimum => "Minimum",
                    FrictionCombine::Maximum => "Maximum",
                    FrictionCombine::Multiply => "Multiply",
                };
                let bounce_combine_str = match pm.bounce_combine {
                    FrictionCombine::Average => "Average",
                    FrictionCombine::Minimum => "Minimum",
                    FrictionCombine::Maximum => "Maximum",
                    FrictionCombine::Multiply => "Multiply",
                };
                ui.label(format!("Dynamic Friction: {:.2}", pm.dynamic_friction));
                ui.label(format!("Static Friction: {:.2}", pm.static_friction));
                ui.label(format!("Bounciness: {:.2}", pm.bounciness));
                ui.label(format!("Friction Combine: {}", friction_combine_str));
                ui.label(format!("Bounce Combine: {}", bounce_combine_str));
            }
            Component::AudioSource(audio) => {
                ui.label(format!("Clip: {}", if audio.clip.is_empty() { "(None)" } else { &audio.clip }));
                ui.label(format!("Volume: {:.2}", audio.volume));
                ui.label(format!("Pitch: {:.2}", audio.pitch));
                ui.label(format!("Loop: {}", audio.loop_audio));
                ui.label(format!("Play on Awake: {}", audio.play_on_awake));
                ui.label(format!("Spatial: {}", audio.spatial));
            }
            Component::Script(script) => {
                ui.label(format!("Script: {}", if script.script.is_empty() { "(None)" } else { &script.script }));
                ui.label(format!("Enabled: {}", script.enabled));
            }
        }
        changed
    }

    fn add_component_button(&mut self, ui: &mut egui::Ui, state: &mut EditorState, entity_id: EntityId) {
        let button_response = ui.button("Add Component");

        if button_response.clicked() {
            self.add_component_popup_open = !self.add_component_popup_open;
            self.add_component_search.clear();
        }

        // Show popup below button
        if self.add_component_popup_open {
            let popup_id = ui.make_persistent_id("add_component_popup");

            egui::popup::popup_below_widget(ui, popup_id, &button_response, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                ui.set_min_width(250.0);

                // Search box
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.text_edit_singleline(&mut self.add_component_search);
                });

                ui.separator();

                let search_lower = self.add_component_search.to_lowercase();
                let categories = get_components_by_category();

                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    for (category, components) in categories {
                        // Filter components by search
                        let filtered: Vec<_> = components
                            .iter()
                            .filter(|info| {
                                search_lower.is_empty()
                                    || info.display_name.to_lowercase().contains(&search_lower)
                                    || info.description.to_lowercase().contains(&search_lower)
                            })
                            .collect();

                        if filtered.is_empty() {
                            continue;
                        }

                        ui.label(egui::RichText::new(category).strong());
                        ui.indent(category, |ui| {
                            for info in filtered {
                                // Check if already has this component
                                let has_component = state.has_component(entity_id, info.type_id);

                                let button = egui::Button::new(info.display_name);
                                let response = ui.add_enabled(!has_component, button);

                                let hover_text = if has_component {
                                    "Already attached"
                                } else {
                                    info.description
                                };

                                if response.clicked() {
                                    let component = (info.create_default)();
                                    state.add_component(entity_id, component);
                                    self.add_component_popup_open = false;
                                }

                                response.on_hover_text(hover_text);
                            }
                        });
                    }
                });
            });

            // Keep popup open
            if self.add_component_popup_open {
                ui.memory_mut(|mem| mem.open_popup(popup_id));
            }
        }
    }

    fn multi_edit_ui(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        self.sync_multi_transform(state);

        let all_active = state.selection.entities.iter().all(|id| {
            state.scene.get(id).map(|e| e.active).unwrap_or(false)
        });
        let all_inactive = state.selection.entities.iter().all(|id| {
            state.scene.get(id).map(|e| !e.active).unwrap_or(false)
        });
        let all_static = state.selection.entities.iter().all(|id| {
            state.scene.get(id).map(|e| e.is_static).unwrap_or(false)
        });
        let all_dynamic = state.selection.entities.iter().all(|id| {
            state.scene.get(id).map(|e| !e.is_static).unwrap_or(false)
        });

        ui.horizontal(|ui| {
            ui.label("Active:");
            if ui.add_enabled(!all_active, egui::Button::new("Activate All")).clicked() {
                self.set_active_for_selection(state, true);
            }
            if ui.add_enabled(!all_inactive, egui::Button::new("Deactivate All")).clicked() {
                self.set_active_for_selection(state, false);
            }
        });

        ui.horizontal(|ui| {
            ui.label("Static:");
            if ui.add_enabled(!all_static, egui::Button::new("Set Static")).clicked() {
                self.set_static_for_selection(state, true);
            }
            if ui.add_enabled(!all_dynamic, egui::Button::new("Set Dynamic")).clicked() {
                self.set_static_for_selection(state, false);
            }
        });

        ui.separator();

        // Mode toggle for relative vs absolute editing
        ui.horizontal(|ui| {
            ui.label("Edit Mode:");
            if ui.selectable_label(self.multi_edit_relative, "Relative").clicked()
                && !self.multi_edit_relative {
                    self.multi_edit_relative = true;
                    // Reset offset values when switching to relative mode
                    self.multi_transform = Transform {
                        position: [0.0, 0.0, 0.0],
                        rotation: [0.0, 0.0, 0.0],
                        scale: [1.0, 1.0, 1.0],
                    };
                }
            if ui.selectable_label(!self.multi_edit_relative, "Absolute").clicked()
                && self.multi_edit_relative {
                    self.multi_edit_relative = false;
                    // Sync to average values when switching to absolute mode
                    self.force_sync_multi_transform(state);
                }
            ui.add_space(10.0);
            ui.label(if self.multi_edit_relative {
                "(+/- offset)"
            } else {
                "(set all to same)"
            }).on_hover_text(if self.multi_edit_relative {
                "Relative mode: Add/subtract values from each entity's current transform"
            } else {
                "Absolute mode: Set all entities to the same transform values"
            });
        });

        let header = egui::CollapsingHeader::new("Transform (All)")
            .default_open(true)
            .show(ui, |ui| {
                let mut changed = false;

                // Position
                ui.horizontal(|ui| {
                    ui.label(if self.multi_edit_relative { "Position +" } else { "Position" });
                    if ui.add(egui::DragValue::new(&mut self.multi_transform.position[0]).speed(0.1).prefix("X: ")).changed() {
                        changed = true;
                    }
                    if ui.add(egui::DragValue::new(&mut self.multi_transform.position[1]).speed(0.1).prefix("Y: ")).changed() {
                        changed = true;
                    }
                    if ui.add(egui::DragValue::new(&mut self.multi_transform.position[2]).speed(0.1).prefix("Z: ")).changed() {
                        changed = true;
                    }
                });

                // Rotation
                ui.horizontal(|ui| {
                    ui.label(if self.multi_edit_relative { "Rotation +" } else { "Rotation" });
                    if ui.add(egui::DragValue::new(&mut self.multi_transform.rotation[0]).speed(1.0).prefix("X: ").suffix("°")).changed() {
                        changed = true;
                    }
                    if ui.add(egui::DragValue::new(&mut self.multi_transform.rotation[1]).speed(1.0).prefix("Y: ").suffix("°")).changed() {
                        changed = true;
                    }
                    if ui.add(egui::DragValue::new(&mut self.multi_transform.rotation[2]).speed(1.0).prefix("Z: ").suffix("°")).changed() {
                        changed = true;
                    }
                });

                // Scale
                ui.horizontal(|ui| {
                    ui.label(if self.multi_edit_relative { "Scale   *" } else { "Scale   " });
                    if ui.add(egui::DragValue::new(&mut self.multi_transform.scale[0]).speed(0.01).prefix("X: ")).changed() {
                        changed = true;
                    }
                    if ui.add(egui::DragValue::new(&mut self.multi_transform.scale[1]).speed(0.01).prefix("Y: ")).changed() {
                        changed = true;
                    }
                    if ui.add(egui::DragValue::new(&mut self.multi_transform.scale[2]).speed(0.01).prefix("Z: ")).changed() {
                        changed = true;
                    }
                });

                // Handle live preview during dragging
                if changed {
                    if !self.multi_edit_dragging {
                        // Start of drag - save initial transforms for undo
                        self.multi_edit_dragging = true;
                        self.multi_edit_start_transforms = state.selection.entities.iter()
                            .filter_map(|id| state.scene.get(id).map(|e| (*id, e.transform.clone())))
                            .collect();
                    }
                    // Apply live preview
                    self.apply_transform_live(state);
                }

                // Check for drag end to commit to undo history
                let commit = ui.input(|i| i.pointer.any_released() || i.key_pressed(egui::Key::Enter));
                if commit && self.multi_edit_dragging {
                    self.multi_edit_dragging = false;
                    self.commit_multi_transform(state);
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Reset Offset").on_hover_text("Reset to zero offset (relative) or average (absolute)").clicked() {
                        if self.multi_edit_relative {
                            self.multi_transform = Transform {
                                position: [0.0, 0.0, 0.0],
                                rotation: [0.0, 0.0, 0.0],
                                scale: [1.0, 1.0, 1.0],
                            };
                        } else {
                            self.force_sync_multi_transform(state);
                        }
                    }
                    if ui.button("Reset All to Origin").on_hover_text("Set all selected entities to default transform").clicked() {
                        self.multi_transform = Transform::default();
                        self.multi_edit_relative = false;
                        self.apply_transform_to_selection(state);
                    }
                });
            });

        if header.header_response.clicked() {
            if self.expanded_sections.contains("Transform (All)") {
                self.expanded_sections.remove("Transform (All)");
            } else {
                self.expanded_sections.insert("Transform (All)".to_string());
            }
        }
    }

    fn sync_multi_transform(&mut self, state: &EditorState) {
        // Only sync when selection changes
        if self.multi_selection == state.selection.entities {
            return;
        }

        self.multi_selection = state.selection.entities.clone();
        self.multi_edit_start_transforms.clear();
        self.multi_edit_dragging = false;

        if self.multi_selection.is_empty() {
            self.multi_transform = Transform::default();
            return;
        }

        // In relative mode, start with zero offset
        if self.multi_edit_relative {
            self.multi_transform = Transform {
                position: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0], // Scale multiplier of 1.0 = no change
            };
        } else {
            // In absolute mode, compute average
            self.force_sync_multi_transform(state);
        }
    }

    /// Force sync to average values (for absolute mode)
    fn force_sync_multi_transform(&mut self, state: &EditorState) {
        let mut sum_position = [0.0_f32; 3];
        let mut sum_rotation = [0.0_f32; 3];
        let mut sum_scale = [0.0_f32; 3];
        let mut count = 0.0_f32;

        for id in &self.multi_selection {
            if let Some(entity) = state.scene.get(id) {
                for i in 0..3 {
                    sum_position[i] += entity.transform.position[i];
                    sum_rotation[i] += entity.transform.rotation[i];
                    sum_scale[i] += entity.transform.scale[i];
                }
                count += 1.0;
            }
        }

        if count > 0.0 {
            for i in 0..3 {
                sum_position[i] /= count;
                sum_rotation[i] /= count;
                sum_scale[i] /= count;
            }
            self.multi_transform = Transform {
                position: sum_position,
                rotation: sum_rotation,
                scale: sum_scale,
            };
        }
    }

    /// Apply transform changes with live preview (directly modifies scene, no undo)
    fn apply_transform_live(&self, state: &mut EditorState) {
        if self.multi_edit_relative {
            // Relative mode: apply offset from start transforms
            for (id, start_transform) in &self.multi_edit_start_transforms {
                if let Some(entity) = state.scene.get_mut(id) {
                    entity.transform = Transform {
                        position: [
                            start_transform.position[0] + self.multi_transform.position[0],
                            start_transform.position[1] + self.multi_transform.position[1],
                            start_transform.position[2] + self.multi_transform.position[2],
                        ],
                        rotation: [
                            start_transform.rotation[0] + self.multi_transform.rotation[0],
                            start_transform.rotation[1] + self.multi_transform.rotation[1],
                            start_transform.rotation[2] + self.multi_transform.rotation[2],
                        ],
                        scale: [
                            start_transform.scale[0] * self.multi_transform.scale[0],
                            start_transform.scale[1] * self.multi_transform.scale[1],
                            start_transform.scale[2] * self.multi_transform.scale[2],
                        ],
                    };
                }
            }
        } else {
            // Absolute mode: set all to same value
            for id in &state.selection.entities {
                if let Some(entity) = state.scene.get_mut(id) {
                    entity.transform = self.multi_transform.clone();
                }
            }
        }
    }

    /// Commit the multi-transform edit to undo history
    fn commit_multi_transform(&mut self, state: &mut EditorState) {
        if self.multi_edit_start_transforms.is_empty() {
            return;
        }

        // Build final transforms
        let ids: Vec<_> = self.multi_edit_start_transforms.iter().map(|(id, _)| *id).collect();
        let transforms: Vec<_> = if self.multi_edit_relative {
            self.multi_edit_start_transforms.iter().map(|(_, start)| {
                Transform {
                    position: [
                        start.position[0] + self.multi_transform.position[0],
                        start.position[1] + self.multi_transform.position[1],
                        start.position[2] + self.multi_transform.position[2],
                    ],
                    rotation: [
                        start.rotation[0] + self.multi_transform.rotation[0],
                        start.rotation[1] + self.multi_transform.rotation[1],
                        start.rotation[2] + self.multi_transform.rotation[2],
                    ],
                    scale: [
                        start.scale[0] * self.multi_transform.scale[0],
                        start.scale[1] * self.multi_transform.scale[1],
                        start.scale[2] * self.multi_transform.scale[2],
                    ],
                }
            }).collect()
        } else {
            vec![self.multi_transform.clone(); ids.len()]
        };

        // Commit to undo history with before values
        state.set_transforms_bulk_with_before(
            &ids,
            &self.multi_edit_start_transforms.iter().map(|(_, t)| t.clone()).collect::<Vec<_>>(),
            &transforms,
            "Multi-Entity Transform",
        );

        // Reset for next edit
        self.multi_edit_start_transforms.clear();

        // In relative mode, reset the offset after commit
        if self.multi_edit_relative {
            self.multi_transform = Transform {
                position: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            };
        }
    }

    fn apply_transform_to_selection(&self, state: &mut EditorState) {
        let ids: Vec<_> = state.selection.entities.to_vec();
        let transforms = vec![self.multi_transform.clone(); ids.len()];
        state.set_transforms_bulk(&ids, &transforms, "Batch Transform");
    }

    fn set_active_for_selection(&self, state: &mut EditorState, active: bool) {
        let ids: Vec<_> = state.selection.entities.to_vec();
        state.set_entities_active_bulk(&ids, active);
    }

    fn set_static_for_selection(&self, state: &mut EditorState, is_static: bool) {
        let ids: Vec<_> = state.selection.entities.to_vec();
        state.set_entities_static_bulk(&ids, is_static);
    }
}

impl Default for InspectorPanel {
    fn default() -> Self {
        Self::new()
    }
}
