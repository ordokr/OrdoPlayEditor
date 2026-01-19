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

/// Tab viewer implementation for egui_dock
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
                self.material_graph_state
                    .ui_with_registry(ui, self.material_graph, Some(self.material_registry));
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
            .find(|f| f.is_srgb())
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
}

impl EditorInner {
    fn new() -> Self {
        let material_registry = create_material_registry();
        let gameplay_registry = create_gameplay_registry();

        Self {
            state: EditorState::new(),
            dock_state: Self::create_default_layout(),
            viewport: ViewportPanel::new(),
            hierarchy: HierarchyPanel::new(),
            inspector: InspectorPanel::new(),
            asset_browser: AssetBrowserPanel::new(),
            console: ConsolePanel::new(),
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
        let delta_time = ctx.input(|i| i.stable_dt) as f32;
        self.sequencer_panel.update(delta_time);

        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                self.file_menu(ui);
                self.edit_menu(ui);
                self.view_menu(ui);
                self.tools_menu(ui);
                self.help_menu(ui);
            });
        });

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
                ui.close_menu();
            }
            if ui.button("Copy (Ctrl+C)").clicked() {
                ui.close_menu();
            }
            if ui.button("Paste (Ctrl+V)").clicked() {
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
                ui.close_menu();
            }
            if ui.button("Gameplay Graph").clicked() {
                ui.close_menu();
            }
            if ui.button("Sequencer").clicked() {
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Profiler").clicked() {
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

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
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
                let id = self.state.scene.add_entity(crate::state::EntityData::new("New Entity"));
                self.state.selection.clear();
                self.state.selection.add(id);
                self.state.dirty = true;
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
}

impl EditorApp {
    /// Create a new editor application
    pub fn new() -> Self {
        Self { running: None }
    }

    /// Run the editor application
    pub fn run() -> Result<()> {
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Poll);

        let mut app = EditorApp::new();
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
        let editor = EditorInner::new();

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
                tracing::info!("Close requested, exiting...");
                event_loop.exit();
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
                    Err(e) => {
                        tracing::error!("Render error: {:?}", e);
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
