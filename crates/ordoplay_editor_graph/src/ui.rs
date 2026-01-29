// SPDX-License-Identifier: MIT OR Apache-2.0
//! Graph UI rendering with full interactive editing support.
//!
//! Features:
//! - Node rendering with ports
//! - Connection rendering (bezier curves)
//! - Pan/zoom navigation
//! - Node selection and multi-selection
//! - Connection drag-to-create
//! - Node dragging
//! - Context menus
//! - Minimap

use crate::connection::ConnectionId;
use crate::graph::Graph;
use crate::node::{Node, NodeId, NodeRegistry};
use crate::port::{Port, PortDirection, PortId};
use egui::{Color32, Pos2, Rect, Stroke, Vec2};
use std::collections::HashSet;

/// Node visual dimensions
const NODE_WIDTH: f32 = 180.0;
const NODE_HEADER_HEIGHT: f32 = 24.0;
const PORT_HEIGHT: f32 = 22.0;
const PORT_RADIUS: f32 = 6.0;
const PORT_PADDING: f32 = 12.0;
const NODE_ROUNDING: f32 = 6.0;
const NODE_SHADOW_OFFSET: f32 = 3.0;

/// Connection visual parameters
const BEZIER_CURVATURE: f32 = 50.0;
const CONNECTION_THICKNESS: f32 = 2.5;

/// Grid parameters
const GRID_SPACING: f32 = 20.0;

/// Dragging state for creating connections
#[derive(Debug, Clone)]
pub struct ConnectionDrag {
    /// Source node
    pub from_node: NodeId,
    /// Source port
    pub from_port: PortId,
    /// Port direction (determines if dragging from input or output)
    pub direction: PortDirection,
    /// Current mouse position (screen space)
    pub current_pos: Pos2,
}

/// Box selection state
#[derive(Debug, Clone)]
pub struct BoxSelection {
    /// Start position (screen space)
    pub start: Pos2,
    /// Current position (screen space)
    pub current: Pos2,
}

/// Graph editor interaction mode
#[derive(Debug, Clone, Default)]
pub enum InteractionMode {
    /// Default mode - selecting and dragging
    #[default]
    Normal,
    /// Panning the view
    Panning,
    /// Dragging selected nodes
    DraggingNodes {
        /// Starting positions of nodes being dragged (node ID, position)
        start_positions: Vec<(NodeId, [f32; 2])>,
    },
    /// Creating a connection
    CreatingConnection(ConnectionDrag),
    /// Box selection
    BoxSelect(BoxSelection),
}

/// Graph editor UI state
pub struct GraphEditorState {
    /// Current pan offset (graph space)
    pub pan: Vec2,
    /// Current zoom level
    pub zoom: f32,
    /// Selected nodes
    pub selected_nodes: HashSet<NodeId>,
    /// Selected connections
    pub selected_connections: HashSet<ConnectionId>,
    /// Current interaction mode
    pub mode: InteractionMode,
    /// Show minimap
    pub show_minimap: bool,
    /// Show grid
    pub show_grid: bool,
    /// Snap to grid
    pub snap_to_grid: bool,
    /// Grid size for snapping
    pub snap_size: f32,
    /// Last mouse position
    last_mouse_pos: Pos2,
    /// Node being hovered
    hovered_node: Option<NodeId>,
    /// Port being hovered
    hovered_port: Option<(NodeId, PortId)>,
    /// Connection being hovered
    hovered_connection: Option<ConnectionId>,
}

impl GraphEditorState {
    /// Create a new graph editor state
    pub fn new() -> Self {
        Self {
            pan: Vec2::ZERO,
            zoom: 1.0,
            selected_nodes: HashSet::new(),
            selected_connections: HashSet::new(),
            mode: InteractionMode::Normal,
            show_minimap: true,
            show_grid: true,
            snap_to_grid: false,
            snap_size: GRID_SPACING,
            last_mouse_pos: Pos2::ZERO,
            hovered_node: None,
            hovered_port: None,
            hovered_connection: None,
        }
    }

    /// Convert screen position to graph position
    pub fn screen_to_graph(&self, screen_pos: Pos2, rect: Rect) -> Pos2 {
        let center = rect.center();
        Pos2::new(
            (screen_pos.x - center.x) / self.zoom - self.pan.x,
            (screen_pos.y - center.y) / self.zoom - self.pan.y,
        )
    }

    /// Convert graph position to screen position
    pub fn graph_to_screen(&self, graph_pos: Pos2, rect: Rect) -> Pos2 {
        let center = rect.center();
        Pos2::new(
            (graph_pos.x + self.pan.x) * self.zoom + center.x,
            (graph_pos.y + self.pan.y) * self.zoom + center.y,
        )
    }

    /// Snap position to grid
    pub fn snap_position(&self, pos: [f32; 2]) -> [f32; 2] {
        if self.snap_to_grid {
            [
                (pos[0] / self.snap_size).round() * self.snap_size,
                (pos[1] / self.snap_size).round() * self.snap_size,
            ]
        } else {
            pos
        }
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selected_nodes.clear();
        self.selected_connections.clear();
    }

    /// Select a node (optionally add to selection)
    pub fn select_node(&mut self, node_id: NodeId, add_to_selection: bool) {
        if !add_to_selection {
            self.selected_nodes.clear();
            self.selected_connections.clear();
        }
        self.selected_nodes.insert(node_id);
    }

    /// Toggle node selection
    pub fn toggle_node_selection(&mut self, node_id: NodeId) {
        if self.selected_nodes.contains(&node_id) {
            self.selected_nodes.remove(&node_id);
        } else {
            self.selected_nodes.insert(node_id);
        }
    }

    /// Delete selected elements
    pub fn delete_selected(&mut self, graph: &mut Graph) {
        // Delete selected connections
        for conn_id in self.selected_connections.drain() {
            graph.disconnect(conn_id);
        }

        // Delete selected nodes (and their connections)
        for node_id in self.selected_nodes.drain() {
            graph.remove_node(node_id);
        }
    }

    /// Render the graph editor
    pub fn ui(&mut self, ui: &mut egui::Ui, graph: &mut Graph) {
        self.ui_with_registry(ui, graph, None);
    }

    /// Render the graph editor with a node registry for context menus
    pub fn ui_with_registry(
        &mut self,
        ui: &mut egui::Ui,
        graph: &mut Graph,
        registry: Option<&NodeRegistry>,
    ) {
        let rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);

        // Reset hover states
        self.hovered_node = None;
        self.hovered_port = None;
        self.hovered_connection = None;

        // Draw grid
        if self.show_grid {
            self.draw_grid(&painter, rect);
        }

        // Handle input
        self.handle_input(ui, &response, rect, graph, registry);

        // Draw connections first (below nodes)
        self.draw_connections(&painter, rect, graph);

        // Draw connection being created
        if let InteractionMode::CreatingConnection(ref drag) = self.mode {
            self.draw_connection_drag(&painter, rect, graph, drag);
        }

        // Draw nodes
        self.draw_nodes(ui, &painter, rect, graph);

        // Draw box selection
        if let InteractionMode::BoxSelect(ref selection) = self.mode {
            self.draw_box_selection(&painter, selection);
        }

        // Draw minimap
        if self.show_minimap {
            self.draw_minimap(&painter, rect, graph);
        }

        // Draw status bar
        self.draw_status_bar(ui, rect, graph);
    }

    fn draw_grid(&self, painter: &egui::Painter, rect: Rect) {
        let spacing = GRID_SPACING * self.zoom;
        let major_spacing = spacing * 5.0;

        // Grid colors
        let grid_color_minor = Color32::from_rgba_unmultiplied(60, 60, 60, 100);
        let grid_color_major = Color32::from_rgba_unmultiplied(80, 80, 80, 150);

        // Calculate grid offset based on pan
        let offset_x = (self.pan.x * self.zoom) % major_spacing;
        let offset_y = (self.pan.y * self.zoom) % major_spacing;

        // Draw minor grid lines
        let mut x = rect.left() + offset_x % spacing;
        while x < rect.right() {
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(1.0, grid_color_minor),
            );
            x += spacing;
        }

        let mut y = rect.top() + offset_y % spacing;
        while y < rect.bottom() {
            painter.line_segment(
                [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                Stroke::new(1.0, grid_color_minor),
            );
            y += spacing;
        }

        // Draw major grid lines
        x = rect.left() + offset_x % major_spacing;
        while x < rect.right() {
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(1.0, grid_color_major),
            );
            x += major_spacing;
        }

        y = rect.top() + offset_y % major_spacing;
        while y < rect.bottom() {
            painter.line_segment(
                [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                Stroke::new(1.0, grid_color_major),
            );
            y += major_spacing;
        }

        // Draw origin axes
        let origin = self.graph_to_screen(Pos2::ZERO, rect);
        if rect.contains(origin) {
            painter.line_segment(
                [Pos2::new(origin.x, rect.top()), Pos2::new(origin.x, rect.bottom())],
                Stroke::new(2.0, Color32::from_rgba_unmultiplied(100, 100, 150, 180)),
            );
            painter.line_segment(
                [Pos2::new(rect.left(), origin.y), Pos2::new(rect.right(), origin.y)],
                Stroke::new(2.0, Color32::from_rgba_unmultiplied(100, 100, 150, 180)),
            );
        }
    }

    fn handle_input(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        rect: Rect,
        graph: &mut Graph,
        _registry: Option<&NodeRegistry>,
    ) {
        let mouse_pos = ui.input(|i| i.pointer.hover_pos().unwrap_or(self.last_mouse_pos));
        let delta = mouse_pos - self.last_mouse_pos;
        self.last_mouse_pos = mouse_pos;

        // Zoom with scroll wheel
        ui.input(|i| {
            if rect.contains(mouse_pos) {
                let scroll_delta = i.raw_scroll_delta.y;
                if scroll_delta != 0.0 {
                    let zoom_factor = 1.0 + scroll_delta * 0.001;
                    let old_zoom = self.zoom;
                    self.zoom = (self.zoom * zoom_factor).clamp(0.1, 4.0);

                    // Zoom toward mouse position
                    if self.zoom != old_zoom {
                        let mouse_graph = self.screen_to_graph(mouse_pos, rect);
                        let zoom_ratio = self.zoom / old_zoom;
                        self.pan.x += mouse_graph.x * (1.0 - zoom_ratio);
                        self.pan.y += mouse_graph.y * (1.0 - zoom_ratio);
                    }
                }
            }
        });

        // Handle dragging based on mode
        match &mut self.mode {
            InteractionMode::Normal => {
                // Start panning with middle mouse or right mouse + space
                if response.dragged_by(egui::PointerButton::Middle) {
                    self.mode = InteractionMode::Panning;
                }

                // Check for node/port clicks
                if response.clicked() {
                    let graph_pos = self.screen_to_graph(mouse_pos, rect);

                    // Check if clicking on a node
                    let clicked_node = self.find_node_at(graph_pos, graph);
                    let shift_held = ui.input(|i| i.modifiers.shift);

                    if let Some(node_id) = clicked_node {
                        self.select_node(node_id, shift_held);
                    } else if !shift_held {
                        self.clear_selection();
                    }
                }

                // Start box selection with left click on empty space
                if response.drag_started_by(egui::PointerButton::Primary) {
                    let graph_pos = self.screen_to_graph(mouse_pos, rect);
                    if self.find_node_at(graph_pos, graph).is_none() {
                        self.mode = InteractionMode::BoxSelect(BoxSelection {
                            start: mouse_pos,
                            current: mouse_pos,
                        });
                    } else if !self.selected_nodes.is_empty() {
                        // Start dragging nodes
                        let start_positions: Vec<_> = self.selected_nodes
                            .iter()
                            .filter_map(|id| graph.node(*id).map(|n| (*id, n.position)))
                            .collect();
                        self.mode = InteractionMode::DraggingNodes { start_positions };
                    }
                }
            }

            InteractionMode::Panning => {
                if response.dragged() {
                    self.pan += delta / self.zoom;
                }
                if response.drag_stopped() {
                    self.mode = InteractionMode::Normal;
                }
            }

            InteractionMode::DraggingNodes { start_positions: _ } => {
                if response.dragged() {
                    let graph_delta = delta / self.zoom;
                    for node_id in &self.selected_nodes {
                        if let Some(node) = graph.node_mut(*node_id) {
                            node.position[0] += graph_delta.x;
                            node.position[1] += graph_delta.y;
                        }
                    }
                }
                if response.drag_stopped() {
                    // Snap to grid on release
                    if self.snap_to_grid {
                        for node_id in &self.selected_nodes {
                            if let Some(node) = graph.node_mut(*node_id) {
                                node.position = self.snap_position(node.position);
                            }
                        }
                    }
                    self.mode = InteractionMode::Normal;
                }
            }

            InteractionMode::CreatingConnection(drag) => {
                drag.current_pos = mouse_pos;

                if response.drag_stopped() {
                    // Try to complete the connection
                    if let Some((target_node, target_port)) = self.hovered_port {
                        let (from_node, from_port, to_node, to_port) = if drag.direction == PortDirection::Output {
                            (drag.from_node, drag.from_port, target_node, target_port)
                        } else {
                            (target_node, target_port, drag.from_node, drag.from_port)
                        };

                        // Ignore connection errors (e.g., incompatible types)
                        let _ = graph.connect(from_node, from_port, to_node, to_port);
                    }
                    self.mode = InteractionMode::Normal;
                }
            }

            InteractionMode::BoxSelect(selection) => {
                selection.current = mouse_pos;

                if response.drag_stopped() {
                    // Select all nodes in the box
                    let min = Pos2::new(
                        selection.start.x.min(selection.current.x),
                        selection.start.y.min(selection.current.y),
                    );
                    let max = Pos2::new(
                        selection.start.x.max(selection.current.x),
                        selection.start.y.max(selection.current.y),
                    );
                    let selection_rect = Rect::from_min_max(min, max);

                    let shift_held = ui.input(|i| i.modifiers.shift);
                    if !shift_held {
                        self.selected_nodes.clear();
                    }

                    for node in graph.nodes() {
                        let node_screen = self.graph_to_screen(
                            Pos2::new(node.position[0], node.position[1]),
                            rect,
                        );
                        if selection_rect.contains(node_screen) {
                            self.selected_nodes.insert(node.id);
                        }
                    }

                    self.mode = InteractionMode::Normal;
                }
            }
        }

        // Delete key
        ui.input(|i| {
            if i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace) {
                self.delete_selected(graph);
            }
        });
    }

    fn find_node_at(&self, graph_pos: Pos2, graph: &Graph) -> Option<NodeId> {
        // Iterate in reverse to find topmost node first
        for node in graph.nodes() {
            let node_rect = self.get_node_rect(node);
            if node_rect.contains(graph_pos) {
                return Some(node.id);
            }
        }
        None
    }

    fn get_node_rect(&self, node: &Node) -> Rect {
        let port_count = node.inputs.len().max(node.outputs.len());
        let height = NODE_HEADER_HEIGHT + (port_count as f32 * PORT_HEIGHT) + 8.0;
        Rect::from_min_size(
            Pos2::new(node.position[0], node.position[1]),
            Vec2::new(NODE_WIDTH, height),
        )
    }

    fn draw_connections(&mut self, painter: &egui::Painter, rect: Rect, graph: &Graph) {
        for connection in graph.connections() {
            let from_node = graph.node(connection.from_node);
            let to_node = graph.node(connection.to_node);

            if let (Some(from), Some(to)) = (from_node, to_node) {
                let from_pos = self.get_port_position(from, &connection.from_port, rect);
                let to_pos = self.get_port_position(to, &connection.to_port, rect);

                if let (Some(from_screen), Some(to_screen)) = (from_pos, to_pos) {
                    let is_selected = self.selected_connections.contains(&connection.id);
                    let is_hovered = self.hovered_connection == Some(connection.id);

                    // Get port type for color
                    let color = if let Some(port) = from.port(&connection.from_port) {
                        let [r, g, b] = port.port_type.color();
                        if is_selected {
                            Color32::from_rgb(255, 255, 255)
                        } else if is_hovered {
                            Color32::from_rgb(
                                (r as u16 + 50).min(255) as u8,
                                (g as u16 + 50).min(255) as u8,
                                (b as u16 + 50).min(255) as u8,
                            )
                        } else {
                            Color32::from_rgb(r, g, b)
                        }
                    } else {
                        Color32::GRAY
                    };

                    self.draw_bezier_connection(painter, from_screen, to_screen, color);
                }
            }
        }
    }

    fn get_port_position(&self, node: &Node, port_id: &PortId, rect: Rect) -> Option<Pos2> {
        // Find port in inputs
        for (i, port) in node.inputs.iter().enumerate() {
            if port.id == *port_id {
                let y = node.position[1] + NODE_HEADER_HEIGHT + (i as f32 * PORT_HEIGHT) + PORT_HEIGHT / 2.0;
                let pos = Pos2::new(node.position[0], y);
                return Some(self.graph_to_screen(pos, rect));
            }
        }

        // Find port in outputs
        for (i, port) in node.outputs.iter().enumerate() {
            if port.id == *port_id {
                let y = node.position[1] + NODE_HEADER_HEIGHT + (i as f32 * PORT_HEIGHT) + PORT_HEIGHT / 2.0;
                let pos = Pos2::new(node.position[0] + NODE_WIDTH, y);
                return Some(self.graph_to_screen(pos, rect));
            }
        }

        None
    }

    fn draw_bezier_connection(&self, painter: &egui::Painter, from: Pos2, to: Pos2, color: Color32) {
        let distance = (to.x - from.x).abs();
        let curvature = (BEZIER_CURVATURE * self.zoom).min(distance * 0.5);

        let ctrl1 = Pos2::new(from.x + curvature, from.y);
        let ctrl2 = Pos2::new(to.x - curvature, to.y);

        let points = bezier_points(from, ctrl1, ctrl2, to, 32);
        for i in 0..points.len() - 1 {
            painter.line_segment(
                [points[i], points[i + 1]],
                Stroke::new(CONNECTION_THICKNESS * self.zoom, color),
            );
        }
    }

    fn draw_connection_drag(&self, painter: &egui::Painter, rect: Rect, graph: &Graph, drag: &ConnectionDrag) {
        if let Some(node) = graph.node(drag.from_node) {
            if let Some(from_pos) = self.get_port_position(node, &drag.from_port, rect) {
                let color = if let Some(port) = node.port(&drag.from_port) {
                    let [r, g, b] = port.port_type.color();
                    Color32::from_rgb(r, g, b)
                } else {
                    Color32::GRAY
                };

                if drag.direction == PortDirection::Output {
                    self.draw_bezier_connection(painter, from_pos, drag.current_pos, color);
                } else {
                    self.draw_bezier_connection(painter, drag.current_pos, from_pos, color);
                }
            }
        }
    }

    fn draw_nodes(&mut self, ui: &egui::Ui, painter: &egui::Painter, rect: Rect, graph: &mut Graph) {
        let mouse_pos = ui.input(|i| i.pointer.hover_pos().unwrap_or(Pos2::ZERO));

        // Collect node IDs to iterate (to avoid borrow issues)
        let node_ids: Vec<_> = graph.node_ids().collect();

        for node_id in node_ids {
            let node = match graph.node(node_id) {
                Some(n) => n,
                None => continue,
            };

            let is_selected = self.selected_nodes.contains(&node.id);
            let node_rect = self.get_node_rect(node);
            let screen_rect = Rect::from_min_size(
                self.graph_to_screen(node_rect.min, rect),
                node_rect.size() * self.zoom,
            );

            // Check if node is visible
            if !screen_rect.intersects(rect) {
                continue;
            }

            // Draw shadow
            let shadow_rect = screen_rect.translate(Vec2::new(NODE_SHADOW_OFFSET, NODE_SHADOW_OFFSET));
            painter.rect_filled(
                shadow_rect,
                NODE_ROUNDING * self.zoom,
                Color32::from_rgba_unmultiplied(0, 0, 0, 60),
            );

            // Draw node background
            let bg_color = if is_selected {
                Color32::from_rgb(60, 70, 90)
            } else {
                Color32::from_rgb(45, 45, 48)
            };
            painter.rect_filled(screen_rect, NODE_ROUNDING * self.zoom, bg_color);

            // Draw node header
            let header_rect = Rect::from_min_size(
                screen_rect.min,
                Vec2::new(screen_rect.width(), NODE_HEADER_HEIGHT * self.zoom),
            );
            let header_color = if let Some([r, g, b]) = node.color {
                Color32::from_rgb(r, g, b)
            } else {
                Color32::from_rgb(70, 100, 130)
            };
            painter.rect_filled(
                header_rect,
                egui::Rounding {
                    nw: NODE_ROUNDING * self.zoom,
                    ne: NODE_ROUNDING * self.zoom,
                    sw: 0.0,
                    se: 0.0,
                },
                header_color,
            );

            // Draw node title
            painter.text(
                header_rect.center(),
                egui::Align2::CENTER_CENTER,
                &node.name,
                egui::FontId::proportional(12.0 * self.zoom),
                Color32::WHITE,
            );

            // Draw selection outline
            if is_selected {
                painter.rect_stroke(
                    screen_rect,
                    NODE_ROUNDING * self.zoom,
                    Stroke::new(2.0, Color32::from_rgb(100, 150, 255)),
                );
            }

            // Draw ports
            self.draw_ports(ui, painter, rect, node, screen_rect, mouse_pos);
        }
    }

    fn draw_ports(
        &mut self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        _rect: Rect,
        node: &Node,
        screen_rect: Rect,
        mouse_pos: Pos2,
    ) {
        // Draw input ports
        for (i, port) in node.inputs.iter().enumerate() {
            let y_offset = NODE_HEADER_HEIGHT + (i as f32 * PORT_HEIGHT) + PORT_HEIGHT / 2.0;
            let port_pos = Pos2::new(
                screen_rect.left(),
                screen_rect.top() + y_offset * self.zoom,
            );

            let is_hovered = self.draw_port(ui, painter, port, port_pos, mouse_pos);
            if is_hovered {
                self.hovered_port = Some((node.id, port.id));
            }

            // Port label
            painter.text(
                Pos2::new(port_pos.x + PORT_PADDING * self.zoom, port_pos.y),
                egui::Align2::LEFT_CENTER,
                &port.name,
                egui::FontId::proportional(10.0 * self.zoom),
                Color32::from_gray(200),
            );
        }

        // Draw output ports
        for (i, port) in node.outputs.iter().enumerate() {
            let y_offset = NODE_HEADER_HEIGHT + (i as f32 * PORT_HEIGHT) + PORT_HEIGHT / 2.0;
            let port_pos = Pos2::new(
                screen_rect.right(),
                screen_rect.top() + y_offset * self.zoom,
            );

            let is_hovered = self.draw_port(ui, painter, port, port_pos, mouse_pos);
            if is_hovered {
                self.hovered_port = Some((node.id, port.id));
            }

            // Port label
            painter.text(
                Pos2::new(port_pos.x - PORT_PADDING * self.zoom, port_pos.y),
                egui::Align2::RIGHT_CENTER,
                &port.name,
                egui::FontId::proportional(10.0 * self.zoom),
                Color32::from_gray(200),
            );
        }
    }

    fn draw_port(&self, _ui: &egui::Ui, painter: &egui::Painter, port: &Port, pos: Pos2, mouse_pos: Pos2) -> bool {
        let radius = PORT_RADIUS * self.zoom;
        let [r, g, b] = port.port_type.color();
        let color = Color32::from_rgb(r, g, b);

        let is_hovered = pos.distance(mouse_pos) < radius * 1.5;

        // Draw port circle
        if is_hovered {
            painter.circle_filled(pos, radius * 1.3, color);
        } else {
            painter.circle_filled(pos, radius, color);
        }

        // Draw outline
        painter.circle_stroke(pos, radius, Stroke::new(1.0, Color32::from_gray(30)));

        is_hovered
    }

    fn draw_box_selection(&self, painter: &egui::Painter, selection: &BoxSelection) {
        let min = Pos2::new(
            selection.start.x.min(selection.current.x),
            selection.start.y.min(selection.current.y),
        );
        let max = Pos2::new(
            selection.start.x.max(selection.current.x),
            selection.start.y.max(selection.current.y),
        );
        let rect = Rect::from_min_max(min, max);

        painter.rect_filled(
            rect,
            0.0,
            Color32::from_rgba_unmultiplied(100, 150, 255, 30),
        );
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, Color32::from_rgb(100, 150, 255)),
        );
    }

    fn draw_minimap(&self, painter: &egui::Painter, rect: Rect, graph: &Graph) {
        let minimap_size = Vec2::new(150.0, 100.0);
        let minimap_rect = Rect::from_min_size(
            Pos2::new(rect.right() - minimap_size.x - 10.0, rect.bottom() - minimap_size.y - 10.0),
            minimap_size,
        );

        // Draw minimap background
        painter.rect_filled(
            minimap_rect,
            4.0,
            Color32::from_rgba_unmultiplied(30, 30, 30, 200),
        );
        painter.rect_stroke(
            minimap_rect,
            4.0,
            Stroke::new(1.0, Color32::from_gray(60)),
        );

        // Calculate bounds of all nodes
        if graph.node_count() == 0 {
            return;
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for node in graph.nodes() {
            min_x = min_x.min(node.position[0]);
            min_y = min_y.min(node.position[1]);
            max_x = max_x.max(node.position[0] + NODE_WIDTH);
            max_y = max_y.max(node.position[1] + 100.0); // Approximate height
        }

        // Add padding
        let padding = 50.0;
        min_x -= padding;
        min_y -= padding;
        max_x += padding;
        max_y += padding;

        let graph_size = Vec2::new(max_x - min_x, max_y - min_y);
        let scale = (minimap_rect.width() / graph_size.x).min(minimap_rect.height() / graph_size.y);

        // Draw nodes
        for node in graph.nodes() {
            let node_pos = Pos2::new(
                minimap_rect.left() + (node.position[0] - min_x) * scale,
                minimap_rect.top() + (node.position[1] - min_y) * scale,
            );
            let node_size = Vec2::new(NODE_WIDTH * scale, 20.0 * scale);

            let color = if self.selected_nodes.contains(&node.id) {
                Color32::from_rgb(100, 150, 255)
            } else {
                Color32::from_rgb(80, 80, 100)
            };

            painter.rect_filled(
                Rect::from_min_size(node_pos, node_size),
                2.0,
                color,
            );
        }

        // Draw viewport indicator
        let view_min = Pos2::new(
            minimap_rect.left() + (-self.pan.x - min_x - rect.width() / (2.0 * self.zoom)) * scale,
            minimap_rect.top() + (-self.pan.y - min_y - rect.height() / (2.0 * self.zoom)) * scale,
        );
        let view_size = Vec2::new(
            rect.width() / self.zoom * scale,
            rect.height() / self.zoom * scale,
        );

        painter.rect_stroke(
            Rect::from_min_size(view_min, view_size),
            2.0,
            Stroke::new(1.0, Color32::WHITE),
        );
    }

    fn draw_status_bar(&self, ui: &mut egui::Ui, rect: Rect, graph: &Graph) {
        let status_rect = Rect::from_min_size(
            Pos2::new(rect.left() + 5.0, rect.bottom() - 20.0),
            Vec2::new(rect.width() - 10.0, 18.0),
        );

        ui.painter().text(
            status_rect.left_center(),
            egui::Align2::LEFT_CENTER,
            format!(
                "Nodes: {} | Connections: {} | Zoom: {:.0}% | Selected: {}",
                graph.node_count(),
                graph.connection_count(),
                self.zoom * 100.0,
                self.selected_nodes.len(),
            ),
            egui::FontId::proportional(11.0),
            Color32::from_gray(150),
        );
    }
}

impl Default for GraphEditorState {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate points along a cubic bezier curve
fn bezier_points(p0: Pos2, p1: Pos2, p2: Pos2, p3: Pos2, segments: usize) -> Vec<Pos2> {
    let mut points = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;

        let x = mt3 * p0.x + 3.0 * mt2 * t * p1.x + 3.0 * mt * t2 * p2.x + t3 * p3.x;
        let y = mt3 * p0.y + 3.0 * mt2 * t * p1.y + 3.0 * mt * t2 * p2.y + t3 * p3.y;

        points.push(Pos2::new(x, y));
    }
    points
}
