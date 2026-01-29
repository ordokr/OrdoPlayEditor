// SPDX-License-Identifier: MIT OR Apache-2.0
//! Theme system for customizable editor appearance.
//!
//! Provides dark/light base themes with customizable accent colors
//! and panel-specific styling.


use egui::{Color32, Rounding, Stroke, Style, Visuals};

/// Theme presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemePreset {
    /// Dark theme (default)
    #[default]
    Dark,
    /// Light theme
    Light,
    /// High contrast dark theme
    HighContrastDark,
    /// Custom theme
    Custom,
}

impl ThemePreset {
    /// Get all preset names for UI
    pub fn all() -> &'static [ThemePreset] {
        &[
            ThemePreset::Dark,
            ThemePreset::Light,
            ThemePreset::HighContrastDark,
            ThemePreset::Custom,
        ]
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            ThemePreset::Dark => "Dark",
            ThemePreset::Light => "Light",
            ThemePreset::HighContrastDark => "High Contrast Dark",
            ThemePreset::Custom => "Custom",
        }
    }
}

/// Accent color presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccentColor {
    /// Blue accent (default)
    #[default]
    Blue,
    /// Purple accent
    Purple,
    /// Green accent
    Green,
    /// Orange accent
    Orange,
    /// Red accent
    Red,
    /// Teal accent
    Teal,
    /// Custom RGB color
    Custom,
}

impl AccentColor {
    /// Get all accent colors for UI
    pub fn all() -> &'static [AccentColor] {
        &[
            AccentColor::Blue,
            AccentColor::Purple,
            AccentColor::Green,
            AccentColor::Orange,
            AccentColor::Red,
            AccentColor::Teal,
            AccentColor::Custom,
        ]
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            AccentColor::Blue => "Blue",
            AccentColor::Purple => "Purple",
            AccentColor::Green => "Green",
            AccentColor::Orange => "Orange",
            AccentColor::Red => "Red",
            AccentColor::Teal => "Teal",
            AccentColor::Custom => "Custom",
        }
    }

    /// Get the RGB color value
    pub fn color(&self) -> Color32 {
        match self {
            AccentColor::Blue => Color32::from_rgb(66, 133, 244),
            AccentColor::Purple => Color32::from_rgb(156, 39, 176),
            AccentColor::Green => Color32::from_rgb(76, 175, 80),
            AccentColor::Orange => Color32::from_rgb(255, 152, 0),
            AccentColor::Red => Color32::from_rgb(244, 67, 54),
            AccentColor::Teal => Color32::from_rgb(0, 150, 136),
            AccentColor::Custom => Color32::from_rgb(66, 133, 244), // Default to blue
        }
    }
}

/// Theme colors for different UI elements
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone)]
pub struct ThemeColors {
    /// Primary background color
    pub bg_primary: Color32,
    /// Secondary background (panels, cards)
    pub bg_secondary: Color32,
    /// Tertiary background (nested elements)
    pub bg_tertiary: Color32,

    /// Primary text color
    pub text_primary: Color32,
    /// Secondary text (hints, labels)
    pub text_secondary: Color32,
    /// Disabled text
    pub text_disabled: Color32,

    /// Accent color for highlights, selections
    pub accent: Color32,
    /// Accent color for hover states
    pub accent_hover: Color32,
    /// Accent color for active/pressed states
    pub accent_active: Color32,

    /// Success/positive color
    pub success: Color32,
    /// Warning color
    pub warning: Color32,
    /// Error/danger color
    pub error: Color32,
    /// Info color
    pub info: Color32,

    /// Border color
    pub border: Color32,
    /// Border color for focused elements
    pub border_focused: Color32,

    /// Selection background
    pub selection_bg: Color32,
    /// Selection text
    pub selection_text: Color32,

    /// Viewport grid color
    pub grid_color: Color32,
    /// Viewport origin axis X color
    pub axis_x: Color32,
    /// Viewport origin axis Y color
    pub axis_y: Color32,
    /// Viewport origin axis Z color
    pub axis_z: Color32,
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self::dark()
    }
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl ThemeColors {
    /// Create dark theme colors
    pub fn dark() -> Self {
        Self {
            bg_primary: Color32::from_rgb(30, 30, 30),
            bg_secondary: Color32::from_rgb(37, 37, 38),
            bg_tertiary: Color32::from_rgb(45, 45, 48),

            text_primary: Color32::from_rgb(220, 220, 220),
            text_secondary: Color32::from_rgb(150, 150, 150),
            text_disabled: Color32::from_rgb(90, 90, 90),

            accent: Color32::from_rgb(66, 133, 244),
            accent_hover: Color32::from_rgb(100, 160, 255),
            accent_active: Color32::from_rgb(40, 100, 200),

            success: Color32::from_rgb(76, 175, 80),
            warning: Color32::from_rgb(255, 193, 7),
            error: Color32::from_rgb(244, 67, 54),
            info: Color32::from_rgb(33, 150, 243),

            border: Color32::from_rgb(60, 60, 60),
            border_focused: Color32::from_rgb(66, 133, 244),

            selection_bg: Color32::from_rgba_unmultiplied(66, 133, 244, 80),
            selection_text: Color32::WHITE,

            grid_color: Color32::from_rgba_unmultiplied(255, 255, 255, 30),
            axis_x: Color32::from_rgb(255, 80, 80),
            axis_y: Color32::from_rgb(80, 255, 80),
            axis_z: Color32::from_rgb(80, 80, 255),
        }
    }

    /// Create light theme colors
    pub fn light() -> Self {
        Self {
            bg_primary: Color32::from_rgb(250, 250, 250),
            bg_secondary: Color32::from_rgb(240, 240, 240),
            bg_tertiary: Color32::from_rgb(230, 230, 230),

            text_primary: Color32::from_rgb(30, 30, 30),
            text_secondary: Color32::from_rgb(100, 100, 100),
            text_disabled: Color32::from_rgb(160, 160, 160),

            accent: Color32::from_rgb(25, 118, 210),
            accent_hover: Color32::from_rgb(66, 133, 244),
            accent_active: Color32::from_rgb(21, 101, 192),

            success: Color32::from_rgb(56, 142, 60),
            warning: Color32::from_rgb(245, 124, 0),
            error: Color32::from_rgb(211, 47, 47),
            info: Color32::from_rgb(25, 118, 210),

            border: Color32::from_rgb(200, 200, 200),
            border_focused: Color32::from_rgb(25, 118, 210),

            selection_bg: Color32::from_rgba_unmultiplied(25, 118, 210, 60),
            selection_text: Color32::BLACK,

            grid_color: Color32::from_rgba_unmultiplied(0, 0, 0, 30),
            axis_x: Color32::from_rgb(200, 50, 50),
            axis_y: Color32::from_rgb(50, 180, 50),
            axis_z: Color32::from_rgb(50, 50, 200),
        }
    }

    /// Create high contrast dark theme colors
    pub fn high_contrast_dark() -> Self {
        Self {
            bg_primary: Color32::from_rgb(0, 0, 0),
            bg_secondary: Color32::from_rgb(20, 20, 20),
            bg_tertiary: Color32::from_rgb(35, 35, 35),

            text_primary: Color32::WHITE,
            text_secondary: Color32::from_rgb(200, 200, 200),
            text_disabled: Color32::from_rgb(120, 120, 120),

            accent: Color32::from_rgb(100, 180, 255),
            accent_hover: Color32::from_rgb(150, 210, 255),
            accent_active: Color32::from_rgb(70, 140, 220),

            success: Color32::from_rgb(100, 255, 100),
            warning: Color32::from_rgb(255, 220, 0),
            error: Color32::from_rgb(255, 100, 100),
            info: Color32::from_rgb(100, 180, 255),

            border: Color32::from_rgb(100, 100, 100),
            border_focused: Color32::from_rgb(100, 180, 255),

            selection_bg: Color32::from_rgba_unmultiplied(100, 180, 255, 100),
            selection_text: Color32::WHITE,

            grid_color: Color32::from_rgba_unmultiplied(255, 255, 255, 50),
            axis_x: Color32::from_rgb(255, 100, 100),
            axis_y: Color32::from_rgb(100, 255, 100),
            axis_z: Color32::from_rgb(100, 100, 255),
        }
    }

    /// Apply an accent color to the theme
    pub fn with_accent(&mut self, accent: Color32) {
        self.accent = accent;

        // Calculate hover and active variants
        let [r, g, b, _] = accent.to_array();
        self.accent_hover = Color32::from_rgb(
            (r as u16 + 40).min(255) as u8,
            (g as u16 + 40).min(255) as u8,
            (b as u16 + 40).min(255) as u8,
        );
        self.accent_active = Color32::from_rgb(
            r.saturating_sub(30),
            g.saturating_sub(30),
            b.saturating_sub(30),
        );

        self.border_focused = accent;
        self.selection_bg = Color32::from_rgba_unmultiplied(r, g, b, 80);
    }
}

/// Complete editor theme configuration
#[derive(Debug, Clone)]
pub struct EditorTheme {
    /// Theme preset being used
    pub preset: ThemePreset,
    /// Accent color preset
    pub accent_preset: AccentColor,
    /// Custom accent color (when accent_preset is Custom)
    pub custom_accent: Color32,
    /// Theme colors
    pub colors: ThemeColors,

    /// UI scaling factor (1.0 = 100%)
    pub ui_scale: f32,
    /// Font size scaling
    pub font_scale: f32,
    /// Panel rounding
    pub panel_rounding: f32,
    /// Widget rounding
    pub widget_rounding: f32,
    /// Border width
    pub border_width: f32,
    /// Item spacing
    pub item_spacing: f32,
}

impl Default for EditorTheme {
    fn default() -> Self {
        Self {
            preset: ThemePreset::Dark,
            accent_preset: AccentColor::Blue,
            custom_accent: AccentColor::Blue.color(),
            colors: ThemeColors::dark(),
            ui_scale: 1.0,
            font_scale: 1.0,
            panel_rounding: 4.0,
            widget_rounding: 4.0,
            border_width: 1.0,
            item_spacing: 8.0,
        }
    }
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl EditorTheme {
    /// Create a new theme with a preset
    pub fn new(preset: ThemePreset) -> Self {
        let mut theme = Self::default();
        theme.set_preset(preset);
        theme
    }

    /// Set the theme preset
    pub fn set_preset(&mut self, preset: ThemePreset) {
        self.preset = preset;
        self.colors = match preset {
            ThemePreset::Dark => ThemeColors::dark(),
            ThemePreset::Light => ThemeColors::light(),
            ThemePreset::HighContrastDark => ThemeColors::high_contrast_dark(),
            ThemePreset::Custom => self.colors.clone(),
        };

        // Re-apply accent color
        self.apply_accent();
    }

    /// Set the accent color preset
    pub fn set_accent(&mut self, accent: AccentColor) {
        self.accent_preset = accent;
        if accent != AccentColor::Custom {
            self.custom_accent = accent.color();
        }
        self.apply_accent();
    }

    /// Set a custom accent color
    pub fn set_custom_accent(&mut self, color: Color32) {
        self.accent_preset = AccentColor::Custom;
        self.custom_accent = color;
        self.apply_accent();
    }

    /// Apply the current accent color to theme colors
    fn apply_accent(&mut self) {
        let accent = if self.accent_preset == AccentColor::Custom {
            self.custom_accent
        } else {
            self.accent_preset.color()
        };
        self.colors.with_accent(accent);
    }

    /// Convert to egui Style
    pub fn to_egui_style(&self) -> Style {
        let mut style = Style::default();

        // Apply visuals based on theme
        style.visuals = self.to_egui_visuals();

        // Apply spacing
        style.spacing.item_spacing = egui::vec2(self.item_spacing, self.item_spacing);
        style.spacing.button_padding = egui::vec2(8.0 * self.ui_scale, 4.0 * self.ui_scale);
        style.spacing.indent = 18.0 * self.ui_scale;

        style
    }

    /// Convert to egui Visuals
    pub fn to_egui_visuals(&self) -> Visuals {
        let colors = &self.colors;
        let is_dark = matches!(self.preset, ThemePreset::Dark | ThemePreset::HighContrastDark | ThemePreset::Custom);

        let mut visuals = if is_dark {
            Visuals::dark()
        } else {
            Visuals::light()
        };

        // Window
        visuals.window_fill = colors.bg_secondary;
        visuals.window_stroke = Stroke::new(self.border_width, colors.border);
        visuals.window_rounding = Rounding::same(self.panel_rounding);

        // Panel
        visuals.panel_fill = colors.bg_primary;

        // Widgets
        visuals.widgets.noninteractive.bg_fill = colors.bg_tertiary;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(self.border_width, colors.border);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, colors.text_primary);
        visuals.widgets.noninteractive.rounding = Rounding::same(self.widget_rounding);

        visuals.widgets.inactive.bg_fill = colors.bg_tertiary;
        visuals.widgets.inactive.bg_stroke = Stroke::new(self.border_width, colors.border);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, colors.text_primary);
        visuals.widgets.inactive.rounding = Rounding::same(self.widget_rounding);

        visuals.widgets.hovered.bg_fill = colors.accent_hover;
        visuals.widgets.hovered.bg_stroke = Stroke::new(self.border_width, colors.accent);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, colors.text_primary);
        visuals.widgets.hovered.rounding = Rounding::same(self.widget_rounding);

        visuals.widgets.active.bg_fill = colors.accent_active;
        visuals.widgets.active.bg_stroke = Stroke::new(self.border_width, colors.accent);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, colors.selection_text);
        visuals.widgets.active.rounding = Rounding::same(self.widget_rounding);

        visuals.widgets.open.bg_fill = colors.accent;
        visuals.widgets.open.bg_stroke = Stroke::new(self.border_width, colors.accent);
        visuals.widgets.open.fg_stroke = Stroke::new(1.0, colors.selection_text);
        visuals.widgets.open.rounding = Rounding::same(self.widget_rounding);

        // Selection
        visuals.selection.bg_fill = colors.selection_bg;
        visuals.selection.stroke = Stroke::new(1.0, colors.accent);

        // Text
        visuals.override_text_color = Some(colors.text_primary);

        // Hyperlinks
        visuals.hyperlink_color = colors.accent;

        // Extreme colors
        visuals.extreme_bg_color = colors.bg_primary;
        visuals.faint_bg_color = colors.bg_tertiary;

        visuals
    }

    /// Apply this theme to an egui context
    pub fn apply(&self, ctx: &egui::Context) {
        ctx.set_style(self.to_egui_style());

        // Apply font scaling if different from default
        if (self.font_scale - 1.0).abs() > 0.01 {
            ctx.set_pixels_per_point(ctx.pixels_per_point() * self.font_scale);
        }
    }

    /// Render theme settings UI
    pub fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.heading("Theme Settings");
        ui.add_space(8.0);

        // Theme preset
        ui.horizontal(|ui| {
            ui.label("Theme:");
            for preset in ThemePreset::all() {
                if ui.selectable_label(self.preset == *preset, preset.name()).clicked() {
                    self.set_preset(*preset);
                    changed = true;
                }
            }
        });

        ui.add_space(4.0);

        // Accent color
        ui.horizontal(|ui| {
            ui.label("Accent:");
            for accent in AccentColor::all() {
                let color = if *accent == AccentColor::Custom {
                    self.custom_accent
                } else {
                    accent.color()
                };

                let response = ui.add(
                    egui::Button::new("")
                        .fill(color)
                        .min_size(egui::vec2(24.0, 24.0))
                        .rounding(4.0)
                );

                if response.clicked() {
                    self.set_accent(*accent);
                    changed = true;
                }

                if self.accent_preset == *accent {
                    // Draw selection indicator
                    let rect = response.rect;
                    ui.painter().rect_stroke(
                        rect.expand(2.0),
                        4.0,
                        Stroke::new(2.0, self.colors.text_primary),
                    );
                }

                if response.hovered() {
                    response.on_hover_text(accent.name());
                }
            }
        });

        // Custom color picker when Custom is selected
        if self.accent_preset == AccentColor::Custom {
            ui.horizontal(|ui| {
                ui.label("Custom color:");
                let mut color_arr = self.custom_accent.to_array();
                if ui.color_edit_button_srgba_unmultiplied(&mut color_arr).changed() {
                    self.custom_accent = Color32::from_rgba_unmultiplied(
                        color_arr[0], color_arr[1], color_arr[2], color_arr[3]
                    );
                    self.apply_accent();
                    changed = true;
                }
            });
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // UI Scale
        ui.horizontal(|ui| {
            ui.label("UI Scale:");
            if ui.add(egui::Slider::new(&mut self.ui_scale, 0.75..=1.5).show_value(true)).changed() {
                changed = true;
            }
        });

        // Font Scale
        ui.horizontal(|ui| {
            ui.label("Font Scale:");
            if ui.add(egui::Slider::new(&mut self.font_scale, 0.8..=1.4).show_value(true)).changed() {
                changed = true;
            }
        });

        // Panel Rounding
        ui.horizontal(|ui| {
            ui.label("Panel Rounding:");
            if ui.add(egui::Slider::new(&mut self.panel_rounding, 0.0..=12.0).show_value(true)).changed() {
                changed = true;
            }
        });

        // Widget Rounding
        ui.horizontal(|ui| {
            ui.label("Widget Rounding:");
            if ui.add(egui::Slider::new(&mut self.widget_rounding, 0.0..=12.0).show_value(true)).changed() {
                changed = true;
            }
        });

        ui.add_space(8.0);

        // Reset button
        if ui.button("Reset to Defaults").clicked() {
            *self = EditorTheme::default();
            changed = true;
        }

        changed
    }
}
