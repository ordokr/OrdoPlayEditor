// SPDX-License-Identifier: MIT OR Apache-2.0
//! Menu definitions, command palette, and keyboard shortcuts.

use std::collections::HashMap;

/// A command that can be executed from the command palette
#[derive(Clone)]
pub struct Command {
    /// Unique identifier for the command
    pub id: &'static str,
    /// Display name shown in the palette
    pub name: &'static str,
    /// Category for grouping (e.g., "File", "Edit", "View")
    pub category: &'static str,
    /// Keyboard shortcut (for display only)
    pub shortcut: Option<&'static str>,
    /// Description shown as hint
    pub description: Option<&'static str>,
}

impl Command {
    /// Create a new command
    pub const fn new(id: &'static str, name: &'static str, category: &'static str) -> Self {
        Self {
            id,
            name,
            category,
            shortcut: None,
            description: None,
        }
    }

    /// Add a keyboard shortcut hint
    pub const fn with_shortcut(mut self, shortcut: &'static str) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    /// Add a description
    pub const fn with_description(mut self, description: &'static str) -> Self {
        self.description = Some(description);
        self
    }

    /// Get display text for fuzzy matching
    pub fn display_text(&self) -> String {
        format!("{}: {}", self.category, self.name)
    }
}

/// Registry of all available commands
pub struct CommandRegistry {
    commands: Vec<Command>,
    by_id: HashMap<&'static str, usize>,
}

impl CommandRegistry {
    /// Create a new command registry with default commands
    pub fn new() -> Self {
        let mut registry = Self {
            commands: Vec::new(),
            by_id: HashMap::new(),
        };

        // File commands
        registry.register(Command::new("file.new", "New Scene", "File")
            .with_shortcut("Ctrl+N")
            .with_description("Create a new empty scene"));
        registry.register(Command::new("file.open", "Open Scene", "File")
            .with_shortcut("Ctrl+O")
            .with_description("Open an existing scene file"));
        registry.register(Command::new("file.save", "Save Scene", "File")
            .with_shortcut("Ctrl+S")
            .with_description("Save the current scene"));
        registry.register(Command::new("file.save_as", "Save Scene As", "File")
            .with_description("Save the current scene to a new file"));
        registry.register(Command::new("file.exit", "Exit", "File")
            .with_description("Exit the editor"));

        // Edit commands
        registry.register(Command::new("edit.undo", "Undo", "Edit")
            .with_shortcut("Ctrl+Z")
            .with_description("Undo the last action"));
        registry.register(Command::new("edit.redo", "Redo", "Edit")
            .with_shortcut("Ctrl+Y")
            .with_description("Redo the last undone action"));
        registry.register(Command::new("edit.delete", "Delete", "Edit")
            .with_shortcut("Delete")
            .with_description("Delete selected entities"));
        registry.register(Command::new("edit.duplicate", "Duplicate", "Edit")
            .with_shortcut("Ctrl+D")
            .with_description("Duplicate selected entities"));
        registry.register(Command::new("edit.select_all", "Select All", "Edit")
            .with_shortcut("Ctrl+A")
            .with_description("Select all entities"));

        // View commands
        registry.register(Command::new("view.reset_layout", "Reset Layout", "View")
            .with_description("Reset panel layout to default"));
        registry.register(Command::new("view.focus_selection", "Focus Selection", "View")
            .with_shortcut("F")
            .with_description("Focus camera on selected entities"));

        // Transform commands
        registry.register(Command::new("transform.translate", "Translate Mode", "Transform")
            .with_shortcut("W")
            .with_description("Switch to translate gizmo"));
        registry.register(Command::new("transform.rotate", "Rotate Mode", "Transform")
            .with_shortcut("E")
            .with_description("Switch to rotate gizmo"));
        registry.register(Command::new("transform.scale", "Scale Mode", "Transform")
            .with_shortcut("R")
            .with_description("Switch to scale gizmo"));
        registry.register(Command::new("transform.toggle_space", "Toggle Local/World Space", "Transform")
            .with_description("Toggle between local and world coordinate space"));
        registry.register(Command::new("transform.toggle_snap", "Toggle Grid Snap", "Transform")
            .with_description("Toggle grid snapping"));

        // Entity commands
        registry.register(Command::new("entity.create", "Create Entity", "Entity")
            .with_description("Create a new entity"));
        registry.register(Command::new("entity.rename", "Rename Entity", "Entity")
            .with_shortcut("F2")
            .with_description("Rename the selected entity"));

        // Panel commands
        registry.register(Command::new("panel.viewport", "Show Viewport", "Panel")
            .with_description("Show the viewport panel"));
        registry.register(Command::new("panel.hierarchy", "Show Hierarchy", "Panel")
            .with_description("Show the hierarchy panel"));
        registry.register(Command::new("panel.inspector", "Show Inspector", "Panel")
            .with_description("Show the inspector panel"));
        registry.register(Command::new("panel.asset_browser", "Show Asset Browser", "Panel")
            .with_description("Show the asset browser panel"));
        registry.register(Command::new("panel.console", "Show Console", "Panel")
            .with_description("Show the console panel"));
        registry.register(Command::new("panel.profiler", "Show Profiler", "Panel")
            .with_description("Show the profiler panel"));

        registry
    }

    /// Register a command
    pub fn register(&mut self, command: Command) {
        let index = self.commands.len();
        self.by_id.insert(command.id, index);
        self.commands.push(command);
    }

    /// Get a command by ID
    pub fn get(&self, id: &str) -> Option<&Command> {
        self.by_id.get(id).map(|&idx| &self.commands[idx])
    }

    /// Get all commands
    pub fn all(&self) -> &[Command] {
        &self.commands
    }

    /// Search commands with fuzzy matching
    pub fn search(&self, query: &str) -> Vec<&Command> {
        if query.is_empty() {
            return self.commands.iter().collect();
        }

        let query_lower = query.to_lowercase();
        let mut results: Vec<(&Command, i32)> = self.commands
            .iter()
            .filter_map(|cmd| {
                let score = fuzzy_score(&cmd.display_text().to_lowercase(), &query_lower);
                if score > 0 {
                    Some((cmd, score))
                } else {
                    None
                }
            })
            .collect();

        // Sort by score (higher is better)
        results.sort_by(|a, b| b.1.cmp(&a.1));
        results.into_iter().map(|(cmd, _)| cmd).collect()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Keyboard Shortcuts System
// ============================================================================

/// Modifier keys for shortcuts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers {
    /// Control key (Cmd on macOS)
    pub ctrl: bool,
    /// Shift key
    pub shift: bool,
    /// Alt key (Option on macOS)
    pub alt: bool,
}

impl Modifiers {
    /// No modifiers
    pub const NONE: Self = Self { ctrl: false, shift: false, alt: false };
    /// Control only
    pub const CTRL: Self = Self { ctrl: true, shift: false, alt: false };
    /// Shift only
    pub const SHIFT: Self = Self { ctrl: false, shift: true, alt: false };
    /// Alt only
    pub const ALT: Self = Self { ctrl: false, shift: false, alt: true };
    /// Control + Shift
    pub const CTRL_SHIFT: Self = Self { ctrl: true, shift: true, alt: false };
    /// Control + Alt
    pub const CTRL_ALT: Self = Self { ctrl: true, shift: false, alt: true };

    /// Create from egui modifiers
    pub fn from_egui(mods: &egui::Modifiers) -> Self {
        Self {
            ctrl: mods.ctrl || mods.command,
            shift: mods.shift,
            alt: mods.alt,
        }
    }

    /// Check if these modifiers match egui modifiers
    pub fn matches(&self, mods: &egui::Modifiers) -> bool {
        let ctrl_match = self.ctrl == (mods.ctrl || mods.command);
        let shift_match = self.shift == mods.shift;
        let alt_match = self.alt == mods.alt;
        ctrl_match && shift_match && alt_match
    }
}

impl std::fmt::Display for Modifiers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        write!(f, "{}", parts.join("+"))
    }
}

/// A keyboard shortcut (key + modifiers)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Shortcut {
    /// The main key
    pub key: egui::Key,
    /// Modifier keys
    pub modifiers: Modifiers,
}

impl Shortcut {
    /// Create a new shortcut with no modifiers
    pub const fn new(key: egui::Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::NONE,
        }
    }

    /// Create a shortcut with Ctrl modifier
    pub const fn ctrl(key: egui::Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::CTRL,
        }
    }

    /// Create a shortcut with Shift modifier
    pub const fn shift(key: egui::Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::SHIFT,
        }
    }

    /// Create a shortcut with Alt modifier
    pub const fn alt(key: egui::Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::ALT,
        }
    }

    /// Create a shortcut with Ctrl+Shift modifiers
    pub const fn ctrl_shift(key: egui::Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::CTRL_SHIFT,
        }
    }

    /// Create a shortcut with custom modifiers
    pub const fn with_modifiers(key: egui::Key, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }

    /// Check if this shortcut is pressed given current input
    pub fn is_pressed(&self, ctx: &egui::Context) -> bool {
        ctx.input(|i| {
            self.modifiers.matches(&i.modifiers) && i.key_pressed(self.key)
        })
    }

    /// Get display string for this shortcut
    pub fn display(&self) -> String {
        let key_name = format!("{:?}", self.key);
        if self.modifiers == Modifiers::NONE {
            key_name
        } else {
            format!("{}+{}", self.modifiers, key_name)
        }
    }

    /// Parse a shortcut from a display string (e.g., "Ctrl+S", "F2", "Ctrl+Shift+Z")
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
        if parts.is_empty() {
            return None;
        }

        let mut modifiers = Modifiers::NONE;
        let mut key_part = "";

        for (i, part) in parts.iter().enumerate() {
            let lower = part.to_lowercase();
            if lower == "ctrl" || lower == "control" || lower == "cmd" {
                modifiers.ctrl = true;
            } else if lower == "shift" {
                modifiers.shift = true;
            } else if lower == "alt" || lower == "option" {
                modifiers.alt = true;
            } else if i == parts.len() - 1 {
                // Last part is the key
                key_part = part;
            }
        }

        let key = Self::parse_key(key_part)?;
        Some(Self { key, modifiers })
    }

    /// Parse a key name to egui::Key
    fn parse_key(s: &str) -> Option<egui::Key> {
        let lower = s.to_lowercase();
        match lower.as_str() {
            // Letters
            "a" => Some(egui::Key::A),
            "b" => Some(egui::Key::B),
            "c" => Some(egui::Key::C),
            "d" => Some(egui::Key::D),
            "e" => Some(egui::Key::E),
            "f" => Some(egui::Key::F),
            "g" => Some(egui::Key::G),
            "h" => Some(egui::Key::H),
            "i" => Some(egui::Key::I),
            "j" => Some(egui::Key::J),
            "k" => Some(egui::Key::K),
            "l" => Some(egui::Key::L),
            "m" => Some(egui::Key::M),
            "n" => Some(egui::Key::N),
            "o" => Some(egui::Key::O),
            "p" => Some(egui::Key::P),
            "q" => Some(egui::Key::Q),
            "r" => Some(egui::Key::R),
            "s" => Some(egui::Key::S),
            "t" => Some(egui::Key::T),
            "u" => Some(egui::Key::U),
            "v" => Some(egui::Key::V),
            "w" => Some(egui::Key::W),
            "x" => Some(egui::Key::X),
            "y" => Some(egui::Key::Y),
            "z" => Some(egui::Key::Z),
            // Function keys
            "f1" => Some(egui::Key::F1),
            "f2" => Some(egui::Key::F2),
            "f3" => Some(egui::Key::F3),
            "f4" => Some(egui::Key::F4),
            "f5" => Some(egui::Key::F5),
            "f6" => Some(egui::Key::F6),
            "f7" => Some(egui::Key::F7),
            "f8" => Some(egui::Key::F8),
            "f9" => Some(egui::Key::F9),
            "f10" => Some(egui::Key::F10),
            "f11" => Some(egui::Key::F11),
            "f12" => Some(egui::Key::F12),
            // Special keys
            "delete" | "del" => Some(egui::Key::Delete),
            "backspace" => Some(egui::Key::Backspace),
            "enter" | "return" => Some(egui::Key::Enter),
            "escape" | "esc" => Some(egui::Key::Escape),
            "tab" => Some(egui::Key::Tab),
            "space" => Some(egui::Key::Space),
            "insert" | "ins" => Some(egui::Key::Insert),
            "home" => Some(egui::Key::Home),
            "end" => Some(egui::Key::End),
            "pageup" | "pgup" => Some(egui::Key::PageUp),
            "pagedown" | "pgdn" => Some(egui::Key::PageDown),
            // Arrow keys
            "up" | "arrowup" => Some(egui::Key::ArrowUp),
            "down" | "arrowdown" => Some(egui::Key::ArrowDown),
            "left" | "arrowleft" => Some(egui::Key::ArrowLeft),
            "right" | "arrowright" => Some(egui::Key::ArrowRight),
            // Numbers (top row)
            "0" => Some(egui::Key::Num0),
            "1" => Some(egui::Key::Num1),
            "2" => Some(egui::Key::Num2),
            "3" => Some(egui::Key::Num3),
            "4" => Some(egui::Key::Num4),
            "5" => Some(egui::Key::Num5),
            "6" => Some(egui::Key::Num6),
            "7" => Some(egui::Key::Num7),
            "8" => Some(egui::Key::Num8),
            "9" => Some(egui::Key::Num9),
            _ => None,
        }
    }
}

impl std::fmt::Display for Shortcut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}

/// A shortcut binding maps a shortcut to a command
#[derive(Debug, Clone)]
pub struct ShortcutBinding {
    /// The command ID this shortcut triggers
    pub command_id: &'static str,
    /// The keyboard shortcut
    pub shortcut: Shortcut,
    /// Whether this is a default binding (vs user-customized)
    pub is_default: bool,
    /// Context where this shortcut is active (None = global)
    pub context: Option<ShortcutContext>,
}

/// Context in which a shortcut is active
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShortcutContext {
    /// Active only when viewport is focused
    Viewport,
    /// Active only when hierarchy panel is focused
    Hierarchy,
    /// Active only when a text field is NOT focused
    NonTextInput,
}

/// Registry of keyboard shortcuts
pub struct ShortcutRegistry {
    /// All bindings, indexed by shortcut
    bindings: HashMap<Shortcut, ShortcutBinding>,
    /// Lookup from command ID to shortcuts
    command_shortcuts: HashMap<&'static str, Vec<Shortcut>>,
    /// User customizations (command_id -> new shortcut)
    customizations: HashMap<String, Option<Shortcut>>,
}

impl ShortcutRegistry {
    /// Create a new registry with default shortcuts
    pub fn new() -> Self {
        let mut registry = Self {
            bindings: HashMap::new(),
            command_shortcuts: HashMap::new(),
            customizations: HashMap::new(),
        };

        // Register default shortcuts
        registry.register_defaults();

        registry
    }

    /// Register all default shortcuts
    fn register_defaults(&mut self) {
        // File commands
        self.register("file.new", Shortcut::ctrl(egui::Key::N));
        self.register("file.open", Shortcut::ctrl(egui::Key::O));
        self.register("file.save", Shortcut::ctrl(egui::Key::S));

        // Edit commands
        self.register("edit.undo", Shortcut::ctrl(egui::Key::Z));
        self.register("edit.redo", Shortcut::ctrl(egui::Key::Y));
        // Also Ctrl+Shift+Z for redo (common alternative)
        self.register_with_context("edit.redo", Shortcut::ctrl_shift(egui::Key::Z), None);
        self.register("edit.delete", Shortcut::new(egui::Key::Delete));
        self.register("edit.duplicate", Shortcut::ctrl(egui::Key::D));
        self.register("edit.select_all", Shortcut::ctrl(egui::Key::A));

        // View commands
        self.register("view.focus_selection", Shortcut::new(egui::Key::F));

        // Transform commands (active in viewport context)
        self.register_with_context(
            "transform.translate",
            Shortcut::new(egui::Key::W),
            Some(ShortcutContext::NonTextInput),
        );
        self.register_with_context(
            "transform.rotate",
            Shortcut::new(egui::Key::E),
            Some(ShortcutContext::NonTextInput),
        );
        self.register_with_context(
            "transform.scale",
            Shortcut::new(egui::Key::R),
            Some(ShortcutContext::NonTextInput),
        );

        // Entity commands
        self.register("entity.rename", Shortcut::new(egui::Key::F2));

        // UI commands
        self.register("ui.command_palette", Shortcut::ctrl(egui::Key::P));
    }

    /// Register a shortcut for a command
    pub fn register(&mut self, command_id: &'static str, shortcut: Shortcut) {
        self.register_with_context(command_id, shortcut, None);
    }

    /// Register a shortcut with a specific context
    pub fn register_with_context(
        &mut self,
        command_id: &'static str,
        shortcut: Shortcut,
        context: Option<ShortcutContext>,
    ) {
        let binding = ShortcutBinding {
            command_id,
            shortcut,
            is_default: true,
            context,
        };

        self.bindings.insert(shortcut, binding);
        self.command_shortcuts
            .entry(command_id)
            .or_insert_with(Vec::new)
            .push(shortcut);
    }

    /// Get the command triggered by a shortcut (if any)
    pub fn get_command(&self, shortcut: &Shortcut) -> Option<&ShortcutBinding> {
        self.bindings.get(shortcut)
    }

    /// Get all shortcuts for a command
    pub fn get_shortcuts(&self, command_id: &str) -> Vec<&Shortcut> {
        self.command_shortcuts
            .get(command_id)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Get the primary (first) shortcut for a command
    pub fn get_primary_shortcut(&self, command_id: &str) -> Option<&Shortcut> {
        self.command_shortcuts
            .get(command_id)
            .and_then(|v| v.first())
    }

    /// Get display string for a command's primary shortcut
    pub fn get_shortcut_display(&self, command_id: &str) -> Option<String> {
        self.get_primary_shortcut(command_id).map(|s| s.display())
    }

    /// Check which command (if any) is triggered by current input
    pub fn check_input(&self, ctx: &egui::Context) -> Option<&'static str> {
        // Check if any text input has focus - if so, skip non-global shortcuts
        let text_has_focus = ctx.memory(|m| m.focused().is_some());

        for (shortcut, binding) in &self.bindings {
            // Skip context-specific shortcuts when in text input
            if text_has_focus {
                if let Some(ShortcutContext::NonTextInput) = binding.context {
                    continue;
                }
            }

            if shortcut.is_pressed(ctx) {
                return Some(binding.command_id);
            }
        }
        None
    }

    /// Customize a shortcut for a command
    pub fn customize(&mut self, command_id: &str, new_shortcut: Option<Shortcut>) {
        // Remove old shortcuts for this command
        if let Some(old_shortcuts) = self.command_shortcuts.get(command_id) {
            for old in old_shortcuts.clone() {
                self.bindings.remove(&old);
            }
        }

        // Store customization
        self.customizations.insert(command_id.to_string(), new_shortcut);

        // Add new shortcut if provided
        if let Some(shortcut) = new_shortcut {
            // Find the static command_id reference
            for (&static_id, _) in &self.command_shortcuts {
                if static_id == command_id {
                    let binding = ShortcutBinding {
                        command_id: static_id,
                        shortcut,
                        is_default: false,
                        context: None,
                    };
                    self.bindings.insert(shortcut, binding);
                    self.command_shortcuts.get_mut(static_id).unwrap().clear();
                    self.command_shortcuts.get_mut(static_id).unwrap().push(shortcut);
                    break;
                }
            }
        }
    }

    /// Check if a shortcut conflicts with existing bindings
    pub fn check_conflict(&self, shortcut: &Shortcut) -> Option<&'static str> {
        self.bindings.get(shortcut).map(|b| b.command_id)
    }

    /// Reset a command's shortcut to default
    pub fn reset_to_default(&mut self, command_id: &str) {
        self.customizations.remove(command_id);
        // Re-register defaults would need to track them separately
        // For now, this just removes customization
    }

    /// Reset all shortcuts to defaults
    pub fn reset_all(&mut self) {
        self.bindings.clear();
        self.command_shortcuts.clear();
        self.customizations.clear();
        self.register_defaults();
    }

    /// Get all bindings for UI display
    pub fn all_bindings(&self) -> impl Iterator<Item = &ShortcutBinding> {
        self.bindings.values()
    }

    /// Get all command IDs that have shortcuts
    pub fn commands_with_shortcuts(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.command_shortcuts.keys().copied()
    }
}

impl Default for ShortcutRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple fuzzy matching score
fn fuzzy_score(text: &str, query: &str) -> i32 {
    if query.is_empty() {
        return 1;
    }

    let text_chars: Vec<char> = text.chars().collect();
    let query_chars: Vec<char> = query.chars().collect();

    let mut score = 0;
    let mut query_idx = 0;
    let mut prev_match_idx: Option<usize> = None;

    for (i, &c) in text_chars.iter().enumerate() {
        if query_idx < query_chars.len() && c == query_chars[query_idx] {
            // Bonus for consecutive matches
            if let Some(prev) = prev_match_idx {
                if i == prev + 1 {
                    score += 5;
                }
            }
            // Bonus for matching at word boundaries
            if i == 0 || !text_chars[i - 1].is_alphanumeric() {
                score += 3;
            }
            score += 1;
            prev_match_idx = Some(i);
            query_idx += 1;
        }
    }

    // Only return positive score if all query chars were found
    if query_idx == query_chars.len() {
        score
    } else {
        0
    }
}

/// Command palette UI state
pub struct CommandPalette {
    /// Whether the palette is visible
    pub visible: bool,
    /// Current search query
    pub query: String,
    /// Currently selected index
    pub selected_index: usize,
    /// Command registry
    pub registry: CommandRegistry,
    /// Command to execute (set when user selects a command)
    pub pending_command: Option<&'static str>,
}

impl CommandPalette {
    /// Create a new command palette
    pub fn new() -> Self {
        Self {
            visible: false,
            query: String::new(),
            selected_index: 0,
            registry: CommandRegistry::new(),
            pending_command: None,
        }
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.query.clear();
            self.selected_index = 0;
        }
    }

    /// Show the palette
    pub fn show(&mut self) {
        self.visible = true;
        self.query.clear();
        self.selected_index = 0;
    }

    /// Hide the palette
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Render the command palette UI
    pub fn ui(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }

        let mut should_close = false;

        // Create a centered modal window
        egui::Window::new("Command Palette")
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 100.0])
            .fixed_size([500.0, 400.0])
            .show(ctx, |ui| {
                // Search input
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.query)
                        .hint_text("Type to search commands...")
                        .desired_width(f32::INFINITY)
                );

                // Focus the search input
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    should_close = true;
                }

                response.request_focus();

                ui.add_space(8.0);

                // Get filtered commands
                let commands = self.registry.search(&self.query);
                let max_visible = 12;

                // Handle keyboard navigation
                ui.input(|i| {
                    if i.key_pressed(egui::Key::ArrowDown) {
                        if self.selected_index < commands.len().saturating_sub(1) {
                            self.selected_index += 1;
                        }
                    }
                    if i.key_pressed(egui::Key::ArrowUp) {
                        if self.selected_index > 0 {
                            self.selected_index -= 1;
                        }
                    }
                    if i.key_pressed(egui::Key::Enter) {
                        if let Some(cmd) = commands.get(self.selected_index) {
                            self.pending_command = Some(cmd.id);
                            should_close = true;
                        }
                    }
                    if i.key_pressed(egui::Key::Escape) {
                        should_close = true;
                    }
                });

                // Clamp selected index
                if !commands.is_empty() && self.selected_index >= commands.len() {
                    self.selected_index = commands.len() - 1;
                }

                // Command list
                egui::ScrollArea::vertical()
                    .max_height(350.0)
                    .show(ui, |ui| {
                        for (i, cmd) in commands.iter().take(max_visible).enumerate() {
                            let is_selected = i == self.selected_index;

                            let response = ui.add(CommandItem {
                                command: cmd,
                                is_selected,
                            });

                            if response.clicked() {
                                self.pending_command = Some(cmd.id);
                                should_close = true;
                            }

                            if response.hovered() {
                                self.selected_index = i;
                            }
                        }

                        if commands.len() > max_visible {
                            ui.label(format!("... and {} more", commands.len() - max_visible));
                        }

                        if commands.is_empty() {
                            ui.centered_and_justified(|ui| {
                                ui.label("No matching commands");
                            });
                        }
                    });
            });

        if should_close {
            self.hide();
        }
    }

    /// Take the pending command (if any)
    pub fn take_pending_command(&mut self) -> Option<&'static str> {
        self.pending_command.take()
    }
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}

/// Custom widget for rendering a command item
struct CommandItem<'a> {
    command: &'a Command,
    is_selected: bool,
}

impl<'a> egui::Widget for CommandItem<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let desired_size = egui::vec2(ui.available_width(), 32.0);

        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);

            // Background
            let bg_color = if self.is_selected {
                ui.style().visuals.selection.bg_fill
            } else if response.hovered() {
                ui.style().visuals.widgets.hovered.bg_fill
            } else {
                egui::Color32::TRANSPARENT
            };

            ui.painter().rect_filled(rect, 4.0, bg_color);

            // Category and name
            let text_color = if self.is_selected {
                ui.style().visuals.selection.stroke.color
            } else {
                visuals.text_color()
            };

            let category_color = if self.is_selected {
                ui.style().visuals.selection.stroke.color.gamma_multiply(0.7)
            } else {
                ui.style().visuals.weak_text_color()
            };

            // Draw category
            ui.painter().text(
                egui::pos2(rect.left() + 8.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                self.command.category,
                egui::FontId::proportional(11.0),
                category_color,
            );

            // Draw name
            ui.painter().text(
                egui::pos2(rect.left() + 80.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                self.command.name,
                egui::FontId::proportional(13.0),
                text_color,
            );

            // Draw shortcut (right-aligned)
            if let Some(shortcut) = self.command.shortcut {
                ui.painter().text(
                    egui::pos2(rect.right() - 8.0, rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    shortcut,
                    egui::FontId::monospace(11.0),
                    category_color,
                );
            }
        }

        response
    }
}
