// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hierarchy panel - Entity tree view.

use crate::state::{EditorState, EntityData, EntityId, SelectMode};
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
                if let Some(e) = state.scene.get_mut(&entity_id) {
                    e.active = !e.active;
                    state.dirty = true;
                }
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

    fn create_entity(&mut self, state: &mut EditorState) {
        let entity_id = state.scene.add_entity(EntityData::new("New Entity"));

        // Select the new entity
        state.selection.clear();
        state.selection.add(entity_id);

        // Start renaming immediately
        self.renaming = Some(entity_id);
        self.rename_buffer = "New Entity".to_string();

        state.dirty = true;
        tracing::info!("Created entity {:?}", entity_id);
    }

    fn create_child_entity(&mut self, state: &mut EditorState, parent_id: EntityId) {
        let child_id = state.scene.add_entity(EntityData {
            name: "New Child".to_string(),
            parent: Some(parent_id),
            ..Default::default()
        });

        // Add child to parent's children list
        if let Some(parent) = state.scene.get_mut(&parent_id) {
            parent.children.push(child_id);
        }

        // Expand the parent
        self.expanded.insert(parent_id);

        // Select the new child
        state.selection.clear();
        state.selection.add(child_id);

        // Start renaming
        self.renaming = Some(child_id);
        self.rename_buffer = "New Child".to_string();

        state.dirty = true;
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
