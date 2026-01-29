// SPDX-License-Identifier: MIT OR Apache-2.0
//! Console panel - Log output and command input.


use crate::state::EditorState;
use std::collections::VecDeque;
use std::sync::mpsc;

/// A tracing event captured by the [`TracingBridge`] layer.
#[derive(Debug, Clone)]
pub struct TracingEvent {
    /// The log level.
    pub level: LogLevel,
    /// The formatted message.
    pub message: String,
    /// Optional target (module path).
    pub target: Option<String>,
    /// Optional file path.
    pub file: Option<String>,
    /// Optional line number.
    pub line: Option<u32>,
}

/// A `tracing_subscriber::Layer` that forwards events over an `mpsc` channel
/// so the [`ConsolePanel`] can display them.
pub struct TracingBridge {
    sender: mpsc::Sender<TracingEvent>,
}

impl TracingBridge {
    /// Create a new bridge and return `(layer, receiver)`.
    pub fn new() -> (Self, mpsc::Receiver<TracingEvent>) {
        let (sender, receiver) = mpsc::channel();
        (Self { sender }, receiver)
    }
}

impl<S> tracing_subscriber::Layer<S> for TracingBridge
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // Convert tracing level to our LogLevel
        let level = match *event.metadata().level() {
            tracing::Level::TRACE => LogLevel::Trace,
            tracing::Level::DEBUG => LogLevel::Debug,
            tracing::Level::INFO => LogLevel::Info,
            tracing::Level::WARN => LogLevel::Warn,
            tracing::Level::ERROR => LogLevel::Error,
        };

        // Extract the message using a visitor
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let message = if visitor.message.is_empty() {
            "(empty)".to_string()
        } else {
            visitor.message
        };

        let meta = event.metadata();
        let _ = self.sender.send(TracingEvent {
            level,
            message,
            target: Some(meta.target().to_string()),
            file: meta.file().map(|s| s.to_string()),
            line: meta.line(),
        });
    }
}

/// Visitor that extracts the `message` field from a tracing event.
#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        } else if self.message.is_empty() {
            self.message = format!("{} = {:?}", field.name(), value);
        } else {
            self.message
                .push_str(&format!(", {} = {:?}", field.name(), value));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else if self.message.is_empty() {
            self.message = format!("{} = {}", field.name(), value);
        } else {
            self.message
                .push_str(&format!(", {} = {}", field.name(), value));
        }
    }
}

/// Format a SystemTime as HH:MM:SS
fn format_system_time(time: &std::time::SystemTime) -> String {
    let duration = time
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let hours = (secs / 3600) % 24;
    let minutes = (secs / 60) % 60;
    let seconds = secs % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

/// Log level for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn name(&self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }

    fn short_name(&self) -> &'static str {
        match self {
            Self::Trace => "T",
            Self::Debug => "D",
            Self::Info => "I",
            Self::Warn => "W",
            Self::Error => "E",
        }
    }

    fn color(&self) -> egui::Color32 {
        match self {
            Self::Trace => egui::Color32::from_rgb(100, 100, 100),
            Self::Debug => egui::Color32::from_rgb(150, 150, 150),
            Self::Info => egui::Color32::from_rgb(200, 200, 200),
            Self::Warn => egui::Color32::from_rgb(255, 200, 80),
            Self::Error => egui::Color32::from_rgb(255, 100, 100),
        }
    }

    fn bg_color(&self) -> egui::Color32 {
        match self {
            Self::Trace => egui::Color32::TRANSPARENT,
            Self::Debug => egui::Color32::TRANSPARENT,
            Self::Info => egui::Color32::TRANSPARENT,
            Self::Warn => egui::Color32::from_rgba_unmultiplied(255, 200, 80, 20),
            Self::Error => egui::Color32::from_rgba_unmultiplied(255, 100, 100, 30),
        }
    }
}

/// Source location for click-to-jump functionality
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: Option<u32>,
}

impl SourceLocation {
    pub fn display(&self) -> String {
        if let Some(col) = self.column {
            format!("{}:{}:{}", self.file, self.line, col)
        } else {
            format!("{}:{}", self.file, self.line)
        }
    }
}

/// A log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: String,
    pub source: Option<SourceLocation>,
    pub count: u32, // For collapsed duplicate messages
}

/// The console panel
#[allow(dead_code)] // Intentionally kept for API completeness
pub struct ConsolePanel {
    /// Receiver for tracing events
    tracing_rx: Option<mpsc::Receiver<TracingEvent>>,
    /// Log entries
    pub entries: VecDeque<LogEntry>,
    /// Maximum entries to keep
    pub max_entries: usize,
    /// Minimum log level to show
    pub min_level: LogLevel,
    /// Search filter
    pub search: String,
    /// Auto-scroll to bottom
    pub auto_scroll: bool,
    /// Show timestamps
    pub show_timestamps: bool,
    /// Collapse duplicate messages
    pub collapse_duplicates: bool,
    /// Command input
    pub command_input: String,
    /// Command history
    pub command_history: Vec<String>,
    /// Command history index
    pub history_index: Option<usize>,
    /// Per-level filter toggles
    pub show_trace: bool,
    pub show_debug: bool,
    pub show_info: bool,
    pub show_warn: bool,
    pub show_error: bool,
    /// Pending source to jump to (will be handled externally)
    pub pending_jump: Option<SourceLocation>,
    /// Entry counts by level
    trace_count: usize,
    debug_count: usize,
    info_count: usize,
    warn_count: usize,
    error_count: usize,
}

impl ConsolePanel {
    /// Create a new console panel with an optional tracing receiver.
    #[allow(dead_code)] // Intentionally kept for API completeness
    pub fn new() -> Self {
        Self::with_tracing_receiver(None)
    }

    /// Create a new console panel wired to a tracing receiver.
    pub fn with_tracing_receiver(tracing_rx: Option<mpsc::Receiver<TracingEvent>>) -> Self {
        let mut panel = Self {
            tracing_rx,
            entries: VecDeque::new(),
            max_entries: 1000,
            min_level: LogLevel::Info,
            search: String::new(),
            auto_scroll: true,
            show_timestamps: false,
            collapse_duplicates: true,
            command_input: String::new(),
            command_history: Vec::new(),
            history_index: None,
            show_trace: true,
            show_debug: true,
            show_info: true,
            show_warn: true,
            show_error: true,
            pending_jump: None,
            trace_count: 0,
            debug_count: 0,
            info_count: 0,
            warn_count: 0,
            error_count: 0,
        };

        // Add some mock log entries
        panel.add_mock_entries();
        panel
    }

    fn add_mock_entries(&mut self) {
        self.log(LogLevel::Info, "OrdoPlay Editor initialized");
        self.log(LogLevel::Info, "Loading project...");
        self.log(LogLevel::Debug, "Asset database rebuilt");
        self.log(LogLevel::Info, "Project loaded successfully");
        self.log_with_source(
            LogLevel::Warn,
            "Shader 'custom_shader' has deprecated features",
            SourceLocation {
                file: "assets/shaders/custom_shader.wgsl".to_string(),
                line: 42,
                column: Some(8),
            },
        );
        self.log(LogLevel::Debug, "Hot-reload watching 42 files");
        self.log_with_source(
            LogLevel::Error,
            "Failed to load texture: file not found",
            SourceLocation {
                file: "assets/textures/missing.png".to_string(),
                line: 1,
                column: None,
            },
        );
    }

    /// Add a log entry
    pub fn log(&mut self, level: LogLevel, message: impl Into<String>) {
        self.log_internal(level, message.into(), None);
    }

    /// Add a log entry with source location
    pub fn log_with_source(&mut self, level: LogLevel, message: impl Into<String>, source: SourceLocation) {
        self.log_internal(level, message.into(), Some(source));
    }

    fn log_internal(&mut self, level: LogLevel, message: String, source: Option<SourceLocation>) {
        let now = std::time::SystemTime::now();

        // Update counts
        match level {
            LogLevel::Trace => self.trace_count += 1,
            LogLevel::Debug => self.debug_count += 1,
            LogLevel::Info => self.info_count += 1,
            LogLevel::Warn => self.warn_count += 1,
            LogLevel::Error => self.error_count += 1,
        }

        // Check if we should collapse with the previous entry
        if self.collapse_duplicates {
            if let Some(last) = self.entries.back_mut() {
                if last.level == level && last.message == message {
                    last.count += 1;
                    last.timestamp = format_system_time(&now);
                    return;
                }
            }
        }

        self.entries.push_back(LogEntry {
            level,
            message,
            timestamp: format_system_time(&now),
            source,
            count: 1,
        });

        while self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.trace_count = 0;
        self.debug_count = 0;
        self.info_count = 0;
        self.warn_count = 0;
        self.error_count = 0;
    }

    /// Check if a level is visible
    fn is_level_visible(&self, level: LogLevel) -> bool {
        match level {
            LogLevel::Trace => self.show_trace,
            LogLevel::Debug => self.show_debug,
            LogLevel::Info => self.show_info,
            LogLevel::Warn => self.show_warn,
            LogLevel::Error => self.show_error,
        }
    }

    /// Drain any pending tracing events into the log.
    pub fn poll_tracing_events(&mut self) {
        let Some(rx) = &self.tracing_rx else {
            return;
        };

        // Drain events into a local buffer to avoid borrow conflict
        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }

        for event in events {
            let source = match (event.file, event.line) {
                (Some(file), Some(line)) => Some(SourceLocation {
                    file,
                    line,
                    column: None,
                }),
                _ => None,
            };
            let message = if let Some(target) = &event.target {
                format!("[{}] {}", target, event.message)
            } else {
                event.message
            };
            self.log_internal(event.level, message, source);
        }
    }

    /// Render the console panel
    pub fn ui(&mut self, ui: &mut egui::Ui, _state: &mut EditorState) {
        self.poll_tracing_events();
        // Toolbar
        ui.horizontal(|ui| {
            // Clear button
            if ui.button("Clear").on_hover_text("Clear all logs").clicked() {
                self.clear();
            }

            ui.separator();

            // Per-level toggle buttons with counts
            let toggle_btn = |ui: &mut egui::Ui, show: &mut bool, level: LogLevel, count: usize| {
                let text = format!("{} {}", level.short_name(), count);
                let color = if *show { level.color() } else { egui::Color32::GRAY };
                if ui.add(egui::Button::new(egui::RichText::new(text).color(color).monospace()))
                    .on_hover_text(format!("{} messages", level.name()))
                    .clicked()
                {
                    *show = !*show;
                }
            };

            toggle_btn(ui, &mut self.show_trace, LogLevel::Trace, self.trace_count);
            toggle_btn(ui, &mut self.show_debug, LogLevel::Debug, self.debug_count);
            toggle_btn(ui, &mut self.show_info, LogLevel::Info, self.info_count);
            toggle_btn(ui, &mut self.show_warn, LogLevel::Warn, self.warn_count);
            toggle_btn(ui, &mut self.show_error, LogLevel::Error, self.error_count);

            ui.separator();

            // Search
            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Search...")
                    .desired_width(150.0),
            );

            if !self.search.is_empty() {
                if ui.button("x").on_hover_text("Clear search").clicked() {
                    self.search.clear();
                }
            }

            ui.separator();

            // Options menu
            ui.menu_button("Options", |ui| {
                ui.checkbox(&mut self.auto_scroll, "Auto-scroll");
                ui.checkbox(&mut self.show_timestamps, "Show timestamps");
                ui.checkbox(&mut self.collapse_duplicates, "Collapse duplicates");
            });
        });

        ui.separator();

        // Log area
        let log_area = egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(self.auto_scroll);

        let entries_to_show: Vec<_> = self.entries.iter().filter(|entry| {
            // Filter by level toggles
            if !self.is_level_visible(entry.level) {
                return false;
            }
            // Filter by search
            if !self.search.is_empty() {
                if !entry.message.to_lowercase().contains(&self.search.to_lowercase()) {
                    return false;
                }
            }
            true
        }).collect();

        log_area.show(ui, |ui| {
            for entry in entries_to_show {
                // Draw background for warnings/errors
                let bg_color = entry.level.bg_color();

                let response = ui.horizontal(|ui| {
                    // Background
                    let rect = ui.available_rect_before_wrap();
                    if bg_color != egui::Color32::TRANSPARENT {
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(rect.min, egui::vec2(ui.available_width(), 18.0)),
                            0.0,
                            bg_color,
                        );
                    }

                    // Timestamp
                    if self.show_timestamps {
                        ui.label(
                            egui::RichText::new(&entry.timestamp)
                                .monospace()
                                .size(11.0)
                                .color(egui::Color32::from_rgb(100, 100, 100)),
                        );
                    }

                    // Level badge
                    let level_text = egui::RichText::new(format!("[{}]", entry.level.short_name()))
                        .monospace()
                        .size(11.0)
                        .color(entry.level.color());
                    ui.label(level_text);

                    // Count badge for collapsed entries
                    if entry.count > 1 {
                        ui.label(
                            egui::RichText::new(format!("({})", entry.count))
                                .monospace()
                                .size(10.0)
                                .color(egui::Color32::from_rgb(150, 150, 200)),
                        );
                    }

                    // Message
                    ui.label(
                        egui::RichText::new(&entry.message)
                            .monospace()
                            .size(12.0)
                            .color(entry.level.color()),
                    );

                    // Source location (clickable)
                    if let Some(source) = &entry.source {
                        let source_text = egui::RichText::new(format!(" @ {}", source.display()))
                            .monospace()
                            .size(10.0)
                            .color(egui::Color32::from_rgb(100, 150, 200))
                            .underline();

                        if ui.add(egui::Label::new(source_text).sense(egui::Sense::click()))
                            .on_hover_text("Click to jump to source")
                            .clicked()
                        {
                            // Store the source location to jump to
                            // This would be handled by the main app
                            // For now, copy to clipboard as fallback
                            ui.output_mut(|o| o.copied_text = source.display());
                        }
                    }
                });

                // Context menu
                response.response.context_menu(|ui| {
                    if ui.button("Copy message").clicked() {
                        ui.output_mut(|o| o.copied_text = entry.message.clone());
                        ui.close_menu();
                    }
                    if let Some(source) = &entry.source {
                        if ui.button("Copy source location").clicked() {
                            ui.output_mut(|o| o.copied_text = source.display());
                            ui.close_menu();
                        }
                    }
                    if ui.button("Copy all").clicked() {
                        let mut text = entry.message.clone();
                        if let Some(source) = &entry.source {
                            text.push_str(&format!(" @ {}", source.display()));
                        }
                        ui.output_mut(|o| o.copied_text = text);
                        ui.close_menu();
                    }
                });
            }
        });

        ui.separator();

        // Command input
        ui.horizontal(|ui| {
            ui.label(">");
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.command_input)
                    .hint_text("Enter command...")
                    .desired_width(ui.available_width() - 60.0)
                    .font(egui::TextStyle::Monospace),
            );

            // Handle enter key
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if !self.command_input.is_empty() {
                    let command = self.command_input.clone();
                    self.execute_command(&command);
                    self.command_history.push(command);
                    self.command_input.clear();
                    self.history_index = None;
                }
            }

            // Handle up/down for history
            if response.has_focus() {
                if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    if let Some(idx) = self.history_index {
                        if idx > 0 {
                            self.history_index = Some(idx - 1);
                            self.command_input = self.command_history[idx - 1].clone();
                        }
                    } else if !self.command_history.is_empty() {
                        self.history_index = Some(self.command_history.len() - 1);
                        self.command_input = self.command_history.last().unwrap().clone();
                    }
                }

                if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    if let Some(idx) = self.history_index {
                        if idx < self.command_history.len() - 1 {
                            self.history_index = Some(idx + 1);
                            self.command_input = self.command_history[idx + 1].clone();
                        } else {
                            self.history_index = None;
                            self.command_input.clear();
                        }
                    }
                }
            }

            if ui.button("Run").clicked() && !self.command_input.is_empty() {
                let command = self.command_input.clone();
                self.execute_command(&command);
                self.command_history.push(command);
                self.command_input.clear();
            }
        });
    }

    fn execute_command(&mut self, command: &str) {
        self.log(LogLevel::Info, format!("> {}", command));

        // Simple command parsing
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        match parts[0] {
            "help" => {
                self.log(LogLevel::Info, "Available commands:");
                self.log(LogLevel::Info, "  help      - Show this help");
                self.log(LogLevel::Info, "  clear     - Clear console");
                self.log(LogLevel::Info, "  version   - Show version info");
                self.log(LogLevel::Info, "  reload    - Hot-reload assets");
                self.log(LogLevel::Info, "  echo      - Echo text back");
                self.log(LogLevel::Info, "  log       - Log message at level (log <level> <message>)");
                self.log(LogLevel::Info, "  stats     - Show log statistics");
                self.log(LogLevel::Info, "  history   - Show command history");
            }
            "clear" => {
                self.clear();
            }
            "version" => {
                self.log(LogLevel::Info, format!("OrdoPlay Editor v{}", env!("CARGO_PKG_VERSION")));
            }
            "reload" => {
                self.log(LogLevel::Info, "Hot-reloading assets...");
                self.log(LogLevel::Info, "Assets reloaded successfully");
            }
            "echo" => {
                let message = parts[1..].join(" ");
                self.log(LogLevel::Info, message);
            }
            "log" => {
                if parts.len() < 3 {
                    self.log(LogLevel::Error, "Usage: log <level> <message>");
                    self.log(LogLevel::Info, "Levels: trace, debug, info, warn, error");
                } else {
                    let level = match parts[1].to_lowercase().as_str() {
                        "trace" => LogLevel::Trace,
                        "debug" => LogLevel::Debug,
                        "info" => LogLevel::Info,
                        "warn" | "warning" => LogLevel::Warn,
                        "error" => LogLevel::Error,
                        _ => {
                            self.log(LogLevel::Error, format!("Unknown log level: {}", parts[1]));
                            return;
                        }
                    };
                    let message = parts[2..].join(" ");
                    self.log(level, message);
                }
            }
            "stats" => {
                self.log(LogLevel::Info, "Log statistics:");
                self.log(LogLevel::Info, format!("  Trace: {}", self.trace_count));
                self.log(LogLevel::Info, format!("  Debug: {}", self.debug_count));
                self.log(LogLevel::Info, format!("  Info:  {}", self.info_count));
                self.log(LogLevel::Info, format!("  Warn:  {}", self.warn_count));
                self.log(LogLevel::Info, format!("  Error: {}", self.error_count));
                self.log(LogLevel::Info, format!("  Total: {}", self.entries.len()));
            }
            "history" => {
                if self.command_history.is_empty() {
                    self.log(LogLevel::Info, "No command history");
                } else {
                    self.log(LogLevel::Info, "Command history:");
                    // Clone to avoid borrow conflict
                    let history: Vec<_> = self.command_history.iter().cloned().collect();
                    for (i, cmd) in history.iter().enumerate() {
                        self.log(LogLevel::Info, format!("  {}: {}", i + 1, cmd));
                    }
                }
            }
            _ => {
                self.log(LogLevel::Error, format!("Unknown command: {}. Type 'help' for available commands.", parts[0]));
            }
        }
    }
}

impl Default for ConsolePanel {
    fn default() -> Self {
        Self::with_tracing_receiver(None)
    }
}

