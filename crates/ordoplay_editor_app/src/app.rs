// SPDX-License-Identifier: MIT OR Apache-2.0
//! Main editor application setup and event loop.

use crate::panel_types::PanelType;
use crate::panels::{
    AssetBrowserPanel, ConsolePanel, HierarchyPanel, InspectorPanel, ProfilerPanel, ViewportPanel,
};
use crate::state::EditorState;
use crate::viewport_renderer::ViewportRenderer;
use egui_dock::{DockArea, DockState, NodeIndex, Style, TabViewer};
use egui_wgpu::wgpu;
use ordoplay_editor_graph::graph::Graph;
use ordoplay_editor_graph::graphs::{gameplay::create_gameplay_registry, material::create_material_registry};
use ordoplay_editor_graph::node::NodeRegistry;
use ordoplay_editor_graph::ui::GraphEditorState;
use ordoplay_editor_sequencer::SequencerPanel;
use std::sync::Arc;
use thiserror::Error;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

/// Editor application errors
#[derive(Debug, Error)]
#[allow(dead_code)] // Error variants defined for future use
pub enum EditorError {
    /// Window creation failed
    #[error("Failed to create window: {0}")]
    WindowCreation(String),

    /// Renderer initialization failed
    #[error("Failed to initialize renderer: {0}")]
    RendererInit(String),

    /// Event loop error
    #[error("Event loop error: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for editor operations
pub type Result<T> = std::result::Result<T, EditorError>;

/// Tab viewer implementation for `egui_dock`
pub struct EditorTabViewer<'a> {
    state: &'a mut EditorState,
    viewport: &'a mut ViewportPanel,
    hierarchy: &'a mut HierarchyPanel,
    inspector: &'a mut InspectorPanel,
    asset_browser: &'a mut AssetBrowserPanel,
    console: &'a mut ConsolePanel,
    profiler: &'a mut ProfilerPanel,
    material_graph: &'a mut Graph,
    material_graph_state: &'a mut GraphEditorState,
    material_registry: &'a NodeRegistry,
    gameplay_graph: &'a mut Graph,
    gameplay_graph_state: &'a mut GraphEditorState,
    gameplay_registry: &'a NodeRegistry,
    sequencer_panel: &'a mut SequencerPanel,
    /// Viewport renderer (optional, for 3D rendering)
    viewport_renderer: Option<&'a mut ViewportRenderer>,
    /// Graphics device (for renderer operations)
    device: Option<&'a wgpu::Device>,
    /// Graphics queue (for renderer operations)
    queue: Option<&'a wgpu::Queue>,
    /// egui renderer (for texture registration)
    egui_renderer: Option<&'a mut egui_wgpu::Renderer>,
}

impl<'a> TabViewer for EditorTabViewer<'a> {
    type Tab = PanelType;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        format!("{} {}", tab.icon(), tab.name()).into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            PanelType::Viewport => {
                // Pass render context to viewport if available
                let has_renderer = self.viewport_renderer.is_some()
                    && self.device.is_some()
                    && self.queue.is_some()
                    && self.egui_renderer.is_some();

                if has_renderer {
                    let renderer = self.viewport_renderer.as_mut().unwrap();
                    let device = self.device.unwrap();
                    let queue = self.queue.unwrap();
                    let egui_renderer = self.egui_renderer.as_mut().unwrap();
                    self.viewport.ui_with_renderer(
                        ui,
                        self.state,
                        renderer,
                        device,
                        queue,
                        egui_renderer,
                    );
                } else {
                    // Fallback to placeholder
                    self.viewport.ui(ui, self.state);
                }
            }
            PanelType::Hierarchy => self.hierarchy.ui(ui, self.state),
            PanelType::Inspector => self.inspector.ui(ui, self.state),
            PanelType::AssetBrowser => {
                // Update thumbnail manager with graphics context if available
                if let (Some(device), Some(queue), Some(egui_renderer)) =
                    (self.device, self.queue, self.egui_renderer.as_mut())
                {
                    self.asset_browser.update_thumbnails(device, queue, egui_renderer);
                }
                self.asset_browser.ui(ui, self.state);
            }
            PanelType::Console => self.console.ui(ui, self.state),
            PanelType::Profiler => self.profiler.ui(ui, self.state),
            PanelType::MaterialGraph => {
                self.material_graph_ui(ui);
            }
            PanelType::GameplayGraph => {
                self.gameplay_graph_state
                    .ui_with_registry(ui, self.gameplay_graph, Some(self.gameplay_registry));
            }
            PanelType::Sequencer => {
                self.sequencer_panel.ui(ui);
            }
        }
    }

    fn closeable(&mut self, _tab: &mut Self::Tab) -> bool {
        true
    }

    fn on_close(&mut self, _tab: &mut Self::Tab) -> bool {
        true // Allow closing
    }
}

impl<'a> EditorTabViewer<'a> {
    fn material_graph_ui(&mut self, ui: &mut egui::Ui) {
        egui::SidePanel::right("material_preview_panel")
            .resizable(false)
            .default_width(260.0)
            .min_width(220.0)
            .show_inside(ui, |ui| {
                self.material_preview_panel(ui);
            });

        self.material_graph_state
            .ui_with_registry(ui, self.material_graph, Some(self.material_registry));
    }

    fn material_preview_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Material Preview");
        ui.add_space(4.0);

        let preview_color = self.material_preview_color();
        let preview_height = 140.0;
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), preview_height),
            egui::Sense::hover(),
        );
        ui.painter()
            .rect_filled(rect, 8.0, preview_color);
        ui.painter()
            .rect_stroke(rect, 8.0, egui::Stroke::new(1.0, egui::Color32::from_gray(60)));

        ui.add_space(6.0);
        ui.label(format!(
            "Nodes: {} | Connections: {}",
            self.material_graph.node_count(),
            self.material_graph.connection_count()
        ));
        ui.label(format!(
            "Selected Nodes: {}",
            self.material_graph_state.selected_nodes.len()
        ));

        if let Some(output_name) = self.material_output_name() {
            ui.label(format!("Output: {}", output_name));
        } else {
            ui.label("Output: None");
        }

        ui.separator();
        ui.label("Graph Settings");
        ui.checkbox(&mut self.material_graph_state.show_grid, "Show Grid");
        ui.checkbox(&mut self.material_graph_state.show_minimap, "Show Minimap");
        ui.checkbox(&mut self.material_graph_state.snap_to_grid, "Snap to Grid");
        if self.material_graph_state.snap_to_grid {
            ui.add(
                egui::DragValue::new(&mut self.material_graph_state.snap_size)
                    .range(5.0..=100.0)
                    .speed(1.0)
                    .suffix(" px"),
            );
        }

        ui.separator();
        ui.label("Selection");
        let selected_nodes: Vec<_> = self
            .material_graph_state
            .selected_nodes
            .iter()
            .filter_map(|id| self.material_graph.node(*id))
            .map(|node| node.name.clone())
            .collect();

        if selected_nodes.is_empty() {
            ui.label("No nodes selected");
        } else {
            for name in selected_nodes.iter().take(6) {
                ui.label(name);
            }
        }
    }

    fn material_output_name(&self) -> Option<String> {
        self.material_graph
            .nodes()
            .find(|node| node.node_type == "material_output" || node.node_type == "unlit_output")
            .map(|node| node.name.clone())
    }

    fn material_preview_color(&self) -> egui::Color32 {
        let output_node = self
            .material_graph
            .nodes()
            .find(|node| node.node_type == "material_output" || node.node_type == "unlit_output");

        if let Some(node) = output_node {
            let port_name = if node.node_type == "material_output" {
                "Base Color"
            } else {
                "Color"
            };

            if let Some(port) = node.inputs.iter().find(|p| p.name == port_name) {
                if let Some(ordoplay_editor_graph::port::PortValue::Color(color)) = &port.default_value {
                    let to_u8 = |v: f32| (v.clamp(0.0, 1.0) * 255.0) as u8;
                    return egui::Color32::from_rgba_unmultiplied(
                        to_u8(color[0]),
                        to_u8(color[1]),
                        to_u8(color[2]),
                        to_u8(color[3]),
                    );
                }
            }
        }

        egui::Color32::from_rgb(70, 70, 80)
    }
}

/// Graphics state for wgpu rendering
struct GraphicsState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    egui_renderer: egui_wgpu::Renderer,
}

impl GraphicsState {
    fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        // Create wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        // Create surface
        let surface = instance.create_surface(window.clone()).expect("Failed to create surface");

        // Request adapter
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("Failed to find suitable GPU adapter");

        tracing::info!("Using GPU: {}", adapter.get_info().name);

        // Request device
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("OrdoPlay Editor Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            },
            None,
        ))
        .expect("Failed to create device");

        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Create egui renderer
        let egui_renderer = egui_wgpu::Renderer::new(&device, surface_format, None, 1, false);

        Self {
            surface,
            device,
            queue,
            config,
            egui_renderer,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    #[allow(unsafe_code)] // Workaround for wgpu 23 lifetime issue with RenderPass
    fn render(
        &mut self,
        egui_ctx: &egui::Context,
        full_output: egui::FullOutput,
        window: &Window,
    ) -> std::result::Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Editor Encoder"),
        });

        // Prepare egui render
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: window.scale_factor() as f32,
        };

        let paint_jobs = egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);

        // Update textures
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(&self.device, &self.queue, *id, image_delta);
        }

        // Update buffers
        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        // wgpu 23 has a 'static lifetime bound issue with RenderPass
        // We work around this using raw pointers
        let encoder_ptr = Box::into_raw(Box::new(encoder));

        {
            // SAFETY: encoder_ptr is valid and we'll properly reclaim it after the render_pass is dropped
            let encoder_ref: &'static mut wgpu::CommandEncoder = unsafe { &mut *encoder_ptr };

            let mut render_pass = encoder_ref.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Editor Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.egui_renderer.render(&mut render_pass, &paint_jobs, &screen_descriptor);
            // render_pass is dropped here
        }

        // SAFETY: We're reclaiming the Box after render_pass is dropped
        let encoder = unsafe { Box::from_raw(encoder_ptr) };

        // Submit and present
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // Free textures
        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        Ok(())
    }
}


/// Running state of the editor
struct EditorRunning {
    window: Arc<Window>,
    graphics: GraphicsState,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    editor: EditorInner,
    viewport_renderer: ViewportRenderer,
}

/// Simple file dialog state
#[derive(Debug, Clone, PartialEq)]
enum FileDialogMode {
    None,
    Open,
    SaveAs,
}

/// Inner editor state and panels
struct EditorInner {
    state: EditorState,
    dock_state: DockState<PanelType>,
    viewport: ViewportPanel,
    hierarchy: HierarchyPanel,
    inspector: InspectorPanel,
    asset_browser: AssetBrowserPanel,
    console: ConsolePanel,
    profiler: ProfilerPanel,
    material_graph: Graph,
    material_graph_state: GraphEditorState,
    material_registry: NodeRegistry,
    gameplay_graph: Graph,
    gameplay_graph_state: GraphEditorState,
    gameplay_registry: NodeRegistry,
    sequencer_panel: SequencerPanel,
    /// Command palette
    command_palette: crate::menus::CommandPalette,
    /// Keyboard shortcut registry
    shortcuts: crate::menus::ShortcutRegistry,
    /// Editor theme
    theme: crate::theme::EditorTheme,
    /// Show theme settings window
    show_theme_settings: bool,
    /// File dialog mode
    file_dialog_mode: FileDialogMode,
    /// File dialog path input
    file_dialog_path: String,
    /// Show unsaved changes warning
    show_unsaved_warning: bool,
    /// Pending action after unsaved warning
    pending_action: Option<Box<dyn FnOnce(&mut EditorInner) + Send + Sync>>,
    /// Project settings panel
    project_settings: crate::panels::ProjectSettingsPanel,
    /// Entity clipboard for Cut/Copy/Paste operations
    clipboard: Vec<(crate::state::EntityId, crate::state::EntityData)>,
    /// Whether the app should exit (set by unsaved changes dialog)
    request_exit: bool,
}

impl EditorInner {
    fn new(tracing_rx: Option<std::sync::mpsc::Receiver<crate::panels::console::TracingEvent>>) -> Self {
        let material_registry = create_material_registry();
        let gameplay_registry = create_gameplay_registry();

        Self {
            state: EditorState::new(),
            dock_state: Self::create_default_layout(),
            viewport: ViewportPanel::new(),
            hierarchy: HierarchyPanel::new(),
            inspector: InspectorPanel::new(),
            asset_browser: AssetBrowserPanel::new(),
            console: ConsolePanel::with_tracing_receiver(tracing_rx),
            profiler: ProfilerPanel::new(),
            material_graph: Self::create_material_graph(&material_registry),
            material_graph_state: GraphEditorState::new(),
            material_registry,
            gameplay_graph: Self::create_gameplay_graph(&gameplay_registry),
            gameplay_graph_state: GraphEditorState::new(),
            gameplay_registry,
            sequencer_panel: SequencerPanel::new("Main Sequencer"),
            command_palette: crate::menus::CommandPalette::new(),
            shortcuts: crate::menus::ShortcutRegistry::new(),
            theme: crate::theme::EditorTheme::default(),
            show_theme_settings: false,
            file_dialog_mode: FileDialogMode::None,
            file_dialog_path: String::new(),
            show_unsaved_warning: false,
            pending_action: None,
            project_settings: crate::panels::ProjectSettingsPanel::new(),
            clipboard: Vec::new(),
            request_exit: false,
        }
    }

    fn create_material_graph(registry: &NodeRegistry) -> Graph {
        let mut graph = Graph::new("Material Graph");
        if let Some(node) = registry.create_node("material_output") {
            graph.add_node(node.with_position(300.0, 0.0));
        }
        graph
    }

    fn create_gameplay_graph(registry: &NodeRegistry) -> Graph {
        let mut graph = Graph::new("Gameplay Graph");
        if let Some(node) = registry.create_node("event_begin_play") {
            graph.add_node(node.with_position(0.0, 0.0));
        }
        if let Some(node) = registry.create_node("print_string") {
            graph.add_node(node.with_position(240.0, 0.0));
        }
        graph
    }

    fn create_default_layout() -> DockState<PanelType> {
        // Start with viewport in the center
        let mut dock_state = DockState::new(vec![PanelType::Viewport]);

        // Get the root surface
        let surface = dock_state.main_surface_mut();

        // Split to create left panel (Hierarchy)
        let [_center, _left] = surface.split_left(
            NodeIndex::root(),
            0.2,
            vec![PanelType::Hierarchy],
        );

        // Split to create right panel (Inspector)
        let [center, _right] = surface.split_right(
            NodeIndex::root(),
            0.75,
            vec![PanelType::Inspector],
        );

        // Split to create bottom panel (Asset Browser + Console + Profiler)
        let [_top, _bottom] = surface.split_below(
            center,
            0.7,
            vec![PanelType::AssetBrowser, PanelType::Console, PanelType::Profiler],
        );

        dock_state
    }

    fn update(
        &mut self,
        ctx: &egui::Context,
        viewport_renderer: &mut ViewportRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        egui_renderer: &mut egui_wgpu::Renderer,
    ) {
        let delta_time = ctx.input(|i| i.stable_dt);
        self.sequencer_panel.update(delta_time);

        // Update physics simulation if in play mode
        if self.state.play_mode.current_state() == crate::play_mode::PlayState::Playing {
            let fixed_timestep = self.state.project_manager.settings.physics.fixed_timestep;
            let steps = self.state.play_mode.update(delta_time as f64, fixed_timestep as f64);

            // Run fixed timestep physics updates
            for _ in 0..steps {
                self.state.physics_world.step(fixed_timestep);
            }

            // Sync physics results back to scene
            self.state.physics_world.sync_to_scene(&mut self.state.scene);

            // Update audio system
            self.state.audio_engine.update(&self.state.scene);
        }

        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                self.file_menu(ui);
                self.edit_menu(ui);
                self.view_menu(ui);
                self.tools_menu(ui);
                self.help_menu(ui);

                // Play mode controls (right-aligned)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    self.play_mode_controls(ui);
                });
            });
        });

        // Prefab editing indicator bar
        if self.state.is_editing_prefab() {
            egui::TopBottomPanel::top("prefab_edit_bar").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("\u{f1b2} Editing Prefab")
                            .color(egui::Color32::from_rgb(100, 180, 255))
                            .strong()
                    );
                    if let Some(path) = self.state.editing_prefab_path() {
                        ui.label(format!("- {}", path.display()));
                    }
                    if self.state.prefab_has_unsaved_changes() {
                        ui.label(egui::RichText::new("(modified)").color(egui::Color32::YELLOW));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Exit").clicked() {
                            if let Err(e) = self.state.exit_prefab_edit_mode(false) {
                                tracing::error!("Failed to exit prefab edit mode: {}", e);
                            }
                        }
                        if ui.button("Save & Exit").clicked() {
                            if let Err(e) = self.state.exit_prefab_edit_mode(true) {
                                tracing::error!("Failed to save and exit prefab edit mode: {}", e);
                            }
                        }
                        if ui.button("Save").clicked() {
                            if let Some(path) = self.state.editing_prefab_path().cloned() {
                                if let Err(e) = self.state.save_prefab_from_scene(&path) {
                                    tracing::error!("Failed to save prefab: {}", e);
                                }
                            }
                        }
                    });
                });
            });
        }

        // Main dock area
        let mut tab_viewer = EditorTabViewer {
            state: &mut self.state,
            viewport: &mut self.viewport,
            hierarchy: &mut self.hierarchy,
            inspector: &mut self.inspector,
            asset_browser: &mut self.asset_browser,
            console: &mut self.console,
            profiler: &mut self.profiler,
            material_graph: &mut self.material_graph,
            material_graph_state: &mut self.material_graph_state,
            material_registry: &self.material_registry,
            gameplay_graph: &mut self.gameplay_graph,
            gameplay_graph_state: &mut self.gameplay_graph_state,
            gameplay_registry: &self.gameplay_registry,
            sequencer_panel: &mut self.sequencer_panel,
            viewport_renderer: Some(viewport_renderer),
            device: Some(device),
            queue: Some(queue),
            egui_renderer: Some(egui_renderer),
        };

        DockArea::new(&mut self.dock_state)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(ctx, &mut tab_viewer);

        // Show dialogs
        self.show_file_dialog(ctx);
        self.show_unsaved_warning_dialog(ctx);
        self.show_theme_settings(ctx);
        self.project_settings.show(ctx, &mut self.state);

        // Show command palette
        self.command_palette.ui(ctx);

        // Handle pending commands from command palette
        if let Some(command_id) = self.command_palette.take_pending_command() {
            self.execute_command(command_id);
        }

        // Handle keyboard shortcuts
        self.handle_shortcuts(ctx);

        // Open any pending panels requested by other systems
        for panel in self.state.take_pending_panels() {
            self.open_panel(panel);
        }
    }

    fn show_theme_settings(&mut self, ctx: &egui::Context) {
        if !self.show_theme_settings {
            return;
        }

        let mut open = true;
        egui::Window::new("Theme Settings")
            .open(&mut open)
            .resizable(true)
            .default_width(350.0)
            .show(ctx, |ui| {
                if self.theme.settings_ui(ui) {
                    // Theme changed, apply it
                    self.theme.apply(ctx);
                }
            });

        if !open {
            self.show_theme_settings = false;
        }
    }

    fn show_file_dialog(&mut self, ctx: &egui::Context) {
        if self.file_dialog_mode == FileDialogMode::None {
            return;
        }

        let title = match self.file_dialog_mode {
            FileDialogMode::Open => "Open Scene",
            FileDialogMode::SaveAs => "Save Scene As",
            FileDialogMode::None => return,
        };

        let mut should_close = false;

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Path:");
                    ui.add(egui::TextEdit::singleline(&mut self.file_dialog_path).desired_width(300.0));
                });

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        should_close = true;
                    }

                    let action_text = match self.file_dialog_mode {
                        FileDialogMode::Open => "Open",
                        FileDialogMode::SaveAs => "Save",
                        FileDialogMode::None => "OK",
                    };

                    if ui.button(action_text).clicked() {
                        let path = std::path::PathBuf::from(&self.file_dialog_path);
                        match self.file_dialog_mode {
                            FileDialogMode::Open => {
                                if let Err(e) = self.state.load_scene(&path) {
                                    tracing::error!("Failed to load scene: {}", e);
                                }
                            }
                            FileDialogMode::SaveAs => {
                                if let Err(e) = self.state.save_scene_to_path(&path) {
                                    tracing::error!("Failed to save scene: {}", e);
                                }
                            }
                            FileDialogMode::None => {}
                        }
                        should_close = true;
                    }
                });
            });

        if should_close {
            self.file_dialog_mode = FileDialogMode::None;
        }
    }

    fn show_unsaved_warning_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_unsaved_warning {
            return;
        }

        let mut should_close = false;
        let mut proceed = false;

        egui::Window::new("Unsaved Changes")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("You have unsaved changes. Do you want to continue?");
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        should_close = true;
                    }
                    if ui.button("Don't Save").clicked() {
                        proceed = true;
                        should_close = true;
                    }
                    if ui.button("Save").clicked() {
                        if self.state.scene_path.is_some() {
                            if let Err(e) = self.state.save_scene() {
                                tracing::error!("Failed to save: {}", e);
                            } else {
                                proceed = true;
                            }
                        } else {
                            // Need to show save as dialog first
                            self.file_dialog_mode = FileDialogMode::SaveAs;
                            self.file_dialog_path = "scene.ron".to_string();
                        }
                        should_close = true;
                    }
                });
            });

        if should_close {
            self.show_unsaved_warning = false;
            if proceed {
                if let Some(action) = self.pending_action.take() {
                    action(self);
                }
            } else {
                self.pending_action = None;
            }
        }
    }

    fn file_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("File", |ui| {
            if ui.button("New Scene (Ctrl+N)").clicked() {
                if self.state.has_unsaved_changes() {
                    self.show_unsaved_warning = true;
                    self.pending_action = Some(Box::new(|editor| {
                        editor.state.new_scene();
                    }));
                } else {
                    self.state.new_scene();
                }
                ui.close_menu();
            }
            if ui.button("Open Scene... (Ctrl+O)").clicked() {
                self.file_dialog_mode = FileDialogMode::Open;
                self.file_dialog_path = String::new();
                ui.close_menu();
            }

            // Recent scenes submenu
            let has_recent = !self.state.recent_scenes.is_empty();
            ui.add_enabled_ui(has_recent, |ui| {
                ui.menu_button("Open Recent", |ui| {
                    // Clone to avoid borrow issues
                    let recent: Vec<_> = self.state.recent_scenes.iter().cloned().collect();
                    for path in recent {
                        let display_name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown");
                        let full_path = path.to_string_lossy();

                        if ui.button(display_name).on_hover_text(full_path.as_ref()).clicked() {
                            if self.state.has_unsaved_changes() {
                                let path_clone = path.clone();
                                self.show_unsaved_warning = true;
                                self.pending_action = Some(Box::new(move |editor| {
                                    if let Err(e) = editor.state.load_scene(&path_clone) {
                                        tracing::error!("Failed to load recent scene: {}", e);
                                    }
                                }));
                            } else if let Err(e) = self.state.load_scene(&path) {
                                tracing::error!("Failed to load recent scene: {}", e);
                            }
                            ui.close_menu();
                        }
                    }

                    ui.separator();
                    if ui.button("Clear Recent").clicked() {
                        self.state.clear_recent_scenes();
                        ui.close_menu();
                    }
                });
            });

            ui.separator();

            let has_path = self.state.scene_path.is_some();
            if ui.add_enabled(has_path, egui::Button::new("Save Scene (Ctrl+S)")).clicked() {
                if let Err(e) = self.state.save_scene() {
                    tracing::error!("Failed to save: {}", e);
                }
                ui.close_menu();
            }
            if ui.button("Save Scene As...").clicked() {
                self.file_dialog_mode = FileDialogMode::SaveAs;
                self.file_dialog_path = self.state.scene_path
                    .as_ref()
                    .and_then(|p| p.to_str())
                    .unwrap_or("scene.ron")
                    .to_string();
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Exit").clicked() {
                if self.state.has_unsaved_changes() {
                    self.show_unsaved_warning = true;
                    self.pending_action = Some(Box::new(|_| {
                        std::process::exit(0);
                    }));
                } else {
                    std::process::exit(0);
                }
                ui.close_menu();
            }
        });
    }

    fn edit_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Edit", |ui| {
            let can_undo = self.state.history.can_undo();
            let can_redo = self.state.history.can_redo();

            // Undo with description
            let undo_text = if let Some(desc) = self.state.history.undo_description() {
                format!("Undo: {} (Ctrl+Z)", desc)
            } else {
                "Undo (Ctrl+Z)".to_string()
            };
            if ui.add_enabled(can_undo, egui::Button::new(undo_text)).clicked() {
                if let Err(err) = self.state.undo() {
                    tracing::warn!("Undo failed: {err}");
                }
                ui.close_menu();
            }

            // Redo with description
            let redo_text = if let Some(desc) = self.state.history.redo_description() {
                format!("Redo: {} (Ctrl+Y)", desc)
            } else {
                "Redo (Ctrl+Y)".to_string()
            };
            if ui.add_enabled(can_redo, egui::Button::new(redo_text)).clicked() {
                if let Err(err) = self.state.redo() {
                    tracing::warn!("Redo failed: {err}");
                }
                ui.close_menu();
            }

            ui.separator();
            if ui.button("Cut (Ctrl+X)").clicked() {
                self.copy_selected();
                self.state.delete_selected();
                ui.close_menu();
            }
            if ui.button("Copy (Ctrl+C)").clicked() {
                self.copy_selected();
                ui.close_menu();
            }
            if ui.button("Paste (Ctrl+V)").clicked() {
                self.paste_clipboard();
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Delete (Del)").clicked() {
                self.state.delete_selected();
                ui.close_menu();
            }
            if ui.button("Duplicate (Ctrl+D)").clicked() {
                self.state.duplicate_selected();
                ui.close_menu();
            }

            ui.separator();
            if ui.button("Project Settings...").clicked() {
                self.project_settings.open = true;
                ui.close_menu();
            }
        });
    }

    fn view_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("View", |ui| {
            ui.menu_button("Panels", |ui| {
                for panel in [
                    PanelType::Viewport,
                    PanelType::Hierarchy,
                    PanelType::Inspector,
                    PanelType::AssetBrowser,
                    PanelType::Console,
                    PanelType::Profiler,
                    PanelType::MaterialGraph,
                    PanelType::GameplayGraph,
                    PanelType::Sequencer,
                ] {
                    if ui.button(panel.name()).clicked() {
                        self.open_panel(panel);
                        ui.close_menu();
                    }
                }
            });

            ui.menu_button("Theme", |ui| {
                // Theme presets
                for preset in crate::theme::ThemePreset::all() {
                    if ui.selectable_label(self.theme.preset == *preset, preset.name()).clicked() {
                        self.theme.set_preset(*preset);
                        ui.close_menu();
                    }
                }
                ui.separator();
                if ui.button("Theme Settings...").clicked() {
                    self.show_theme_settings = true;
                    ui.close_menu();
                }
            });

            ui.menu_button("Physics Debug", |ui| {
                ui.checkbox(&mut self.state.physics_debug.show_colliders, "Show Colliders").changed();
                ui.checkbox(&mut self.state.physics_debug.show_velocities, "Show Velocities").changed();
                ui.checkbox(&mut self.state.physics_debug.show_contacts, "Show Contacts").changed();
                if ui.checkbox(&mut self.state.physics_debug.show_layers, "Show Layers").changed() {
                }
            });

            ui.separator();
            if ui.button("Reset Layout").clicked() {
                self.dock_state = Self::create_default_layout();
                ui.close_menu();
            }
        });
    }

    fn tools_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Tools", |ui| {
            if ui.button("Material Editor").clicked() {
                self.open_panel(PanelType::MaterialGraph);
                ui.close_menu();
            }
            if ui.button("Gameplay Graph").clicked() {
                self.open_panel(PanelType::GameplayGraph);
                ui.close_menu();
            }
            if ui.button("Sequencer").clicked() {
                self.open_panel(PanelType::Sequencer);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Profiler").clicked() {
                self.open_panel(PanelType::Profiler);
                ui.close_menu();
            }
        });
    }

    fn help_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Help", |ui| {
            if ui.button("Documentation").clicked() {
                ui.close_menu();
            }
            if ui.button("About OrdoPlay Editor").clicked() {
                ui.close_menu();
            }
        });
    }

    fn play_mode_controls(&mut self, ui: &mut egui::Ui) {
        use crate::play_mode::PlayState;

        let play_state = self.state.play_mode.current_state();
        let is_playing = play_state == PlayState::Playing;
        let is_paused = play_state == PlayState::Paused;
        let is_stopped = play_state == PlayState::Stopped;

        // Status indicator
        let status_color = match play_state {
            PlayState::Playing => egui::Color32::from_rgb(80, 200, 120),
            PlayState::Paused => egui::Color32::from_rgb(255, 200, 80),
            PlayState::Stopped => ui.style().visuals.text_color(),
        };
        ui.label(egui::RichText::new(self.state.play_mode.status_text()).color(status_color));

        ui.separator();

        // Time scale (only show when playing or paused)
        if !is_stopped {
            let time_scale = self.state.play_mode.time_scale;
            if ui.small_button(format!("{:.1}x", time_scale))
                .on_hover_text("Time scale (click to reset)")
                .clicked()
            {
                self.state.play_mode.reset_time_scale();
            }
        }

        // Step frame button (only when paused)
        if is_paused
            && ui.small_button("\u{23ED}")  // Next frame symbol
                .on_hover_text("Step Frame")
                .clicked()
            {
                let timestep = self.state.project_manager.settings.physics.fixed_timestep as f64;
                self.state.play_mode.step_frame(timestep);
            }

        // Stop button
        if ui.add_enabled(!is_stopped, egui::Button::new("\u{25A0}"))  // Stop symbol
            .on_hover_text("Stop (Esc)")
            .clicked()
        {
            if let Some((scene, selection)) = self.state.play_mode.stop() {
                self.state.scene = scene;
                self.state.selection = selection;
                self.state.physics_world.clear();
                self.state.audio_engine.stop_all();
            }
        }

        // Pause button
        if ui.add_enabled(is_playing, egui::Button::new("\u{23F8}"))  // Pause symbol
            .on_hover_text("Pause")
            .clicked()
        {
            self.state.play_mode.pause();
            self.state.audio_engine.pause_all();
        }

        // Play button
        let play_text = "\u{25B6}";  // Play symbol
        let play_hover = if is_paused { "Resume" } else { "Play (F5)" };
        if ui.add_enabled(!is_playing, egui::Button::new(play_text))
            .on_hover_text(play_hover)
            .clicked()
        {
            if is_stopped {
                // Initialize physics world when entering play mode with project settings
                let gravity = self.state.project_manager.settings.physics.gravity;
                let collision_layers = &self.state.project_manager.settings.physics.collision_layers;
                self.state.physics_world.initialize_with_settings(&self.state.scene, gravity, collision_layers);

                // Initialize audio engine for play mode
                self.state.audio_engine.initialize_from_scene(&self.state.scene);
            } else if is_paused {
                // Resume audio when resuming from pause
                self.state.audio_engine.resume_all();
            }
            self.state.play_mode.play(&self.state.scene, &self.state.selection);
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        // Check play mode shortcuts (extract key states first to avoid borrow issues)
        let (escape_pressed, f5_pressed, f6_pressed) = ctx.input(|input| {
            (
                input.key_pressed(egui::Key::Escape),
                input.key_pressed(egui::Key::F5),
                input.key_pressed(egui::Key::F6),
            )
        });

        // Escape to stop play mode
        if escape_pressed && self.state.play_mode.current_state().is_active() {
            if let Some((scene, selection)) = self.state.play_mode.stop() {
                self.state.scene = scene;
                self.state.selection = selection;
                self.state.physics_world.clear();
                self.state.audio_engine.stop_all();
            }
        }

        // F5 to play/resume
        if f5_pressed {
            use crate::play_mode::PlayState;
            match self.state.play_mode.current_state() {
                PlayState::Stopped => {
                    // Initialize physics world when entering play mode with project settings
                    let gravity = self.state.project_manager.settings.physics.gravity;
                    let collision_layers = &self.state.project_manager.settings.physics.collision_layers;
                    self.state.physics_world.initialize_with_settings(&self.state.scene, gravity, collision_layers);

                    // Initialize audio engine for play mode
                    self.state.audio_engine.initialize_from_scene(&self.state.scene);

                    self.state.play_mode.play(&self.state.scene, &self.state.selection);
                }
                PlayState::Paused => {
                    self.state.audio_engine.resume_all();
                    self.state.play_mode.play(&self.state.scene, &self.state.selection);
                }
                PlayState::Playing => {}
            }
        }

        // F6 to pause
        if f6_pressed && self.state.play_mode.current_state() == crate::play_mode::PlayState::Playing {
            self.state.play_mode.pause();
            self.state.audio_engine.pause_all();
        }

        // Use the shortcut registry to check for triggered commands
        if let Some(command_id) = self.shortcuts.check_input(ctx) {
            // Handle special UI commands that aren't in execute_command
            if command_id == "ui.command_palette" {
                self.command_palette.toggle();
                return;
            }

            // Execute the command
            self.execute_command(command_id);
        }
    }

    /// Execute a command from the command palette
    fn execute_command(&mut self, command_id: &str) {
        match command_id {
            // File commands
            "file.new" => {
                if self.state.has_unsaved_changes() {
                    self.show_unsaved_warning = true;
                    self.pending_action = Some(Box::new(|editor| {
                        editor.state.new_scene();
                    }));
                } else {
                    self.state.new_scene();
                }
            }
            "file.open" => {
                self.file_dialog_mode = FileDialogMode::Open;
                self.file_dialog_path = String::new();
            }
            "file.save" => {
                if self.state.scene_path.is_some() {
                    let _ = self.state.save_scene();
                } else {
                    self.file_dialog_mode = FileDialogMode::SaveAs;
                    self.file_dialog_path = "scene.ron".to_string();
                }
            }
            "file.save_as" => {
                self.file_dialog_mode = FileDialogMode::SaveAs;
                self.file_dialog_path = self.state.scene_path
                    .as_ref()
                    .and_then(|p| p.to_str())
                    .unwrap_or("scene.ron")
                    .to_string();
            }
            "file.exit" => {
                if self.state.has_unsaved_changes() {
                    self.show_unsaved_warning = true;
                    self.pending_action = Some(Box::new(|_| {
                        std::process::exit(0);
                    }));
                } else {
                    std::process::exit(0);
                }
            }

            // Edit commands
            "edit.undo" => {
                if let Err(err) = self.state.undo() {
                    tracing::warn!("Undo failed: {err}");
                }
            }
            "edit.redo" => {
                if let Err(err) = self.state.redo() {
                    tracing::warn!("Redo failed: {err}");
                }
            }
            "edit.delete" => {
                self.state.delete_selected();
            }
            "edit.duplicate" => {
                self.state.duplicate_selected();
            }
            "edit.select_all" => {
                // Select all entities
                let ids: Vec<_> = self.state.scene.entities.keys().copied().collect();
                self.state.selection.clear();
                for id in ids {
                    self.state.selection.add(id);
                }
            }

            // View commands
            "view.reset_layout" => {
                self.dock_state = Self::create_default_layout();
            }
            "view.focus_selection" => {
                self.viewport.focus_on_selection(&self.state);
            }

            // Transform commands
            "transform.translate" => {
                self.state.gizmo_mode = crate::tools::GizmoMode::Translate;
            }
            "transform.rotate" => {
                self.state.gizmo_mode = crate::tools::GizmoMode::Rotate;
            }
            "transform.scale" => {
                self.state.gizmo_mode = crate::tools::GizmoMode::Scale;
            }
            "transform.toggle_space" => {
                self.state.use_world_space = !self.state.use_world_space;
            }
            "transform.toggle_snap" => {
                self.state.snap_enabled = !self.state.snap_enabled;
            }

            // Entity commands
            "entity.create" => {
                let _ = self.state.spawn_entity_with_command("New Entity", None, true);
            }
            "entity.rename" => {
                // Focus would be handled by hierarchy panel
                tracing::info!("Rename entity (F2)");
            }

            // Panel commands - these would ideally show/focus the panels
            "panel.viewport" | "panel.hierarchy" | "panel.inspector" |
            "panel.asset_browser" | "panel.console" | "panel.profiler" => {
                let panel = match command_id {
                    "panel.viewport" => PanelType::Viewport,
                    "panel.hierarchy" => PanelType::Hierarchy,
                    "panel.inspector" => PanelType::Inspector,
                    "panel.asset_browser" => PanelType::AssetBrowser,
                    "panel.console" => PanelType::Console,
                    "panel.profiler" => PanelType::Profiler,
                    _ => return,
                };
                self.open_panel(panel);
            }

            _ => {
                tracing::warn!("Unknown command: {}", command_id);
            }
        }
    }

    /// Copy selected entities to the internal clipboard
    fn copy_selected(&mut self) {
        self.clipboard.clear();
        for id in &self.state.selection.entities {
            if let Some(data) = self.state.scene.get(id) {
                self.clipboard.push((*id, data.clone()));
            }
        }
        tracing::info!("Copied {} entities to clipboard", self.clipboard.len());
    }

    /// Paste entities from the internal clipboard with new IDs
    fn paste_clipboard(&mut self) {
        if self.clipboard.is_empty() {
            return;
        }

        use crate::state::EntityId;
        use std::collections::HashMap;

        // Build a mapping from old IDs to new IDs
        let mut id_map: HashMap<EntityId, EntityId> = HashMap::new();
        for (old_id, _) in &self.clipboard {
            id_map.insert(*old_id, EntityId::new());
        }

        // Insert cloned entities with remapped IDs
        self.state.selection.clear();
        for (old_id, data) in &self.clipboard {
            let new_id = id_map[old_id];
            let mut new_data = data.clone();

            // Remap parent reference if it was also copied
            new_data.parent = new_data.parent.and_then(|p| id_map.get(&p).copied());

            // Remap children references
            new_data.children = new_data
                .children
                .iter()
                .filter_map(|c| id_map.get(c).copied())
                .collect();

            // Append " (Copy)" to name
            new_data.name = format!("{} (Copy)", new_data.name);

            self.state.scene.insert_entity(new_id, new_data);
            self.state.selection.add(new_id);
        }

        self.state.dirty = true;
        tracing::info!("Pasted {} entities from clipboard", self.clipboard.len());
    }

    fn open_panel(&mut self, panel: PanelType) {
        if let Some((surface, node, tab)) = self.dock_state.find_tab(&panel) {
            self.dock_state.set_active_tab((surface, node, tab));
            self.dock_state.set_focused_node_and_surface((surface, node));
        } else {
            self.dock_state.push_to_focused_leaf(panel);
        }
    }
}

/// Main editor application
pub struct EditorApp {
    running: Option<EditorRunning>,
    /// Tracing receiver passed to the console panel on first resume.
    tracing_rx: Option<std::sync::mpsc::Receiver<crate::panels::console::TracingEvent>>,
}

impl EditorApp {
    /// Create a new editor application
    pub fn new() -> Self {
        Self { running: None, tracing_rx: None }
    }

    /// Create a new editor application with a tracing receiver for the console.
    pub fn with_tracing_receiver(rx: std::sync::mpsc::Receiver<crate::panels::console::TracingEvent>) -> Self {
        Self { running: None, tracing_rx: Some(rx) }
    }

    /// Run the editor application with an optional tracing receiver.
    pub fn run_with_tracing_receiver(rx: Option<std::sync::mpsc::Receiver<crate::panels::console::TracingEvent>>) -> Result<()> {
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Poll);

        let mut app = match rx {
            Some(rx) => EditorApp::with_tracing_receiver(rx),
            None => EditorApp::new(),
        };
        event_loop.run_app(&mut app)?;

        Ok(())
    }
}

impl Default for EditorApp {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationHandler for EditorApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.running.is_some() {
            return;
        }

        tracing::info!("Creating editor window...");

        // Create window
        let window_attrs = Window::default_attributes()
            .with_title("OrdoPlay Editor")
            .with_inner_size(winit::dpi::LogicalSize::new(1600, 900))
            .with_min_inner_size(winit::dpi::LogicalSize::new(800, 600));

        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("Failed to create window"),
        );

        tracing::info!("Initializing graphics...");

        // Initialize graphics
        let graphics = GraphicsState::new(window.clone());

        // Create egui context
        let egui_ctx = egui::Context::default();

        // Create editor inner state
        let editor = EditorInner::new(self.tracing_rx.take());

        // Apply editor theme to egui context
        editor.theme.apply(&egui_ctx);

        // Create egui-winit state
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui_ctx.viewport_id(),
            &window,
            Some(window.scale_factor() as f32),
            None,
            Some(2 * 1024), // max texture side
        );

        // Create viewport renderer
        let initial_size = window.inner_size();
        let viewport_renderer = ViewportRenderer::new(
            &graphics.device,
            [initial_size.width.max(1), initial_size.height.max(1)],
        );

        tracing::info!("Editor initialized successfully!");
        tracing::info!("Window size: {:?}", window.inner_size());

        self.running = Some(EditorRunning {
            window,
            graphics,
            egui_ctx,
            egui_state,
            editor,
            viewport_renderer,
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        let Some(running) = &mut self.running else {
            return;
        };

        // Let egui handle the event
        let response = running.egui_state.on_window_event(&running.window, &event);

        if response.consumed {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                if running.editor.state.has_unsaved_changes() {
                    // Show unsaved changes dialog instead of exiting immediately
                    running.editor.show_unsaved_warning = true;
                    running.editor.request_exit = false;
                    running.editor.pending_action = Some(Box::new(|editor| {
                        editor.request_exit = true;
                    }));
                } else {
                    tracing::info!("Close requested, exiting...");
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(new_size) => {
                tracing::debug!("Window resized to {:?}", new_size);
                running.graphics.resize(new_size);
                running.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                // Begin egui frame
                let raw_input = running.egui_state.take_egui_input(&running.window);
                let full_output = running.egui_ctx.run(raw_input, |ctx| {
                    running.editor.update(
                        ctx,
                        &mut running.viewport_renderer,
                        &running.graphics.device,
                        &running.graphics.queue,
                        &mut running.graphics.egui_renderer,
                    );
                });

                // Check if the editor requested an exit (e.g. after unsaved changes dialog)
                if running.editor.request_exit {
                    event_loop.exit();
                    return;
                }

                // Handle platform output
                running.egui_state.handle_platform_output(&running.window, full_output.platform_output.clone());

                // Render
                match running.graphics.render(&running.egui_ctx, full_output, &running.window) {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = running.window.inner_size();
                        running.graphics.resize(size);
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        tracing::error!("Out of GPU memory!");
                        event_loop.exit();
                    }
                    Err(wgpu::SurfaceError::Timeout) => {
                        tracing::warn!("Surface timeout");
                    }
                }

                // Request another frame
                running.window.request_redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(running) = &self.running {
            running.window.request_redraw();
        }
    }
}
