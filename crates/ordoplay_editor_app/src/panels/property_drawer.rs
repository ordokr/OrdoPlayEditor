// SPDX-License-Identifier: MIT OR Apache-2.0
//! Property drawer system for reflection-based property editing.
//!
//! This module provides a trait-based approach to drawing different property types
//! in the inspector panel, supporting custom drawers for specific types.


use egui::Ui;
use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Result of drawing a property - indicates if the value was changed
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawResult {
    /// Value was not modified
    Unchanged,
    /// Value was modified, needs to be applied
    Changed,
    /// Value editing started (for undo tracking)
    EditStarted,
    /// Value editing ended (commit to undo history)
    EditEnded,
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl DrawResult {
    pub fn is_changed(&self) -> bool {
        matches!(self, DrawResult::Changed | DrawResult::EditEnded)
    }
}

/// Metadata about a property for display purposes
#[allow(dead_code)] // Intentionally kept for API completeness
#[derive(Debug, Clone, Default)]
pub struct PropertyMeta {
    /// Display name (defaults to field name)
    pub name: String,
    /// Tooltip/description
    pub tooltip: Option<String>,
    /// Category for grouping
    pub category: Option<String>,
    /// Whether this property is read-only
    pub read_only: bool,
    /// Minimum value (for numeric types)
    pub min: Option<f64>,
    /// Maximum value (for numeric types)
    pub max: Option<f64>,
    /// Step size for drag values
    pub step: Option<f64>,
    /// Suffix to display (e.g., "°" for angles, "m" for meters)
    pub suffix: Option<String>,
    /// Whether to display as angle (degrees)
    pub is_angle: bool,
    /// Whether this is a color property
    pub is_color: bool,
    /// Asset type filter (for asset references)
    pub asset_filter: Option<String>,
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl PropertyMeta {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }

    pub fn with_step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    pub fn as_angle(mut self) -> Self {
        self.is_angle = true;
        self.suffix = Some("°".to_string());
        self
    }

    pub fn as_color(mut self) -> Self {
        self.is_color = true;
        self
    }

    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }
}

/// Trait for types that can be drawn in the inspector
#[allow(dead_code)] // Intentionally kept for API completeness
pub trait PropertyDrawer {
    /// Draw the property editor UI
    /// Returns whether the value was changed
    fn draw(&mut self, ui: &mut Ui, meta: &PropertyMeta) -> DrawResult;

    /// Get the default value for reset functionality
    fn default_value(&self) -> Option<Box<dyn Any>> {
        None
    }
}

// ============================================================================
// Built-in Property Drawers
// ============================================================================

/// Draw a f32 value
#[allow(dead_code)] // Intentionally kept for API completeness
pub fn draw_f32(ui: &mut Ui, value: &mut f32, meta: &PropertyMeta) -> DrawResult {
    let mut result = DrawResult::Unchanged;

    ui.horizontal(|ui| {
        ui.label(&meta.name);

        let mut drag = egui::DragValue::new(value)
            .speed(meta.step.unwrap_or(0.1));

        if let Some(min) = meta.min {
            drag = drag.range(min as f32..=f32::MAX);
        }
        if let Some(max) = meta.max {
            drag = drag.range(f32::MIN..=max as f32);
        }
        if let (Some(min), Some(max)) = (meta.min, meta.max) {
            drag = drag.range(min as f32..=max as f32);
        }
        if let Some(suffix) = &meta.suffix {
            drag = drag.suffix(suffix.as_str());
        }

        let response = ui.add_enabled(!meta.read_only, drag);

        if response.changed() {
            result = DrawResult::Changed;
        }
        if response.drag_started() {
            result = DrawResult::EditStarted;
        }
        if response.drag_stopped() || response.lost_focus() {
            result = DrawResult::EditEnded;
        }

        if let Some(tooltip) = &meta.tooltip {
            response.on_hover_text(tooltip);
        }
    });

    result
}

/// Draw a f64 value
#[allow(dead_code)] // Intentionally kept for API completeness
pub fn draw_f64(ui: &mut Ui, value: &mut f64, meta: &PropertyMeta) -> DrawResult {
    let mut result = DrawResult::Unchanged;

    ui.horizontal(|ui| {
        ui.label(&meta.name);

        let mut drag = egui::DragValue::new(value)
            .speed(meta.step.unwrap_or(0.1));

        if let (Some(min), Some(max)) = (meta.min, meta.max) {
            drag = drag.range(min..=max);
        }
        if let Some(suffix) = &meta.suffix {
            drag = drag.suffix(suffix.as_str());
        }

        let response = ui.add_enabled(!meta.read_only, drag);

        if response.changed() {
            result = DrawResult::Changed;
        }

        if let Some(tooltip) = &meta.tooltip {
            response.on_hover_text(tooltip);
        }
    });

    result
}

/// Draw an i32 value
#[allow(dead_code)] // Intentionally kept for API completeness
pub fn draw_i32(ui: &mut Ui, value: &mut i32, meta: &PropertyMeta) -> DrawResult {
    let mut result = DrawResult::Unchanged;

    ui.horizontal(|ui| {
        ui.label(&meta.name);

        let mut drag = egui::DragValue::new(value)
            .speed(meta.step.unwrap_or(1.0));

        if let (Some(min), Some(max)) = (meta.min, meta.max) {
            drag = drag.range(min as i32..=max as i32);
        }

        let response = ui.add_enabled(!meta.read_only, drag);

        if response.changed() {
            result = DrawResult::Changed;
        }

        if let Some(tooltip) = &meta.tooltip {
            response.on_hover_text(tooltip);
        }
    });

    result
}

/// Draw a bool value
#[allow(dead_code)] // Intentionally kept for API completeness
pub fn draw_bool(ui: &mut Ui, value: &mut bool, meta: &PropertyMeta) -> DrawResult {
    let mut result = DrawResult::Unchanged;

    ui.horizontal(|ui| {
        let response = ui.add_enabled(!meta.read_only, egui::Checkbox::new(value, &meta.name));

        if response.changed() {
            result = DrawResult::Changed;
        }

        if let Some(tooltip) = &meta.tooltip {
            response.on_hover_text(tooltip);
        }
    });

    result
}

/// Draw a String value
#[allow(dead_code)] // Intentionally kept for API completeness
pub fn draw_string(ui: &mut Ui, value: &mut String, meta: &PropertyMeta) -> DrawResult {
    let mut result = DrawResult::Unchanged;

    ui.horizontal(|ui| {
        ui.label(&meta.name);

        let response = ui.add_enabled(
            !meta.read_only,
            egui::TextEdit::singleline(value).desired_width(150.0),
        );

        if response.changed() {
            result = DrawResult::Changed;
        }
        if response.lost_focus() {
            result = DrawResult::EditEnded;
        }

        if let Some(tooltip) = &meta.tooltip {
            response.on_hover_text(tooltip);
        }
    });

    result
}

/// Draw a Vec3 (as [f32; 3])
#[allow(dead_code)] // Intentionally kept for API completeness
pub fn draw_vec3(ui: &mut Ui, value: &mut [f32; 3], meta: &PropertyMeta) -> DrawResult {
    let mut result = DrawResult::Unchanged;
    let speed = meta.step.unwrap_or(0.1) as f32;

    ui.horizontal(|ui| {
        ui.label(&meta.name);

        for (i, label) in ["X", "Y", "Z"].iter().enumerate() {
            let mut drag = egui::DragValue::new(&mut value[i])
                .speed(speed)
                .prefix(format!("{}: ", label));

            if let Some(suffix) = &meta.suffix {
                drag = drag.suffix(suffix.as_str());
            }

            let response = ui.add_enabled(!meta.read_only, drag);

            if response.changed() {
                result = DrawResult::Changed;
            }
            if response.drag_started() {
                result = DrawResult::EditStarted;
            }
            if response.drag_stopped() {
                result = DrawResult::EditEnded;
            }
        }

        if let Some(tooltip) = &meta.tooltip {
            ui.label("").on_hover_text(tooltip);
        }
    });

    result
}

/// Draw a color picker (RGB as [f32; 3])
#[allow(dead_code)] // Intentionally kept for API completeness
pub fn draw_color3(ui: &mut Ui, value: &mut [f32; 3], meta: &PropertyMeta) -> DrawResult {
    let mut result = DrawResult::Unchanged;

    ui.horizontal(|ui| {
        ui.label(&meta.name);

        // Convert to egui color
        let mut color = egui::Color32::from_rgb(
            (value[0] * 255.0) as u8,
            (value[1] * 255.0) as u8,
            (value[2] * 255.0) as u8,
        );

        let response = ui.color_edit_button_srgba(&mut color);

        if response.changed() {
            value[0] = color.r() as f32 / 255.0;
            value[1] = color.g() as f32 / 255.0;
            value[2] = color.b() as f32 / 255.0;
            result = DrawResult::Changed;
        }

        // Also show RGB sliders
        ui.add_space(8.0);
        for (i, label) in ["R", "G", "B"].iter().enumerate() {
            let r = ui.add(
                egui::DragValue::new(&mut value[i])
                    .speed(0.01)
                    .range(0.0..=1.0)
                    .prefix(format!("{}: ", label)),
            );
            if r.changed() {
                result = DrawResult::Changed;
            }
        }

        if let Some(tooltip) = &meta.tooltip {
            response.on_hover_text(tooltip);
        }
    });

    result
}

/// Draw a color picker with alpha (RGBA as [f32; 4])
#[allow(dead_code)] // Intentionally kept for API completeness
pub fn draw_color4(ui: &mut Ui, value: &mut [f32; 4], meta: &PropertyMeta) -> DrawResult {
    let mut result = DrawResult::Unchanged;

    ui.horizontal(|ui| {
        ui.label(&meta.name);

        // Convert to egui color
        let mut color = egui::Color32::from_rgba_unmultiplied(
            (value[0] * 255.0) as u8,
            (value[1] * 255.0) as u8,
            (value[2] * 255.0) as u8,
            (value[3] * 255.0) as u8,
        );

        let response = ui.color_edit_button_srgba(&mut color);

        if response.changed() {
            value[0] = color.r() as f32 / 255.0;
            value[1] = color.g() as f32 / 255.0;
            value[2] = color.b() as f32 / 255.0;
            value[3] = color.a() as f32 / 255.0;
            result = DrawResult::Changed;
        }

        if let Some(tooltip) = &meta.tooltip {
            response.on_hover_text(tooltip);
        }
    });

    result
}

/// Draw an angle in degrees with a visual dial
#[allow(dead_code)] // Intentionally kept for API completeness
pub fn draw_angle(ui: &mut Ui, degrees: &mut f32, meta: &PropertyMeta) -> DrawResult {
    let mut result = DrawResult::Unchanged;

    ui.horizontal(|ui| {
        ui.label(&meta.name);

        // Drag value for precise input
        let drag = egui::DragValue::new(degrees)
            .speed(1.0)
            .suffix("°");

        let response = ui.add_enabled(!meta.read_only, drag);

        if response.changed() {
            result = DrawResult::Changed;
        }
        if response.drag_started() {
            result = DrawResult::EditStarted;
        }
        if response.drag_stopped() {
            result = DrawResult::EditEnded;
        }

        // Visual angle indicator
        let (rect, _response) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
        let painter = ui.painter();
        let center = rect.center();
        let radius = 8.0;

        // Draw circle
        painter.circle_stroke(center, radius, egui::Stroke::new(1.0, egui::Color32::GRAY));

        // Draw angle line
        let angle_rad = degrees.to_radians();
        let end = center + egui::vec2(angle_rad.cos(), -angle_rad.sin()) * radius;
        painter.line_segment([center, end], egui::Stroke::new(2.0, egui::Color32::WHITE));

        if let Some(tooltip) = &meta.tooltip {
            response.on_hover_text(tooltip);
        }
    });

    result
}

/// Draw an asset reference field
#[allow(dead_code)] // Intentionally kept for API completeness
pub fn draw_asset_ref(
    ui: &mut Ui,
    path: &mut String,
    meta: &PropertyMeta,
    _on_browse: Option<&dyn Fn() -> Option<String>>,
) -> DrawResult {
    let mut result = DrawResult::Unchanged;

    ui.horizontal(|ui| {
        ui.label(&meta.name);

        // Text field for path
        let response = ui.add_enabled(
            !meta.read_only,
            egui::TextEdit::singleline(path)
                .desired_width(120.0)
                .hint_text("None"),
        );

        if response.changed() {
            result = DrawResult::Changed;
        }

        // Browse button
        if ui.add_enabled(!meta.read_only, egui::Button::new("...")).clicked() {
            // TODO: Open asset browser popup
            // For now, just indicate the action
        }

        // Clear button
        if !path.is_empty() && ui.add_enabled(!meta.read_only, egui::Button::new("X")).clicked() {
            path.clear();
            result = DrawResult::Changed;
        }

        if let Some(tooltip) = &meta.tooltip {
            response.on_hover_text(tooltip);
        }
    });

    result
}

// ============================================================================
// Enum Drawing Support
// ============================================================================

/// Draw an enum as a dropdown
#[allow(dead_code)] // Intentionally kept for API completeness
pub fn draw_enum<T: Clone + PartialEq + std::fmt::Debug>(
    ui: &mut Ui,
    value: &mut T,
    variants: &[(T, &str)],
    meta: &PropertyMeta,
) -> DrawResult {
    let mut result = DrawResult::Unchanged;

    ui.horizontal(|ui| {
        ui.label(&meta.name);

        // Find current variant name
        let current_name = variants
            .iter()
            .find(|(v, _)| v == value)
            .map(|(_, name)| *name)
            .unwrap_or("Unknown");

        egui::ComboBox::from_id_salt(&meta.name)
            .selected_text(current_name)
            .show_ui(ui, |ui| {
                for (variant, name) in variants {
                    if ui.selectable_label(value == variant, *name).clicked() {
                        *value = variant.clone();
                        result = DrawResult::Changed;
                    }
                }
            });

        if let Some(tooltip) = &meta.tooltip {
            ui.label("").on_hover_text(tooltip);
        }
    });

    result
}

// ============================================================================
// Property Registry
// ============================================================================

/// Type-erased property drawer function
#[allow(dead_code)] // Intentionally kept for API completeness
pub type DrawerFn = Box<dyn Fn(&mut Ui, &mut dyn Any, &PropertyMeta) -> DrawResult + Send + Sync>;

/// Registry for custom property drawers
#[allow(dead_code)] // Intentionally kept for API completeness
pub struct PropertyRegistry {
    drawers: HashMap<TypeId, DrawerFn>,
}

impl Default for PropertyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)] // Intentionally kept for API completeness
impl PropertyRegistry {
    pub fn new() -> Self {
        Self {
            drawers: HashMap::new(),
        }
    }

    /// Register a custom drawer for a type
    pub fn register<T: 'static, F>(&mut self, drawer: F)
    where
        F: Fn(&mut Ui, &mut T, &PropertyMeta) -> DrawResult + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        self.drawers.insert(
            type_id,
            Box::new(move |ui, value, meta| {
                if let Some(v) = value.downcast_mut::<T>() {
                    drawer(ui, v, meta)
                } else {
                    DrawResult::Unchanged
                }
            }),
        );
    }

    /// Draw a property using the registered drawer
    pub fn draw<T: 'static>(&self, ui: &mut Ui, value: &mut T, meta: &PropertyMeta) -> DrawResult {
        let type_id = TypeId::of::<T>();
        if let Some(drawer) = self.drawers.get(&type_id) {
            drawer(ui, value, meta)
        } else {
            // Fallback: display as debug
            ui.horizontal(|ui| {
                ui.label(&meta.name);
                ui.label("(no drawer)");
            });
            DrawResult::Unchanged
        }
    }
}

// ============================================================================
// Nested Struct Support
// ============================================================================

/// Trait for types that can expose their properties for editing
#[allow(dead_code)] // Intentionally kept for API completeness
pub trait Inspectable {
    /// Draw all properties of this type
    fn inspect(&mut self, ui: &mut Ui, registry: &PropertyRegistry) -> DrawResult;

    /// Get the type name for display
    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

/// Helper to draw a collapsible section for nested structs
#[allow(dead_code)] // Intentionally kept for API completeness
pub fn draw_nested<T: Inspectable>(
    ui: &mut Ui,
    label: &str,
    value: &mut T,
    registry: &PropertyRegistry,
    default_open: bool,
) -> DrawResult {
    let mut result = DrawResult::Unchanged;

    egui::CollapsingHeader::new(label)
        .default_open(default_open)
        .show(ui, |ui| {
            ui.indent(label, |ui| {
                let inner_result = value.inspect(ui, registry);
                if inner_result.is_changed() {
                    result = inner_result;
                }
            });
        });

    result
}

// ============================================================================
// Collection Support (Vec, HashMap)
// ============================================================================

/// Draw a Vec with add/remove/reorder capabilities
#[allow(dead_code)] // Intentionally kept for API completeness
pub fn draw_vec<T: Default + Clone>(
    ui: &mut Ui,
    items: &mut Vec<T>,
    meta: &PropertyMeta,
    mut draw_item: impl FnMut(&mut Ui, usize, &mut T) -> DrawResult,
) -> DrawResult {
    let mut result = DrawResult::Unchanged;
    let mut remove_index: Option<usize> = None;
    let mut move_from: Option<usize> = None;
    let mut move_to: Option<usize> = None;

    let item_count = items.len();

    egui::CollapsingHeader::new(format!("{} ({})", meta.name, item_count))
        .default_open(false)
        .show(ui, |ui| {
            for i in 0..item_count {
                let is_last = i == item_count - 1;

                ui.horizontal(|ui| {
                    // Index label
                    ui.label(format!("[{}]", i));

                    // Draw the item
                    let item_result = draw_item(ui, i, &mut items[i]);
                    if item_result.is_changed() {
                        result = item_result;
                    }

                    // Move up/down buttons
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("X").on_hover_text("Remove").clicked() {
                            remove_index = Some(i);
                        }
                        if !is_last {
                            if ui.small_button("v").on_hover_text("Move down").clicked() {
                                move_from = Some(i);
                                move_to = Some(i + 1);
                            }
                        }
                        if i > 0 {
                            if ui.small_button("^").on_hover_text("Move up").clicked() {
                                move_from = Some(i);
                                move_to = Some(i - 1);
                            }
                        }
                    });
                });
            }

            // Add button
            ui.horizontal(|ui| {
                if ui.button("+ Add").clicked() {
                    items.push(T::default());
                    result = DrawResult::Changed;
                }
            });
        });

    // Apply deferred operations
    if let Some(i) = remove_index {
        items.remove(i);
        result = DrawResult::Changed;
    }
    if let (Some(from), Some(to)) = (move_from, move_to) {
        items.swap(from, to);
        result = DrawResult::Changed;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_meta_builder() {
        let meta = PropertyMeta::new("Test")
            .with_tooltip("A test property")
            .with_range(0.0, 100.0)
            .with_step(0.5)
            .as_angle();

        assert_eq!(meta.name, "Test");
        assert_eq!(meta.tooltip, Some("A test property".to_string()));
        assert_eq!(meta.min, Some(0.0));
        assert_eq!(meta.max, Some(100.0));
        assert_eq!(meta.step, Some(0.5));
        assert!(meta.is_angle);
    }
}
