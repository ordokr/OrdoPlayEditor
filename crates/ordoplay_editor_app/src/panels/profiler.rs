// SPDX-License-Identifier: MIT OR Apache-2.0
//! Profiler panel - Performance monitoring.


use crate::state::EditorState;
use std::collections::VecDeque;

/// Profiler view mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfilerViewMode {
    /// Frame time overview
    Overview,
    /// CPU scope view
    Cpu,
    /// GPU scope view
    Gpu,
    /// CPU flame graph
    Flame,
    /// Timeline view
    Timeline,
    /// Statistics
    Stats,
}

/// A profiling scope (CPU or GPU timing)
#[derive(Debug, Clone)]
pub struct ProfileScope {
    /// Scope name
    pub name: String,
    /// Duration in milliseconds
    pub duration_ms: f32,
    /// Start time relative to frame start (ms)
    pub start_ms: f32,
    /// Depth in call stack
    pub depth: usize,
    /// Child scopes
    pub children: Vec<ProfileScope>,
}

impl ProfileScope {
    fn new(name: &str, start_ms: f32, duration_ms: f32, depth: usize) -> Self {
        Self {
            name: name.to_string(),
            duration_ms,
            start_ms,
            depth,
            children: Vec::new(),
        }
    }
}

/// A captured frame's profiling data
#[derive(Debug, Clone)]
pub struct ProfileFrame {
    /// Frame number
    pub frame_number: u64,
    /// Total frame time in ms
    pub total_ms: f32,
    /// CPU scopes
    pub cpu_scopes: Vec<ProfileScope>,
    /// GPU scopes
    pub gpu_scopes: Vec<ProfileScope>,
}

/// Capture mode for profiling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    /// Continuously record frames
    Continuous,
    /// Capture a single frame
    SingleFrame,
    /// Paused
    Paused,
}

/// The profiler panel
#[allow(dead_code)] // Intentionally kept for API completeness
pub struct ProfilerPanel {
    /// Current view mode
    pub view_mode: ProfilerViewMode,
    /// Capture mode
    pub capture_mode: CaptureMode,
    /// Frame time history
    pub frame_times: VecDeque<f32>,
    /// Captured frames with detailed data
    pub captured_frames: VecDeque<ProfileFrame>,
    /// Currently selected frame for detailed view
    pub selected_frame: Option<usize>,
    /// Max frames to keep
    pub max_frames: usize,
    /// Target frame time (for 60 FPS)
    pub target_frame_time: f32,
    /// Frame counter
    pub frame_counter: u64,
    /// Show level filter toggles
    pub show_trace: bool,
    pub show_debug: bool,
    pub show_info: bool,
}

impl ProfilerPanel {
    /// Create a new profiler panel
    pub fn new() -> Self {
        let mut panel = Self {
            view_mode: ProfilerViewMode::Overview,
            capture_mode: CaptureMode::Continuous,
            frame_times: VecDeque::new(),
            captured_frames: VecDeque::new(),
            selected_frame: None,
            max_frames: 300,
            target_frame_time: 16.67, // 60 FPS
            frame_counter: 0,
            show_trace: false,
            show_debug: true,
            show_info: true,
        };

        // Add mock frame time data
        panel.add_mock_data();
        panel
    }

    fn add_mock_data(&mut self) {
        for i in 0..300 {
            // Simulate varying frame times with occasional spikes
            let base = 16.0;
            let variation = (i as f32 * 0.1).sin() * 2.0;
            let spike = if i % 50 == 0 { 8.0 } else { 0.0 };
            let frame_time = base + variation + spike;
            self.frame_times.push_back(frame_time);

            // Create detailed frame data
            self.captured_frames.push_back(self.create_mock_frame(i as u64, frame_time));
        }
        self.frame_counter = 300;
    }

    fn create_mock_frame(&self, frame_num: u64, total_ms: f32) -> ProfileFrame {
        // Create mock CPU scopes
        let cpu_scopes = vec![
            ProfileScope {
                name: "Frame".to_string(),
                duration_ms: total_ms,
                start_ms: 0.0,
                depth: 0,
                children: vec![
                    ProfileScope::new("Update", 0.0, total_ms * 0.3, 1),
                    ProfileScope {
                        name: "Render".to_string(),
                        duration_ms: total_ms * 0.6,
                        start_ms: total_ms * 0.3,
                        depth: 1,
                        children: vec![
                            ProfileScope::new("Culling", total_ms * 0.3, total_ms * 0.1, 2),
                            ProfileScope::new("Shadow Pass", total_ms * 0.4, total_ms * 0.15, 2),
                            ProfileScope::new("Main Pass", total_ms * 0.55, total_ms * 0.25, 2),
                            ProfileScope::new("Post Process", total_ms * 0.8, total_ms * 0.1, 2),
                        ],
                    },
                    ProfileScope::new("UI", total_ms * 0.9, total_ms * 0.1, 1),
                ],
            },
        ];

        // Create mock GPU scopes
        let gpu_scopes = vec![
            ProfileScope {
                name: "GPU Frame".to_string(),
                duration_ms: total_ms * 0.8,
                start_ms: total_ms * 0.1,
                depth: 0,
                children: vec![
                    ProfileScope::new("Shadow Maps", 0.0, total_ms * 0.2, 1),
                    ProfileScope::new("G-Buffer", total_ms * 0.2, total_ms * 0.25, 1),
                    ProfileScope::new("Lighting", total_ms * 0.45, total_ms * 0.2, 1),
                    ProfileScope::new("Post FX", total_ms * 0.65, total_ms * 0.15, 1),
                ],
            },
        ];

        ProfileFrame {
            frame_number: frame_num,
            total_ms,
            cpu_scopes,
            gpu_scopes,
        }
    }

    /// Record a new frame
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn record_frame(&mut self, frame_time_ms: f32) {
        if self.capture_mode == CaptureMode::Paused {
            return;
        }

        self.frame_times.push_back(frame_time_ms);
        while self.frame_times.len() > self.max_frames {
            self.frame_times.pop_front();
        }

        let frame = self.create_mock_frame(self.frame_counter, frame_time_ms);
        self.captured_frames.push_back(frame);
        while self.captured_frames.len() > self.max_frames {
            self.captured_frames.pop_front();
        }

        self.frame_counter += 1;

        if self.capture_mode == CaptureMode::SingleFrame {
            self.capture_mode = CaptureMode::Paused;
        }
    }

    /// Export to Chrome trace format
    pub fn export_chrome_trace(&self) -> String {
        let mut events = Vec::new();

        for frame in &self.captured_frames {
            // Add CPU events
            self.add_trace_events(&mut events, &frame.cpu_scopes, "CPU", frame.frame_number);
            // Add GPU events
            self.add_trace_events(&mut events, &frame.gpu_scopes, "GPU", frame.frame_number);
        }

        format!(
            r#"{{"traceEvents": [{}]}}"#,
            events.join(",")
        )
    }

    fn add_trace_events(&self, events: &mut Vec<String>, scopes: &[ProfileScope], category: &str, frame: u64) {
        for scope in scopes {
            // Duration event (begin)
            events.push(format!(
                r#"{{"name": "{}", "cat": "{}", "ph": "B", "ts": {}, "pid": 1, "tid": {}}}"#,
                scope.name,
                category,
                (scope.start_ms * 1000.0 + frame as f32 * 20000.0) as u64,
                if category == "CPU" { 1 } else { 2 }
            ));
            // Duration event (end)
            events.push(format!(
                r#"{{"name": "{}", "cat": "{}", "ph": "E", "ts": {}, "pid": 1, "tid": {}}}"#,
                scope.name,
                category,
                ((scope.start_ms + scope.duration_ms) * 1000.0 + frame as f32 * 20000.0) as u64,
                if category == "CPU" { 1 } else { 2 }
            ));
            // Recurse into children
            self.add_trace_events(events, &scope.children, category, frame);
        }
    }

    /// Render the profiler panel
    pub fn ui(&mut self, ui: &mut egui::Ui, _state: &mut EditorState) {
        // Toolbar
        ui.horizontal(|ui| {
            // Capture controls
            let (icon, tooltip) = match self.capture_mode {
                CaptureMode::Continuous => ("\u{f04d}", "Recording - Click to pause"),
                CaptureMode::SingleFrame => ("\u{f111}", "Single frame capture"),
                CaptureMode::Paused => ("\u{f04b}", "Paused - Click to resume"),
            };

            if ui.button(icon).on_hover_text(tooltip).clicked() {
                self.capture_mode = match self.capture_mode {
                    CaptureMode::Paused => CaptureMode::Continuous,
                    CaptureMode::Continuous | CaptureMode::SingleFrame => CaptureMode::Paused,
                };
            }

            // Single frame capture button
            if ui.button("\u{f030}").on_hover_text("Capture single frame").clicked() {
                self.capture_mode = CaptureMode::SingleFrame;
            }

            if ui.button("Clear").on_hover_text("Clear captured data").clicked() {
                self.frame_times.clear();
                self.captured_frames.clear();
                self.selected_frame = None;
            }

            ui.separator();

            // View mode tabs
            for mode in [
                ProfilerViewMode::Overview,
                ProfilerViewMode::Cpu,
                ProfilerViewMode::Gpu,
                ProfilerViewMode::Flame,
                ProfilerViewMode::Timeline,
                ProfilerViewMode::Stats,
            ] {
                let name = match mode {
                    ProfilerViewMode::Overview => "Overview",
                    ProfilerViewMode::Cpu => "CPU",
                    ProfilerViewMode::Gpu => "GPU",
                    ProfilerViewMode::Flame => "Flame",
                    ProfilerViewMode::Timeline => "Timeline",
                    ProfilerViewMode::Stats => "Stats",
                };
                if ui.selectable_label(self.view_mode == mode, name).clicked() {
                    self.view_mode = mode;
                }
            }

            ui.separator();

            // Export button
            if ui.button("Export").on_hover_text("Export to Chrome trace format").clicked() {
                let trace = self.export_chrome_trace();
                // Copy to clipboard for now
                ui.output_mut(|o| o.copied_text = trace);
            }
        });

        ui.separator();

        match self.view_mode {
            ProfilerViewMode::Overview => self.overview_view(ui),
            ProfilerViewMode::Cpu => self.cpu_scope_view(ui),
            ProfilerViewMode::Gpu => self.gpu_scope_view(ui),
            ProfilerViewMode::Flame => self.flame_view(ui),
            ProfilerViewMode::Timeline => self.timeline_view(ui),
            ProfilerViewMode::Stats => self.stats_view(ui),
        }
    }

    fn overview_view(&mut self, ui: &mut egui::Ui) {
        // Frame time graph
        let available_size = ui.available_size();
        let graph_height = 150.0;

        ui.label("Frame Time (ms)");

        let (response, painter) = ui.allocate_painter(
            egui::vec2(available_size.x, graph_height),
            egui::Sense::click(),
        );
        let rect = response.rect;

        // Draw background
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(30, 30, 30));

        // Draw target line (60 FPS)
        let target_y = rect.bottom() - (self.target_frame_time / 50.0) * rect.height();
        painter.line_segment(
            [egui::pos2(rect.left(), target_y), egui::pos2(rect.right(), target_y)],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 200, 80)),
        );

        // Draw 30 FPS line
        let thirty_fps_y = rect.bottom() - (33.33 / 50.0) * rect.height();
        painter.line_segment(
            [egui::pos2(rect.left(), thirty_fps_y), egui::pos2(rect.right(), thirty_fps_y)],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 200, 80)),
        );

        // Draw frame time bars and handle click to select
        if !self.frame_times.is_empty() {
            let bar_width = rect.width() / self.frame_times.len() as f32;

            for (i, &frame_time) in self.frame_times.iter().enumerate() {
                let x = rect.left() + i as f32 * bar_width;
                let height = (frame_time / 50.0) * rect.height();
                let y = rect.bottom() - height;

                let is_selected = self.selected_frame == Some(i);
                let color = if is_selected {
                    egui::Color32::from_rgb(100, 150, 255) // Blue for selected
                } else if frame_time > 33.33 {
                    egui::Color32::from_rgb(255, 80, 80) // Red for < 30 FPS
                } else if frame_time > 16.67 {
                    egui::Color32::from_rgb(255, 200, 80) // Yellow for < 60 FPS
                } else {
                    egui::Color32::from_rgb(80, 200, 80) // Green for >= 60 FPS
                };

                painter.rect_filled(
                    egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(bar_width.max(1.0), height)),
                    0.0,
                    color,
                );
            }

            // Handle click to select frame
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let x_ratio = (pos.x - rect.left()) / rect.width();
                    let frame_idx = (x_ratio * self.frame_times.len() as f32) as usize;
                    if frame_idx < self.frame_times.len() {
                        self.selected_frame = Some(frame_idx);
                    }
                }
            }
        }

        // Labels
        painter.text(
            egui::pos2(rect.right() - 50.0, target_y - 5.0),
            egui::Align2::RIGHT_BOTTOM,
            "60 FPS",
            egui::FontId::proportional(10.0),
            egui::Color32::from_rgb(80, 200, 80),
        );
        painter.text(
            egui::pos2(rect.right() - 50.0, thirty_fps_y - 5.0),
            egui::Align2::RIGHT_BOTTOM,
            "30 FPS",
            egui::FontId::proportional(10.0),
            egui::Color32::from_rgb(200, 200, 80),
        );

        ui.add_space(10.0);

        // Statistics
        if !self.frame_times.is_empty() {
            let avg: f32 = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
            let min = self.frame_times.iter().cloned().fold(f32::MAX, f32::min);
            let max = self.frame_times.iter().cloned().fold(f32::MIN, f32::max);

            ui.horizontal(|ui| {
                ui.label(format!("Avg: {:.2} ms ({:.0} FPS)", avg, 1000.0 / avg));
                ui.separator();
                ui.label(format!("Min: {:.2} ms", min));
                ui.separator();
                ui.label(format!("Max: {:.2} ms", max));
                if let Some(idx) = self.selected_frame {
                    ui.separator();
                    ui.label(format!("Selected: Frame {} ({:.2} ms)", idx, self.frame_times[idx]));
                }
            });
        }
    }

    fn cpu_scope_view(&self, ui: &mut egui::Ui) {
        ui.label("CPU Profiling Scopes");
        ui.add_space(4.0);

        // Get the selected frame or the latest
        let frame = if let Some(idx) = self.selected_frame {
            self.captured_frames.get(idx)
        } else {
            self.captured_frames.back()
        };

        if let Some(frame) = frame {
            ui.label(format!("Frame {} - {:.2} ms total", frame.frame_number, frame.total_ms));
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                self.render_scope_tree(ui, &frame.cpu_scopes, frame.total_ms);
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No frame data captured");
            });
        }
    }

    fn gpu_scope_view(&self, ui: &mut egui::Ui) {
        ui.label("GPU Profiling Scopes");
        ui.add_space(4.0);

        // Get the selected frame or the latest
        let frame = if let Some(idx) = self.selected_frame {
            self.captured_frames.get(idx)
        } else {
            self.captured_frames.back()
        };

        if let Some(frame) = frame {
            ui.label(format!("Frame {} - GPU Time", frame.frame_number));
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                self.render_scope_tree(ui, &frame.gpu_scopes, frame.total_ms);
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No frame data captured");
            });
        }
    }

    fn render_scope_tree(&self, ui: &mut egui::Ui, scopes: &[ProfileScope], total_ms: f32) {
        for scope in scopes {
            let indent = scope.depth as f32 * 16.0;
            let percent = (scope.duration_ms / total_ms) * 100.0;

            ui.horizontal(|ui| {
                ui.add_space(indent);

                // Color based on percentage
                let color = if percent > 50.0 {
                    egui::Color32::from_rgb(255, 100, 100)
                } else if percent > 20.0 {
                    egui::Color32::from_rgb(255, 200, 100)
                } else {
                    egui::Color32::from_rgb(150, 200, 150)
                };

                // Bar showing relative time
                let bar_width = (percent / 100.0 * 200.0).max(2.0);
                let (bar_rect, _) = ui.allocate_exact_size(egui::vec2(bar_width, 16.0), egui::Sense::hover());
                ui.painter().rect_filled(bar_rect, 2.0, color);

                ui.label(format!("{}: {:.2} ms ({:.1}%)", scope.name, scope.duration_ms, percent));
            });

            // Recurse into children
            if !scope.children.is_empty() {
                self.render_scope_tree(ui, &scope.children, total_ms);
            }
        }
    }

    fn flame_view(&self, ui: &mut egui::Ui) {
        ui.label("CPU Flame Graph");

        // Get the selected frame or the latest
        let frame = if let Some(idx) = self.selected_frame {
            self.captured_frames.get(idx)
        } else {
            self.captured_frames.back()
        };

        if let Some(frame) = frame {
            ui.label(format!("Frame {} - {:.2} ms", frame.frame_number, frame.total_ms));

            let (response, painter) = ui.allocate_painter(
                egui::vec2(ui.available_width(), 250.0),
                egui::Sense::hover(),
            );
            let rect = response.rect;

            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(30, 30, 30));

            let row_height = 24.0;
            let total_ms = frame.total_ms;

            // Render flame graph from CPU scopes
            self.render_flame_scopes(&painter, rect, &frame.cpu_scopes, total_ms, row_height);
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No frame data captured");
            });
        }
    }

    fn render_flame_scopes(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        scopes: &[ProfileScope],
        total_ms: f32,
        row_height: f32,
    ) {
        let colors = [
            egui::Color32::from_rgb(230, 100, 80),
            egui::Color32::from_rgb(230, 150, 80),
            egui::Color32::from_rgb(200, 200, 80),
            egui::Color32::from_rgb(100, 200, 80),
            egui::Color32::from_rgb(80, 200, 150),
            egui::Color32::from_rgb(80, 150, 200),
        ];

        for scope in scopes {
            let x_start = rect.left() + (scope.start_ms / total_ms) * rect.width();
            let width = (scope.duration_ms / total_ms) * rect.width();
            let y = rect.top() + scope.depth as f32 * row_height;

            let color = colors[scope.depth % colors.len()];

            if width > 1.0 {
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(x_start, y),
                        egui::vec2(width, row_height - 2.0),
                    ),
                    2.0,
                    color,
                );

                // Draw label if there's enough space
                if width > 40.0 {
                    painter.text(
                        egui::pos2(x_start + 4.0, y + row_height / 2.0),
                        egui::Align2::LEFT_CENTER,
                        &scope.name,
                        egui::FontId::proportional(11.0),
                        egui::Color32::BLACK,
                    );
                }
            }

            // Recurse into children
            if !scope.children.is_empty() {
                self.render_flame_scopes(painter, rect, &scope.children, total_ms, row_height);
            }
        }
    }

    fn timeline_view(&self, ui: &mut egui::Ui) {
        ui.label("CPU/GPU Timeline");

        let frame = if let Some(idx) = self.selected_frame {
            self.captured_frames.get(idx)
        } else {
            self.captured_frames.back()
        };

        if let Some(frame) = frame {
            ui.label(format!("Frame {} - {:.2} ms", frame.frame_number, frame.total_ms));

            let (response, painter) = ui.allocate_painter(
                egui::vec2(ui.available_width(), 120.0),
                egui::Sense::hover(),
            );
            let rect = response.rect;

            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(30, 30, 30));

            let row_height = 40.0;
            let label_width = 50.0;
            let timeline_rect = egui::Rect::from_min_max(
                egui::pos2(rect.left() + label_width, rect.top()),
                rect.max,
            );

            // CPU row
            let cpu_y = rect.top() + 10.0;
            painter.text(
                egui::pos2(rect.left() + 5.0, cpu_y + row_height / 2.0),
                egui::Align2::LEFT_CENTER,
                "CPU",
                egui::FontId::proportional(12.0),
                egui::Color32::WHITE,
            );

            // Draw CPU scopes on timeline
            for scope in &frame.cpu_scopes {
                self.render_timeline_scope(
                    &painter,
                    timeline_rect,
                    scope,
                    frame.total_ms,
                    cpu_y,
                    row_height - 5.0,
                    egui::Color32::from_rgb(100, 150, 230),
                );
            }

            // GPU row
            let gpu_y = rect.top() + 10.0 + row_height;
            painter.text(
                egui::pos2(rect.left() + 5.0, gpu_y + row_height / 2.0),
                egui::Align2::LEFT_CENTER,
                "GPU",
                egui::FontId::proportional(12.0),
                egui::Color32::WHITE,
            );

            // Draw GPU scopes on timeline
            for scope in &frame.gpu_scopes {
                self.render_timeline_scope(
                    &painter,
                    timeline_rect,
                    scope,
                    frame.total_ms,
                    gpu_y,
                    row_height - 5.0,
                    egui::Color32::from_rgb(230, 150, 100),
                );
            }

            // Draw time markers
            for i in 0..=4 {
                let t = frame.total_ms * i as f32 / 4.0;
                let x = timeline_rect.left() + timeline_rect.width() * i as f32 / 4.0;
                painter.line_segment(
                    [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 60)),
                );
                painter.text(
                    egui::pos2(x, rect.bottom() - 2.0),
                    egui::Align2::CENTER_BOTTOM,
                    format!("{:.1}ms", t),
                    egui::FontId::proportional(9.0),
                    egui::Color32::from_rgb(120, 120, 120),
                );
            }
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No frame data captured");
            });
        }
    }

    fn render_timeline_scope(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        scope: &ProfileScope,
        total_ms: f32,
        y: f32,
        height: f32,
        color: egui::Color32,
    ) {
        let x_start = rect.left() + (scope.start_ms / total_ms) * rect.width();
        let width = (scope.duration_ms / total_ms) * rect.width();

        if width > 1.0 {
            painter.rect_filled(
                egui::Rect::from_min_size(egui::pos2(x_start, y), egui::vec2(width, height)),
                2.0,
                color,
            );
        }
    }

    fn stats_view(&self, ui: &mut egui::Ui) {
        egui::Grid::new("profiler_stats")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                ui.label("Metric");
                ui.label("Value");
                ui.end_row();

                ui.label("Draw Calls");
                ui.label("1,234");
                ui.end_row();

                ui.label("Triangles");
                ui.label("2.4M");
                ui.end_row();

                ui.label("Vertices");
                ui.label("1.2M");
                ui.end_row();

                ui.label("Textures");
                ui.label("89");
                ui.end_row();

                ui.label("GPU Memory");
                ui.label("1.8 GB");
                ui.end_row();

                ui.label("CPU Memory");
                ui.label("512 MB");
                ui.end_row();

                ui.label("Entities");
                ui.label("5,678");
                ui.end_row();

                ui.label("Components");
                ui.label("23,456");
                ui.end_row();
            });
    }
}

impl Default for ProfilerPanel {
    fn default() -> Self {
        Self::new()
    }
}
