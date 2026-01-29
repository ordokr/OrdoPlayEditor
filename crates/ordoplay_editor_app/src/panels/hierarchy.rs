// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hierarchy panel - Entity tree view.

use crate::state::{EditorState, EntityId, SelectMode};
use std::collections::HashSet;

/// The hierarchy panel showing the entity tree
pub struct HierarchyPanel {
    /// Search filter
    pub filter: String,
    /// Show hidden entities
    pub show_hidden: bool,
    /// Expanded state per entity
    pub expanded: HashSet<EntityId>,
    /// Entity being renamed (if any)
    renaming: Option<EntityId>,
    /// Rename buffer
    rename_buffer: String,
    /// Currently dragged entity (for reparenting)
    dragging_entity: Option<EntityId>,
}

impl HierarchyPanel {
    /// Create a new hierarchy panel
    pub fn new() -> Self {
        Self {
            filter: String::new(),
            show_hidden: false,
            expanded: HashSet::new(),
            renaming: None,
            rename_buffer: String::new(),
            dragging_entity: None,
        }
    }

    /// Render the hierarchy panel
    pub fn ui(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        // Toolbar
        ui.horizontal(|ui| {
            // Search box
            ui.add(
                egui::TextEdit::singleline(&mut self.filter)
                    .hint_text("Search...")
                    .desired_width(ui.available_width() - 60.0),
            );

            // Add entity button
            if ui.button("+").on_hover_text("Create Entity").clicked() {
                self.create_entity(state);
            }

            // Options menu
            ui.menu_button("...", |ui| {
                ui.checkbox(&mut self.show_hidden, "Show Hidden");
                if ui.button("Expand All").clicked() {
                    self.expand_all(state);
                    ui.close_menu();
                }
                if ui.button("Collapse All").clicked() {
                    self.collapse_all();
                    ui.close_menu();
                }
            });
        });

        ui.separator();

        // Entity tree
        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(dragging) = self.dragging_entity {
                let sources = self.drag_sources(state, dragging);
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 20.0),
                    egui::Sense::hover(),
                );
                let dropped = response.hovered() && ui.input(|i| i.pointer.any_released());
                if dropped {
                    state.reparent_entities_with_command(&sources, None);
                    self.dragging_entity = None;
                }

                let fill = egui::Color32::from_rgba_unmultiplied(40, 120, 200, 28);
                let stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(70));
                ui.painter().rect_filled(rect, 4.0, fill);
                ui.painter().rect_stroke(rect, 4.0, stroke);
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Drop To Root",
                    egui::TextStyle::Small.resolve(ui.style()),
                    ui.style().visuals.text_color(),
                );
                response.on_hover_text("Drop here to unparent (move to root)");

                ui.add_space(4.0);
            }

            // Get root entities from scene
            let roots = state.scene.root_entities();

            if roots.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("No entities in scene");
                });
            } else {
                for entity_id in roots {
                    self.render_node(ui, entity_id, state, 0);
                }
            }
        });

        if ui.input(|i| i.pointer.any_released()) {
            self.dragging_entity = None;
        }
    }

    fn render_node(&mut self, ui: &mut egui::Ui, entity_id: EntityId, state: &mut EditorState, depth: usize) {
        // Get entity data
        let entity = match state.scene.get(&entity_id) {
            Some(e) => e.clone(),
            None => return,
        };

        // Filter check
        if !self.filter.is_empty() {
            let matches_filter = entity.name.to_lowercase().contains(&self.filter.to_lowercase());
            let any_child_matches = entity.children.iter().any(|child_id| {
                state.scene.get(child_id)
                    .map(|c| c.name.to_lowercase().contains(&self.filter.to_lowercase()))
                    .unwrap_or(false)
            });

            if !matches_filter && !any_child_matches {
                return;
            }
        }

        // Hidden check (based on active flag)
        if !entity.active && !self.show_hidden {
            return;
        }

        let is_selected = state.selection.contains(&entity_id);
        let is_expanded = self.expanded.contains(&entity_id);
        let has_children = !entity.children.is_empty();

        ui.horizontal(|ui| {
            // Indentation
            ui.add_space(depth as f32 * 16.0);

            // Expand/collapse button
            if has_children {
                let icon = if is_expanded { "v" } else { ">" };
                if ui.small_button(icon).clicked() {
                    if is_expanded {
                        self.expanded.remove(&entity_id);
                    } else {
                        self.expanded.insert(entity_id);
                    }
                }
            } else {
                ui.add_space(20.0);
            }

            // Visibility toggle (based on active flag)
            let vis_icon = if entity.active { "O" } else { "-" };
            if ui.small_button(vis_icon).on_hover_text("Toggle Active").clicked() {
                state.set_entity_active(entity_id, !entity.active);
            }

            // Prefab indicator
            let is_prefab_root = state.prefab_manager.is_prefab_root(entity_id);
            let is_prefab_child = state.prefab_manager.is_prefab_entity(entity_id) && !is_prefab_root;

            if is_prefab_root {
                ui.label(egui::RichText::new("\u{f1b2}").color(egui::Color32::from_rgb(100, 180, 255)))
                    .on_hover_text("Prefab Instance (root)");
            } else if is_prefab_child {
                ui.label(egui::RichText::new("\u{f0c1}").color(egui::Color32::from_rgb(100, 180, 255).gamma_multiply(0.6)))
                    .on_hover_text("Prefab Instance (child)");
            }

            // Entity name (selectable)
            let response = if self.renaming == Some(entity_id) {
                // Rename mode
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.rename_buffer)
                        .desired_width(100.0),
                );
                if response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    // Commit the rename with undo support
                    state.set_entity_name(entity_id, self.rename_buffer.clone());
                    self.renaming = None;
                }
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.renaming = None;
                }
                response
            } else {
                // Normal display - dim text if inactive
                let text_color = if entity.active {
                    ui.style().visuals.text_color()
                } else {
                    ui.style().visuals.weak_text_color()
                };

                let label = egui::SelectableLabel::new(is_selected, egui::RichText::new(&entity.name).color(text_color));
                let response = ui.add(label);

                // Double-click to rename
                if response.double_clicked() {
                    self.renaming = Some(entity_id);
                    self.rename_buffer = entity.name.clone();
                }

                response
            };

            if response.drag_started() {
                self.dragging_entity = Some(entity_id);
            }

            if let Some(dragging) = self.dragging_entity {
                let dropped = response.hovered() && ui.input(|i| i.pointer.any_released());
                if dropped && dragging != entity_id {
                    let sources = self.drag_sources(state, dragging);
                    if !self.is_invalid_drop(state, &sources, entity_id) {
                        state.reparent_entities_with_command(&sources, Some(entity_id));
                    }
                    self.dragging_entity = None;
                }

                if response.hovered() && dragging != entity_id {
                    let sources = self.drag_sources(state, dragging);
                    let invalid = self.is_invalid_drop(state, &sources, entity_id);
                    let fill = if invalid {
                        egui::Color32::from_rgba_unmultiplied(180, 60, 60, 35)
                    } else {
                        egui::Color32::from_rgba_unmultiplied(60, 180, 90, 35)
                    };
                    ui.painter().rect_filled(response.rect, 4.0, fill);
                    response.clone().on_hover_text(if invalid {
                        "Cannot parent to self or descendant"
                    } else {
                        "Drop to reparent"
                    });
                }
            }

            // Handle selection
            if response.clicked() {
                let modifiers = ui.input(|i| i.modifiers);
                if modifiers.shift {
                    state.select_mode = SelectMode::Add;
                } else if modifiers.ctrl || modifiers.command {
                    state.select_mode = SelectMode::Toggle;
                } else {
                    state.select_mode = SelectMode::Set;
                }
                state.select(&[entity_id]);
                state.select_mode = SelectMode::Set;
            }

            // Context menu
            response.context_menu(|ui| {
                if ui.button("Rename").clicked() {
                    self.renaming = Some(entity_id);
                    self.rename_buffer = entity.name.clone();
                    ui.close_menu();
                }
                if ui.button("Duplicate").clicked() {
                    self.duplicate_entity(state, entity_id);
                    ui.close_menu();
                }
                if ui.button("Delete").clicked() {
                    self.delete_entity(state, entity_id);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Create Child").clicked() {
                    self.create_child_entity(state, entity_id);
                    ui.close_menu();
                }

                // Prefab options
                ui.separator();
                if is_prefab_root {
                    if ui.button("Open Prefab").clicked() {
                        if let Some(instance) = state.prefab_manager.get_instance(entity_id) {
                            let path = instance.prefab_path.clone();
                            if let Err(e) = state.enter_prefab_edit_mode(&path) {
                                tracing::error!("Failed to open prefab: {}", e);
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button("Select Prefab Asset").clicked() {
                        if let Some(instance) = state.prefab_manager.get_instance(entity_id) {
                            state.selected_asset = Some(instance.prefab_path.clone());
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Unpack Prefab").clicked() {
                        state.unpack_prefab(entity_id);
                        ui.close_menu();
                    }
                    if ui.button("Unpack Prefab Completely").clicked() {
                        state.unpack_prefab_completely(entity_id);
                        ui.close_menu();
                    }
                } else if !is_prefab_child {
                    // Only show "Create Prefab" for non-prefab entities
                    if ui.button("Create Prefab...").clicked() {
                        state.show_create_prefab_dialog = Some(entity_id);
                        ui.close_menu();
                    }
                }
            });
        });

        // Render children if expanded
        if is_expanded && has_children {
            let children = entity.children.clone();
            for child_id in children {
                self.render_node(ui, child_id, state, depth + 1);
            }
        }
    }

    fn drag_sources(&self, state: &EditorState, dragged: EntityId) -> Vec<EntityId> {
        if state.selection.contains(&dragged) {
            state.selection.entities.clone()
        } else {
            vec![dragged]
        }
    }

    fn is_invalid_drop(
        &self,
        state: &EditorState,
        sources: &[EntityId],
        target: EntityId,
    ) -> bool {
        if sources.contains(&target) {
            return true;
        }
        let descendants = state.collect_with_descendants(sources);
        descendants.contains(&target)
    }

    fn create_entity(&mut self, state: &mut EditorState) {
        let Some(entity_id) = state.spawn_entity_with_command("New Entity", None, true) else {
            return;
        };

        self.renaming = Some(entity_id);
        self.rename_buffer = "New Entity".to_string();

        tracing::info!("Created entity {:?}", entity_id);
    }

    fn create_child_entity(&mut self, state: &mut EditorState, parent_id: EntityId) {
        let Some(child_id) = state.spawn_entity_with_command("New Child", Some(parent_id), true) else {
            return;
        };

        // Expand the parent
        self.expanded.insert(parent_id);

        // Select the new child
        state.selection.clear();
        state.selection.add(child_id);

        // Start renaming
        self.renaming = Some(child_id);
        self.rename_buffer = "New Child".to_string();

        tracing::info!("Created child entity {:?} under {:?}", child_id, parent_id);
    }

    fn delete_entity(&mut self, state: &mut EditorState, entity_id: EntityId) {
        state.delete_entities_with_command(&[entity_id]);
    }

    fn duplicate_entity(&mut self, state: &mut EditorState, entity_id: EntityId) {
        let new_ids = state.duplicate_entities(&[entity_id]);
        if let Some(new_id) = new_ids.first().copied() {
            self.renaming = Some(new_id);
            if let Some(entity) = state.scene.get(&new_id) {
                self.rename_buffer = entity.name.clone();
            }
        }
    }

    fn expand_all(&mut self, state: &EditorState) {
        for id in state.scene.entities.keys() {
            self.expanded.insert(*id);
        }
    }

    fn collapse_all(&mut self) {
        self.expanded.clear();
    }
}

impl Default for HierarchyPanel {
    fn default() -> Self {
        Self::new()
    }
}
