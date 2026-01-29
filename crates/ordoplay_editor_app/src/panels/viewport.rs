// SPDX-License-Identifier: MIT OR Apache-2.0
//! Viewport panel - 3D scene view with gizmos and camera controls.


use crate::state::{EditorState, EntityId, SelectMode};
use crate::tools::{EditorCamera, GizmoMode, GizmoOperation};
use crate::viewport_renderer::ViewportRenderer;
use egui_wgpu::wgpu;

/// Gizmo axis being dragged
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoAxis {
    X,
    Y,
    Z,
}

/// Active gizmo drag state
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone)]
struct GizmoDragState {
    /// Which axis is being dragged
    axis: GizmoAxis,
    /// Starting transforms for all selected entities (`entity_id`, transform)
    start_transforms: Vec<(EntityId, crate::state::Transform)>,
    /// Starting mouse position
    start_mouse: egui::Pos2,
    /// Primary entity being manipulated (for gizmo positioning)
    primary_entity_id: EntityId,
}

/// The main 3D viewport panel
pub struct ViewportPanel {
    /// Editor camera
    pub camera: EditorCamera,
    /// Current gizmo operation (if any)
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub gizmo_op: Option<GizmoOperation>,
    /// Render texture handle (will be set up with wgpu)
    render_texture: Option<egui::TextureId>,
    /// Last known viewport size
    viewport_size: [f32; 2],
    /// Whether the viewport has focus
    has_focus: bool,
    /// Grid visibility
    pub show_grid: bool,
    /// Gizmo visibility
    pub show_gizmos: bool,
    /// Stats overlay visibility
    pub show_stats: bool,
    /// Active gizmo drag state
    gizmo_drag: Option<GizmoDragState>,
    /// Currently hovered gizmo axis (for highlighting)
    hovered_axis: Option<GizmoAxis>,
}

impl ViewportPanel {
    /// Create a new viewport panel
    pub fn new() -> Self {
        Self {
            camera: EditorCamera::new(),
            gizmo_op: None,
            render_texture: None,
            viewport_size: [800.0, 600.0],
            has_focus: false,
            show_grid: true,
            show_gizmos: true,
            show_stats: true,
            gizmo_drag: None,
            hovered_axis: None,
        }
    }

    /// Render the viewport panel
    pub fn ui(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        // Toolbar
        ui.horizontal(|ui| {
            self.toolbar(ui, state);
        });

        ui.separator();

        // Main viewport area
        let available_size = ui.available_size();
        self.viewport_size = [available_size.x, available_size.y];

        let (response, painter) = ui.allocate_painter(available_size, egui::Sense::click_and_drag());

        // Track focus
        self.has_focus = response.has_focus() || response.hovered();

        // Draw viewport background
        painter.rect_filled(response.rect, 0.0, egui::Color32::from_rgb(30, 30, 30));

        // Draw placeholder content
        if self.render_texture.is_none() {
            // Draw a grid pattern as placeholder
            self.draw_placeholder_grid(&painter, response.rect);
        }

        // Draw viewport overlay info
        self.draw_overlay(ui, &painter, response.rect, state);

        // Handle input
        self.handle_input(&response, state);

        // Draw gizmos if selection exists
        if !state.selection.is_empty() && self.show_gizmos {
            self.draw_gizmo_overlay(&painter, response.rect, state);
        }
    }

    /// Render the viewport panel with a 3D renderer
    pub fn ui_with_renderer(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut EditorState,
        renderer: &mut ViewportRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        egui_renderer: &mut egui_wgpu::Renderer,
    ) {
        // Toolbar
        ui.horizontal(|ui| {
            self.toolbar(ui, state);
        });

        ui.separator();

        // Main viewport area
        let available_size = ui.available_size();
        self.viewport_size = [available_size.x, available_size.y];

        // Convert to pixels (accounting for scale factor)
        let pixels_per_point = ui.ctx().pixels_per_point();
        let width = (available_size.x * pixels_per_point) as u32;
        let height = (available_size.y * pixels_per_point) as u32;

        // Resize renderer if needed
        renderer.resize(device, [width.max(1), height.max(1)]);

        // Update camera
        let aspect = available_size.x / available_size.y.max(1.0);
        renderer.update_camera(
            queue,
            self.camera.position,
            self.camera.target,
            [0.0, 1.0, 0.0], // up vector
            aspect,
            std::f32::consts::FRAC_PI_4, // 45 degree FOV
            0.1,
            1000.0,
        );

        // Render the 3D scene
        renderer.render(device, queue, self.show_grid);

        // Get or create egui texture ID
        let texture_id = renderer.get_egui_texture_id(egui_renderer, device);
        self.render_texture = Some(texture_id);

        // Allocate space and display the rendered texture
        let (response, painter) = ui.allocate_painter(available_size, egui::Sense::click_and_drag());

        // Track focus
        self.has_focus = response.has_focus() || response.hovered();

        // Draw the rendered texture
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        painter.image(texture_id, response.rect, uv, egui::Color32::WHITE);

        // Draw viewport overlay info
        self.draw_overlay(ui, &painter, response.rect, state);

        // Handle input
        self.handle_input(&response, state);

        // Draw gizmos if selection exists
        if !state.selection.is_empty() && self.show_gizmos {
            self.draw_gizmo_overlay(&painter, response.rect, state);
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        // Gizmo mode buttons
        let modes = [
            (GizmoMode::Translate, "W"),
            (GizmoMode::Rotate, "E"),
            (GizmoMode::Scale, "R"),
        ];

        for (mode, key) in modes {
            let selected = state.gizmo_mode == mode;
            if ui
                .selectable_label(selected, format!("{} {}", mode.icon(), key))
                .on_hover_text(format!("{} ({})", mode.name(), key))
                .clicked()
            {
                state.gizmo_mode = mode;
            }
        }

        ui.separator();

        // Coordinate space toggle
        let space_text = if state.use_world_space { "World" } else { "Local" };
        if ui.button(space_text).on_hover_text("Toggle coordinate space").clicked() {
            state.use_world_space = !state.use_world_space;
        }

        // Snap toggle
        let snap_text = if state.snap_enabled {
            format!("Snap: {}", state.snap_size)
        } else {
            "Snap: Off".to_string()
        };
        if ui.button(&snap_text).on_hover_text("Toggle grid snapping").clicked() {
            state.snap_enabled = !state.snap_enabled;
        }

        ui.separator();

        // View options
        ui.checkbox(&mut self.show_grid, "Grid");
        ui.checkbox(&mut self.show_gizmos, "Gizmos");
        ui.checkbox(&mut self.show_stats, "Stats");
    }

    fn draw_placeholder_grid(&self, painter: &egui::Painter, rect: egui::Rect) {
        let grid_color = egui::Color32::from_rgb(50, 50, 50);
        let grid_spacing = 50.0;

        // Vertical lines
        let mut x = rect.left();
        while x < rect.right() {
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(1.0, grid_color),
            );
            x += grid_spacing;
        }

        // Horizontal lines
        let mut y = rect.top();
        while y < rect.bottom() {
            painter.line_segment(
                [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                egui::Stroke::new(1.0, grid_color),
            );
            y += grid_spacing;
        }

        // Center crosshair
        let center = rect.center();
        let cross_size = 20.0;
        let axis_colors = [
            egui::Color32::from_rgb(255, 80, 80),  // X - Red
            egui::Color32::from_rgb(80, 255, 80),  // Y - Green
        ];

        painter.line_segment(
            [egui::pos2(center.x - cross_size, center.y), egui::pos2(center.x + cross_size, center.y)],
            egui::Stroke::new(2.0, axis_colors[0]),
        );
        painter.line_segment(
            [egui::pos2(center.x, center.y - cross_size), egui::pos2(center.x, center.y + cross_size)],
            egui::Stroke::new(2.0, axis_colors[1]),
        );

        // Placeholder text
        painter.text(
            center,
            egui::Align2::CENTER_CENTER,
            "Viewport\n(Renderer pending integration)",
            egui::FontId::proportional(16.0),
            egui::Color32::from_rgb(100, 100, 100),
        );
    }

    fn draw_overlay(&self, _ui: &egui::Ui, painter: &egui::Painter, rect: egui::Rect, state: &EditorState) {
        if !self.show_stats {
            return;
        }

        let margin = 10.0;
        let mut y = rect.top() + margin;
        let x = rect.left() + margin;
        let line_height = 16.0;
        let font = egui::FontId::monospace(12.0);
        let color = egui::Color32::from_rgb(200, 200, 200);

        // Camera info
        painter.text(
            egui::pos2(x, y),
            egui::Align2::LEFT_TOP,
            format!("Pos: ({:.1}, {:.1}, {:.1})",
                self.camera.position[0],
                self.camera.position[1],
                self.camera.position[2]),
            font.clone(),
            color,
        );
        y += line_height;

        painter.text(
            egui::pos2(x, y),
            egui::Align2::LEFT_TOP,
            format!("Target: ({:.1}, {:.1}, {:.1})",
                self.camera.target[0],
                self.camera.target[1],
                self.camera.target[2]),
            font.clone(),
            color,
        );
        y += line_height;

        // Selection info
        let selection_count = state.selection.len();
        painter.text(
            egui::pos2(x, y),
            egui::Align2::LEFT_TOP,
            format!("Selected: {}", selection_count),
            font.clone(),
            color,
        );
        y += line_height;

        // Gizmo mode
        painter.text(
            egui::pos2(x, y),
            egui::Align2::LEFT_TOP,
            format!("Mode: {} ({})", state.gizmo_mode.name(),
                if state.use_world_space { "World" } else { "Local" }),
            font.clone(),
            color,
        );
    }

    fn draw_gizmo_overlay(&self, painter: &egui::Painter, rect: egui::Rect, state: &EditorState) {
        // Get selected entity position for gizmo placement
        let gizmo_center = if let Some(entity_id) = state.selection.primary() {
            if let Some(entity) = state.scene.get(entity_id) {
                // Project the entity's world position to screen space
                self.project_to_screen(entity.transform.position, rect)
            } else {
                rect.center()
            }
        } else {
            rect.center()
        };

        let size = 60.0;

        // Base colors per axis
        let base_colors = [
            egui::Color32::from_rgb(255, 80, 80),   // X - Red
            egui::Color32::from_rgb(80, 255, 80),   // Y - Green
            egui::Color32::from_rgb(80, 80, 255),   // Z - Blue
        ];

        // Highlight colors when hovered/dragging
        let highlight_colors = [
            egui::Color32::from_rgb(255, 200, 100), // X highlighted
            egui::Color32::from_rgb(200, 255, 100), // Y highlighted
            egui::Color32::from_rgb(100, 200, 255), // Z highlighted
        ];

        // Determine which axis to highlight
        let get_color = |axis: GizmoAxis| {
            let is_dragging = self.gizmo_drag.as_ref().map(|d| d.axis == axis).unwrap_or(false);
            let is_hovered = self.hovered_axis == Some(axis);

            if is_dragging || is_hovered {
                match axis {
                    GizmoAxis::X => highlight_colors[0],
                    GizmoAxis::Y => highlight_colors[1],
                    GizmoAxis::Z => highlight_colors[2],
                }
            } else {
                match axis {
                    GizmoAxis::X => base_colors[0],
                    GizmoAxis::Y => base_colors[1],
                    GizmoAxis::Z => base_colors[2],
                }
            }
        };

        let get_stroke_width = |axis: GizmoAxis| {
            let is_dragging = self.gizmo_drag.as_ref().map(|d| d.axis == axis).unwrap_or(false);
            let is_hovered = self.hovered_axis == Some(axis);

            if is_dragging || is_hovered {
                4.0
            } else {
                2.0
            }
        };

        // Draw mode indicator
        let mode_label = match state.gizmo_mode {
            GizmoMode::Translate => "Move",
            GizmoMode::Rotate => "Rotate",
            GizmoMode::Scale => "Scale",
        };
        painter.text(
            egui::pos2(gizmo_center.x, gizmo_center.y - size - 15.0),
            egui::Align2::CENTER_BOTTOM,
            mode_label,
            egui::FontId::proportional(11.0),
            egui::Color32::WHITE,
        );

        // X axis (right)
        painter.arrow(
            gizmo_center,
            egui::vec2(size, 0.0),
            egui::Stroke::new(get_stroke_width(GizmoAxis::X), get_color(GizmoAxis::X)),
        );

        // Y axis (up)
        painter.arrow(
            gizmo_center,
            egui::vec2(0.0, -size),
            egui::Stroke::new(get_stroke_width(GizmoAxis::Y), get_color(GizmoAxis::Y)),
        );

        // Z axis (diagonal for 2D representation - towards camera)
        painter.arrow(
            gizmo_center,
            egui::vec2(-size * 0.5, size * 0.5),
            egui::Stroke::new(get_stroke_width(GizmoAxis::Z), get_color(GizmoAxis::Z)),
        );

        // Draw axis labels
        painter.text(
            egui::pos2(gizmo_center.x + size + 8.0, gizmo_center.y),
            egui::Align2::LEFT_CENTER,
            "X",
            egui::FontId::proportional(12.0),
            get_color(GizmoAxis::X),
        );
        painter.text(
            egui::pos2(gizmo_center.x, gizmo_center.y - size - 8.0),
            egui::Align2::CENTER_BOTTOM,
            "Y",
            egui::FontId::proportional(12.0),
            get_color(GizmoAxis::Y),
        );
        painter.text(
            egui::pos2(gizmo_center.x - size * 0.5 - 8.0, gizmo_center.y + size * 0.5),
            egui::Align2::RIGHT_CENTER,
            "Z",
            egui::FontId::proportional(12.0),
            get_color(GizmoAxis::Z),
        );
    }

    /// Project a 3D world position to 2D screen position
    fn project_to_screen(&self, world_pos: [f32; 3], rect: egui::Rect) -> egui::Pos2 {
        // Simple projection using camera matrices
        let cam_pos = self.camera.position;
        let cam_forward = self.camera.get_forward();
        let cam_right = self.camera.get_right();
        let cam_up = self.camera.get_up();

        // Vector from camera to world position
        let to_point = [
            world_pos[0] - cam_pos[0],
            world_pos[1] - cam_pos[1],
            world_pos[2] - cam_pos[2],
        ];

        // Project onto camera plane
        let depth = to_point[0] * cam_forward[0] + to_point[1] * cam_forward[1] + to_point[2] * cam_forward[2];

        if depth <= 0.1 {
            // Behind camera
            return rect.center();
        }

        let x = to_point[0] * cam_right[0] + to_point[1] * cam_right[1] + to_point[2] * cam_right[2];
        let y = to_point[0] * cam_up[0] + to_point[1] * cam_up[1] + to_point[2] * cam_up[2];

        // Apply perspective
        let fov_factor = (std::f32::consts::FRAC_PI_4 * 0.5).tan();
        let aspect = rect.width() / rect.height().max(1.0);

        let screen_x = (x / (depth * fov_factor * aspect)) * 0.5 + 0.5;
        let screen_y = (-y / (depth * fov_factor)) * 0.5 + 0.5;

        egui::pos2(
            rect.left() + screen_x * rect.width(),
            rect.top() + screen_y * rect.height(),
        )
    }

    /// Get the gizmo center in screen space
    fn get_gizmo_screen_center(&self, rect: egui::Rect, state: &EditorState) -> Option<egui::Pos2> {
        state.selection.primary().and_then(|entity_id| {
            state.scene.get(entity_id).map(|entity| {
                self.project_to_screen(entity.transform.position, rect)
            })
        })
    }

    /// Check if a screen position is over a gizmo axis
    fn hit_test_gizmo(&self, pos: egui::Pos2, gizmo_center: egui::Pos2) -> Option<GizmoAxis> {
        let size = 60.0;
        let hit_radius = 12.0;

        // X axis (right)
        let x_end = egui::pos2(gizmo_center.x + size, gizmo_center.y);
        if Self::point_near_line(pos, gizmo_center, x_end, hit_radius) {
            return Some(GizmoAxis::X);
        }

        // Y axis (up)
        let y_end = egui::pos2(gizmo_center.x, gizmo_center.y - size);
        if Self::point_near_line(pos, gizmo_center, y_end, hit_radius) {
            return Some(GizmoAxis::Y);
        }

        // Z axis (diagonal)
        let z_end = egui::pos2(gizmo_center.x - size * 0.5, gizmo_center.y + size * 0.5);
        if Self::point_near_line(pos, gizmo_center, z_end, hit_radius) {
            return Some(GizmoAxis::Z);
        }

        None
    }

    /// Check if a point is near a line segment
    fn point_near_line(point: egui::Pos2, line_start: egui::Pos2, line_end: egui::Pos2, threshold: f32) -> bool {
        let line_vec = line_end - line_start;
        let point_vec = point - line_start;
        let line_len_sq = line_vec.x * line_vec.x + line_vec.y * line_vec.y;

        if line_len_sq == 0.0 {
            return point_vec.length() < threshold;
        }

        let t = ((point_vec.x * line_vec.x + point_vec.y * line_vec.y) / line_len_sq).clamp(0.0, 1.0);
        let closest = egui::pos2(line_start.x + t * line_vec.x, line_start.y + t * line_vec.y);
        let dist = (point - closest).length();

        dist < threshold
    }

    fn handle_input(&mut self, response: &egui::Response, state: &mut EditorState) {
        // Only handle input if viewport is focused
        if !self.has_focus {
            return;
        }

        let modifiers = response.ctx.input(|i| i.modifiers);
        let rect = response.rect;

        // Update hovered gizmo axis
        if let Some(hover_pos) = response.hover_pos() {
            if let Some(gizmo_center) = self.get_gizmo_screen_center(rect, state) {
                self.hovered_axis = self.hit_test_gizmo(hover_pos, gizmo_center);
            } else {
                self.hovered_axis = None;
            }
        } else {
            self.hovered_axis = None;
        }

        // Handle gizmo drag
        if let Some(drag_state) = &self.gizmo_drag {
            if response.dragged_by(egui::PointerButton::Primary) {
                // Continue dragging
                if let Some(current_pos) = response.hover_pos() {
                    let delta = current_pos - drag_state.start_mouse;
                    let sensitivity = 0.02 * self.camera.distance;

                    // Calculate transform delta based on gizmo mode
                    let (pos_delta, rot_delta, scale_delta) = match state.gizmo_mode {
                        GizmoMode::Translate => {
                            let mut d = [0.0, 0.0, 0.0];
                            match drag_state.axis {
                                GizmoAxis::X => d[0] = delta.x * sensitivity,
                                GizmoAxis::Y => d[1] = -delta.y * sensitivity,
                                GizmoAxis::Z => d[2] = (-delta.x + delta.y) * sensitivity * 0.5,
                            }
                            // Apply grid snapping if enabled
                            if state.snap_enabled {
                                let snap = state.snap_size;
                                d[0] = (d[0] / snap).round() * snap;
                                d[1] = (d[1] / snap).round() * snap;
                                d[2] = (d[2] / snap).round() * snap;
                            }
                            (d, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0])
                        }
                        GizmoMode::Rotate => {
                            let rotation_sensitivity = 0.5;
                            let mut d = [0.0, 0.0, 0.0];
                            match drag_state.axis {
                                GizmoAxis::X => d[0] = delta.y * rotation_sensitivity,
                                GizmoAxis::Y => d[1] = delta.x * rotation_sensitivity,
                                GizmoAxis::Z => d[2] = (delta.x - delta.y) * rotation_sensitivity * 0.5,
                            }
                            // Apply rotation snapping if enabled (15 degree increments)
                            if state.snap_enabled {
                                let snap = state.rotation_snap;
                                d[0] = (d[0] / snap).round() * snap;
                                d[1] = (d[1] / snap).round() * snap;
                                d[2] = (d[2] / snap).round() * snap;
                            }
                            ([0.0, 0.0, 0.0], d, [0.0, 0.0, 0.0])
                        }
                        GizmoMode::Scale => {
                            let scale_delta_val = (delta.x - delta.y) * 0.01;
                            let mut d = [0.0, 0.0, 0.0];
                            match drag_state.axis {
                                GizmoAxis::X => d[0] = scale_delta_val,
                                GizmoAxis::Y => d[1] = scale_delta_val,
                                GizmoAxis::Z => d[2] = scale_delta_val,
                            }
                            // Apply scale snapping if enabled
                            if state.snap_enabled {
                                let snap = state.scale_snap;
                                d[0] = (d[0] / snap).round() * snap;
                                d[1] = (d[1] / snap).round() * snap;
                                d[2] = (d[2] / snap).round() * snap;
                            }
                            ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], d)
                        }
                    };

                    // Apply transform delta to ALL selected entities
                    for (entity_id, start_transform) in &drag_state.start_transforms {
                        if let Some(entity_data) = state.scene.get_mut(entity_id) {
                            let mut new_transform = start_transform.clone();
                            // Apply position delta
                            new_transform.position[0] += pos_delta[0];
                            new_transform.position[1] += pos_delta[1];
                            new_transform.position[2] += pos_delta[2];
                            // Apply rotation delta
                            new_transform.rotation[0] += rot_delta[0];
                            new_transform.rotation[1] += rot_delta[1];
                            new_transform.rotation[2] += rot_delta[2];
                            // Apply scale delta (additive, clamped to min 0.01)
                            new_transform.scale[0] = (start_transform.scale[0] + scale_delta[0]).max(0.01);
                            new_transform.scale[1] = (start_transform.scale[1] + scale_delta[1]).max(0.01);
                            new_transform.scale[2] = (start_transform.scale[2] + scale_delta[2]).max(0.01);
                            entity_data.transform = new_transform;
                        }
                    }
                }
            } else {
                // End drag - commit to undo history for all entities
                let start_transforms = drag_state.start_transforms.clone();
                self.gizmo_drag = None;

                let description = match state.gizmo_mode {
                    GizmoMode::Translate => if start_transforms.len() > 1 { "Move entities" } else { "Move entity" },
                    GizmoMode::Rotate => if start_transforms.len() > 1 { "Rotate entities" } else { "Rotate entity" },
                    GizmoMode::Scale => if start_transforms.len() > 1 { "Scale entities" } else { "Scale entity" },
                };

                // Commit each entity's transform change to undo history
                for (entity_id, start_transform) in start_transforms {
                    if let Some(entity_data) = state.scene.get(&entity_id) {
                        let new_transform = entity_data.transform.clone();
                        if new_transform != start_transform {
                            state.set_transform_with_before(entity_id, start_transform, new_transform, description);
                        }
                    }
                }
            }
            return; // Don't process other input while dragging gizmo
        }

        // Start gizmo drag
        if response.drag_started_by(egui::PointerButton::Primary) && !modifiers.alt {
            if let Some(start_pos) = response.hover_pos() {
                if let Some(gizmo_center) = self.get_gizmo_screen_center(rect, state) {
                    if let Some(axis) = self.hit_test_gizmo(start_pos, gizmo_center) {
                        if let Some(primary_id) = state.selection.primary().copied() {
                            // Collect starting transforms for ALL selected entities
                            let start_transforms: Vec<_> = state.selection.entities.iter()
                                .filter_map(|id| {
                                    state.scene.get(id).map(|e| (*id, e.transform.clone()))
                                })
                                .collect();

                            if !start_transforms.is_empty() {
                                self.gizmo_drag = Some(GizmoDragState {
                                    axis,
                                    start_transforms,
                                    start_mouse: start_pos,
                                    primary_entity_id: primary_id,
                                });
                                tracing::debug!("Started gizmo drag on {:?} axis ({} entities)", axis, state.selection.len());
                                return;
                            }
                        }
                    }
                }
            }
        }

        // Right-click drag: Orbit camera
        if response.dragged_by(egui::PointerButton::Secondary) {
            let delta = response.drag_delta();
            self.camera.orbit(delta.x, delta.y);
        }

        // Middle-click drag: Pan camera
        if response.dragged_by(egui::PointerButton::Middle) {
            let delta = response.drag_delta();
            self.camera.pan(delta.x, delta.y);
        }

        // Alt + Left-click drag: Orbit camera (Maya-style)
        if modifiers.alt && response.dragged_by(egui::PointerButton::Primary) {
            let delta = response.drag_delta();
            self.camera.orbit(delta.x, delta.y);
        }

        // Scroll: Zoom camera
        response.ctx.input(|i| {
            if response.hovered() {
                let scroll = i.raw_scroll_delta.y;
                if scroll != 0.0 {
                    self.camera.zoom(scroll * 0.01);
                }
            }
        });

        // Left-click: Select (when not on gizmo and not orbiting)
        if response.clicked() && !modifiers.alt {
            if let Some(click_pos) = response.hover_pos() {
                // Check if clicked on gizmo first
                if let Some(gizmo_center) = self.get_gizmo_screen_center(rect, state) {
                    if self.hit_test_gizmo(click_pos, gizmo_center).is_some() {
                        // Clicked on gizmo, don't change selection
                        return;
                    }
                }

                // Convert click position to viewport-relative coordinates
                let normalized_x = (click_pos.x - rect.left()) / rect.width();
                let normalized_y = (click_pos.y - rect.top()) / rect.height();

                // Raycast to find clicked entity
                if let Some(entity_id) = self.raycast_pick(normalized_x, normalized_y, state) {
                    // Determine select mode based on modifiers
                    if modifiers.shift {
                        state.select_mode = SelectMode::Add;
                    } else if modifiers.ctrl || modifiers.command {
                        state.select_mode = SelectMode::Toggle;
                    } else {
                        state.select_mode = SelectMode::Set;
                    }

                    state.select(&[entity_id]);
                    state.select_mode = SelectMode::Set;
                    tracing::debug!("Selected entity {:?}", entity_id);
                } else {
                    // Clicked on nothing - clear selection unless modifier held
                    if !modifiers.shift && !modifiers.ctrl && !modifiers.command {
                        state.selection.clear();
                        tracing::debug!("Cleared selection");
                    }
                }
            }
        }
    }

    /// Simple raycast picking - returns the entity closest to the camera that was clicked
    fn raycast_pick(&self, normalized_x: f32, normalized_y: f32, state: &EditorState) -> Option<EntityId> {
        // Convert normalized screen coordinates to clip space (-1 to 1)
        let clip_x = normalized_x * 2.0 - 1.0;
        let clip_y = 1.0 - normalized_y * 2.0; // Y is flipped in screen space

        // Calculate ray direction from camera through click point
        // This is a simplified approach - we project the ray using the camera's orientation
        let forward = self.camera.get_forward();
        let right = self.camera.get_right();
        let up = self.camera.get_up();

        // Approximate FOV and aspect ratio
        let fov_rad = std::f32::consts::FRAC_PI_4; // 45 degrees
        let aspect = self.viewport_size[0] / self.viewport_size[1].max(1.0);

        let half_height = (fov_rad * 0.5).tan();
        let half_width = half_height * aspect;

        // Ray direction in world space
        let ray_dir = [
            forward[0] + right[0] * clip_x * half_width + up[0] * clip_y * half_height,
            forward[1] + right[1] * clip_x * half_width + up[1] * clip_y * half_height,
            forward[2] + right[2] * clip_x * half_width + up[2] * clip_y * half_height,
        ];

        // Normalize ray direction
        let ray_len = (ray_dir[0] * ray_dir[0] + ray_dir[1] * ray_dir[1] + ray_dir[2] * ray_dir[2]).sqrt();
        let ray_dir = [ray_dir[0] / ray_len, ray_dir[1] / ray_len, ray_dir[2] / ray_len];

        let ray_origin = self.camera.position;

        // Find closest entity hit by the ray (using sphere intersection)
        let mut closest_entity: Option<EntityId> = None;
        let mut closest_dist = f32::MAX;

        // Assume each entity has a bounding sphere of radius 1.0 for picking
        let pick_radius = 1.0_f32;

        for (entity_id, entity_data) in state.scene.entities.iter() {
            let sphere_center = entity_data.transform.position;

            // Ray-sphere intersection
            let oc = [
                ray_origin[0] - sphere_center[0],
                ray_origin[1] - sphere_center[1],
                ray_origin[2] - sphere_center[2],
            ];

            let a = ray_dir[0] * ray_dir[0] + ray_dir[1] * ray_dir[1] + ray_dir[2] * ray_dir[2];
            let b = 2.0 * (oc[0] * ray_dir[0] + oc[1] * ray_dir[1] + oc[2] * ray_dir[2]);
            let c = oc[0] * oc[0] + oc[1] * oc[1] + oc[2] * oc[2] - pick_radius * pick_radius;

            let discriminant = b * b - 4.0 * a * c;

            if discriminant >= 0.0 {
                let t = (-b - discriminant.sqrt()) / (2.0 * a);
                if t > 0.0 && t < closest_dist {
                    closest_dist = t;
                    closest_entity = Some(*entity_id);
                }
            }
        }

        closest_entity
    }

    /// Focus the camera on the current selection
    pub fn focus_on_selection(&mut self, state: &EditorState) {
        if state.selection.is_empty() {
            // Focus on origin if nothing selected
            self.camera.focus([0.0, 0.0, 0.0], Some(10.0));
            return;
        }

        // Calculate bounding box center of selected entities
        let mut min = [f32::MAX, f32::MAX, f32::MAX];
        let mut max = [f32::MIN, f32::MIN, f32::MIN];
        let mut count = 0;

        for entity_id in state.selection.entities.iter() {
            if let Some(entity) = state.scene.get(entity_id) {
                let pos = entity.transform.position;
                for i in 0..3 {
                    min[i] = min[i].min(pos[i]);
                    max[i] = max[i].max(pos[i]);
                }
                count += 1;
            }
        }

        if count == 0 {
            self.camera.focus([0.0, 0.0, 0.0], Some(10.0));
            return;
        }

        // Calculate center and appropriate distance
        let center = [
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ];

        // Calculate distance based on bounding box size
        let size = [
            (max[0] - min[0]).abs(),
            (max[1] - min[1]).abs(),
            (max[2] - min[2]).abs(),
        ];
        let max_size = size[0].max(size[1]).max(size[2]);
        let distance = (max_size * 2.0).max(5.0); // Ensure minimum distance

        self.camera.focus(center, Some(distance));
        tracing::debug!("Focused on selection: center={:?}, distance={}", center, distance);
    }
}

impl Default for ViewportPanel {
    fn default() -> Self {
        Self::new()
    }
}
