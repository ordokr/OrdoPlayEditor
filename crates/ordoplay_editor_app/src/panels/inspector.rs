// SPDX-License-Identifier: MIT OR Apache-2.0
//! Inspector panel - Component/property editor.

use crate::state::{EditorState, EntityId, Transform};

/// The inspector panel for editing entity components
pub struct InspectorPanel {
    /// Sections that are expanded
    expanded_sections: std::collections::HashSet<String>,
    /// Temporary edit values (for tracking changes)
    editing_transform: Option<(EntityId, Transform)>,
    /// Entity name being edited
    editing_name: Option<(EntityId, String)>,
    /// Multi-edit transform buffer
    multi_transform: Transform,
    /// Cached selection for multi-edit
    multi_selection: Vec<EntityId>,
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
            editing_name: None,
            multi_transform: Transform::default(),
            multi_selection: Vec::new(),
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
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Entity header
                    self.entity_header(ui, state, entity_id, &data.name, data.active, data.is_static);

                    ui.separator();

                    // Transform component
                    self.transform_section(ui, state, entity_id, &data.transform);

                    // Other components would go here
                    self.mock_components(ui);
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
        // Initialize editing name if not set
        let edit_name = self.editing_name.get_or_insert_with(|| (entity_id, name.to_string()));

        // Reset if entity changed
        if edit_name.0 != entity_id {
            *edit_name = (entity_id, name.to_string());
        }

        ui.horizontal(|ui| {
            ui.label("Entity:");
            let response = ui.text_edit_singleline(&mut edit_name.1);

            // Commit name change when focus is lost
            if response.lost_focus() && edit_name.1 != name {
                state.set_entity_name(entity_id, edit_name.1.clone());
            }
        });

        ui.horizontal(|ui| {
            if ui.checkbox(&mut active, "Active").changed() {
                if let Some(data) = state.scene.get_mut(&entity_id) {
                    data.active = active;
                    state.dirty = true;
                }
            }
            if ui.checkbox(&mut is_static, "Static").changed() {
                if let Some(data) = state.scene.get_mut(&entity_id) {
                    data.is_static = is_static;
                    state.dirty = true;
                }
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

        // Initialize editing transform if not set
        let edit_transform = self.editing_transform.get_or_insert_with(|| {
            (entity_id, current_transform.clone())
        });

        // Reset if entity changed
        if edit_transform.0 != entity_id {
            *edit_transform = (entity_id, current_transform.clone());
        }

        let header = egui::CollapsingHeader::new("Transform")
            .default_open(expanded)
            .show(ui, |ui| {
                let mut changed = false;

                // Position
                ui.horizontal(|ui| {
                    ui.label("Position");
                    if ui.add(egui::DragValue::new(&mut edit_transform.1.position[0]).speed(0.1).prefix("X: ")).changed() {
                        changed = true;
                    }
                    if ui.add(egui::DragValue::new(&mut edit_transform.1.position[1]).speed(0.1).prefix("Y: ")).changed() {
                        changed = true;
                    }
                    if ui.add(egui::DragValue::new(&mut edit_transform.1.position[2]).speed(0.1).prefix("Z: ")).changed() {
                        changed = true;
                    }
                });

                // Rotation
                ui.horizontal(|ui| {
                    ui.label("Rotation");
                    if ui.add(egui::DragValue::new(&mut edit_transform.1.rotation[0]).speed(1.0).prefix("X: ").suffix("°")).changed() {
                        changed = true;
                    }
                    if ui.add(egui::DragValue::new(&mut edit_transform.1.rotation[1]).speed(1.0).prefix("Y: ").suffix("°")).changed() {
                        changed = true;
                    }
                    if ui.add(egui::DragValue::new(&mut edit_transform.1.rotation[2]).speed(1.0).prefix("Z: ").suffix("°")).changed() {
                        changed = true;
                    }
                });

                // Scale
                ui.horizontal(|ui| {
                    ui.label("Scale   ");
                    if ui.add(egui::DragValue::new(&mut edit_transform.1.scale[0]).speed(0.01).prefix("X: ")).changed() {
                        changed = true;
                    }
                    if ui.add(egui::DragValue::new(&mut edit_transform.1.scale[1]).speed(0.01).prefix("Y: ")).changed() {
                        changed = true;
                    }
                    if ui.add(egui::DragValue::new(&mut edit_transform.1.scale[2]).speed(0.01).prefix("Z: ")).changed() {
                        changed = true;
                    }
                });

                // Commit changes when dragging ends
                if changed {
                    // Apply live updates for visual feedback
                    if let Some(data) = state.scene.get_mut(&entity_id) {
                        data.transform = edit_transform.1.clone();
                    }
                }

                // Check for drag end to commit to undo history
                if ui.input(|i| i.pointer.any_released()) && edit_transform.1 != *current_transform {
                    state.set_transform(entity_id, edit_transform.1.clone(), "Transform entity");
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

    fn mock_components(&mut self, ui: &mut egui::Ui) {
        // Mock: MeshRenderer component
        egui::CollapsingHeader::new("Mesh Renderer")
            .default_open(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Mesh");
                    if ui.button("cube.glb").clicked() {
                        // TODO: Open asset picker
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Material");
                    if ui.button("default_material").clicked() {
                        // TODO: Open material picker
                    }
                });

                let mut cast_shadows = true;
                let mut receive_shadows = true;
                ui.checkbox(&mut cast_shadows, "Cast Shadows");
                ui.checkbox(&mut receive_shadows, "Receive Shadows");
            });

        // Mock: Rigidbody component
        egui::CollapsingHeader::new("Rigidbody")
            .default_open(false)
            .show(ui, |ui| {
                let mut mass = 1.0_f32;
                let mut drag = 0.0_f32;
                let mut is_kinematic = false;
                let mut use_gravity = true;

                ui.horizontal(|ui| {
                    ui.label("Mass");
                    ui.add(egui::DragValue::new(&mut mass).speed(0.1).suffix(" kg"));
                });

                ui.horizontal(|ui| {
                    ui.label("Drag");
                    ui.add(egui::DragValue::new(&mut drag).speed(0.01));
                });

                ui.checkbox(&mut is_kinematic, "Is Kinematic");
                ui.checkbox(&mut use_gravity, "Use Gravity");
            });

        // Add component button
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Add Component").clicked() {
                // TODO: Show component picker popup
            }
        });
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

        let header = egui::CollapsingHeader::new("Transform (All)")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Position");
                    ui.add(egui::DragValue::new(&mut self.multi_transform.position[0]).speed(0.1).prefix("X: "));
                    ui.add(egui::DragValue::new(&mut self.multi_transform.position[1]).speed(0.1).prefix("Y: "));
                    ui.add(egui::DragValue::new(&mut self.multi_transform.position[2]).speed(0.1).prefix("Z: "));
                });

                ui.horizontal(|ui| {
                    ui.label("Rotation");
                    ui.add(egui::DragValue::new(&mut self.multi_transform.rotation[0]).speed(1.0).prefix("X: ").suffix(" deg"));
                    ui.add(egui::DragValue::new(&mut self.multi_transform.rotation[1]).speed(1.0).prefix("Y: ").suffix(" deg"));
                    ui.add(egui::DragValue::new(&mut self.multi_transform.rotation[2]).speed(1.0).prefix("Z: ").suffix(" deg"));
                });

                ui.horizontal(|ui| {
                    ui.label("Scale   ");
                    ui.add(egui::DragValue::new(&mut self.multi_transform.scale[0]).speed(0.01).prefix("X: "));
                    ui.add(egui::DragValue::new(&mut self.multi_transform.scale[1]).speed(0.01).prefix("Y: "));
                    ui.add(egui::DragValue::new(&mut self.multi_transform.scale[2]).speed(0.01).prefix("Z: "));
                });

                ui.horizontal(|ui| {
                    if ui.button("Apply Transform").clicked() {
                        self.apply_transform_to_selection(state);
                    }
                    if ui.button("Reset Transform").clicked() {
                        self.multi_transform = Transform::default();
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
        if self.multi_selection == state.selection.entities {
            return;
        }

        self.multi_selection = state.selection.entities.clone();
        if self.multi_selection.is_empty() {
            self.multi_transform = Transform::default();
            return;
        }

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

    fn apply_transform_to_selection(&self, state: &mut EditorState) {
        let ids: Vec<_> = state.selection.entities.iter().copied().collect();
        let transforms = vec![self.multi_transform.clone(); ids.len()];
        state.set_transforms_bulk(&ids, &transforms, "Batch Transform");
    }

    fn set_active_for_selection(&self, state: &mut EditorState, active: bool) {
        for id in state.selection.entities.iter() {
            if let Some(entity) = state.scene.get_mut(id) {
                entity.active = active;
            }
        }
        state.dirty = true;
    }

    fn set_static_for_selection(&self, state: &mut EditorState, is_static: bool) {
        for id in state.selection.entities.iter() {
            if let Some(entity) = state.scene.get_mut(id) {
                entity.is_static = is_static;
            }
        }
        state.dirty = true;
    }
}

impl Default for InspectorPanel {
    fn default() -> Self {
        Self::new()
    }
}
