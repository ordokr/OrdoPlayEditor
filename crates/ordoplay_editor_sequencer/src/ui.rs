// SPDX-License-Identifier: MIT OR Apache-2.0
//! Sequencer UI rendering.
//!
//! Features:
//! - Timeline header with time ruler
//! - Track list panel
//! - Keyframe rendering and editing
//! - Curve editor
//! - Playback controls
//! - Zoom/pan navigation

use crate::keyframe::{KeyframeId, KeyframeValue, InterpolationMode};
use crate::sequence::{Sequence, PlaybackController};
use crate::track::{Track, TrackId, TrackType};
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::HashSet;

const TRACK_HEIGHT: f32 = 28.0;
const TRACK_HEADER_WIDTH: f32 = 200.0;
const TIMELINE_HEADER_HEIGHT: f32 = 32.0;
const KEYFRAME_SIZE: f32 = 10.0;
const PLAYHEAD_WIDTH: f32 = 2.0;
const MIN_ZOOM: f32 = 20.0;
const MAX_ZOOM: f32 = 500.0;

/// View mode for the sequencer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    /// Dopesheet view (keyframes as diamonds)
    #[default]
    Dopesheet,
    /// Curve editor view
    CurveEditor,
}

/// Selection state
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct Selection {
    /// Selected tracks
    pub tracks: HashSet<TrackId>,
    /// Selected keyframes (`track_id`, `keyframe_id`)
    pub keyframes: HashSet<(TrackId, KeyframeId)>,
}


/// Drag operation state
#[derive(Debug, Clone)]
pub enum DragOperation {
    /// Not dragging
    None,
    /// Dragging playhead
    Playhead,
    /// Dragging keyframe(s)
    Keyframes {
        /// Time where drag started
        start_time: f32,
        /// Original times of all dragged keyframes
        original_times: Vec<(TrackId, KeyframeId, f32)>,
    },
    /// Box selection
    BoxSelect {
        /// Start position of box selection
        start: Pos2,
    },
    /// Panning the timeline
    Pan {
        /// Scroll offset when pan started
        start_scroll: f32,
    },
}

/// Sequencer editor state
pub struct SequencerState {
    /// Playback controller
    pub playback: PlaybackController,
    /// Horizontal zoom level (pixels per second)
    pub zoom: f32,
    /// Scroll offset (in seconds)
    pub scroll_offset: f32,
    /// Vertical scroll offset (in pixels)
    pub vertical_scroll: f32,
    /// Current view mode
    pub view_mode: ViewMode,
    /// Selection state
    pub selection: Selection,
    /// Current drag operation
    drag_op: DragOperation,
    /// Snap to grid enabled
    pub snap_enabled: bool,
    /// Grid snap interval (in seconds)
    pub snap_interval: f32,
    /// Show waveforms for audio tracks
    pub show_waveforms: bool,
    /// Auto-scroll to follow playhead
    pub auto_scroll: bool,
    /// Curve editor Y scale
    pub curve_scale: f32,
    /// Curve editor Y offset
    pub curve_offset: f32,
}

impl SequencerState {
    /// Create a new sequencer state
    pub fn new() -> Self {
        Self {
            playback: PlaybackController::new(),
            zoom: 100.0,
            scroll_offset: 0.0,
            vertical_scroll: 0.0,
            view_mode: ViewMode::Dopesheet,
            selection: Selection::default(),
            drag_op: DragOperation::None,
            snap_enabled: true,
            snap_interval: 0.1, // 100ms
            show_waveforms: true,
            auto_scroll: true,
            curve_scale: 100.0,
            curve_offset: 0.0,
        }
    }

    /// Convert time to x position
    fn time_to_x(&self, time: f32) -> f32 {
        (time - self.scroll_offset) * self.zoom + TRACK_HEADER_WIDTH
    }

    /// Convert x position to time
    fn x_to_time(&self, x: f32) -> f32 {
        (x - TRACK_HEADER_WIDTH) / self.zoom + self.scroll_offset
    }

    /// Snap time to grid if enabled
    fn snap_time(&self, time: f32) -> f32 {
        if self.snap_enabled {
            (time / self.snap_interval).round() * self.snap_interval
        } else {
            time
        }
    }

    /// Render the full sequencer UI
    pub fn ui(&mut self, ui: &mut egui::Ui, sequence: &mut Sequence) {
        let available_rect = ui.available_rect_before_wrap();

        // Toolbar
        self.render_toolbar(ui, sequence);
        ui.separator();

        // Main area
        let remaining_rect = ui.available_rect_before_wrap();

        // Calculate layout
        let timeline_rect = Rect::from_min_size(
            remaining_rect.min,
            Vec2::new(remaining_rect.width(), TIMELINE_HEADER_HEIGHT),
        );

        let content_rect = Rect::from_min_max(
            Pos2::new(remaining_rect.min.x, timeline_rect.max.y),
            remaining_rect.max,
        );

        // Render timeline header
        self.render_timeline_header(ui, timeline_rect, sequence);

        // Render tracks area
        self.render_tracks_area(ui, content_rect, sequence);

        // Handle global input
        self.handle_input(ui, remaining_rect, sequence);

        // Auto-scroll to follow playhead
        if self.auto_scroll && self.playback.is_playing() {
            let playhead_x = self.time_to_x(self.playback.time);
            let visible_end = available_rect.max.x - 50.0;
            if playhead_x > visible_end {
                self.scroll_offset = self.playback.time - (remaining_rect.width() - TRACK_HEADER_WIDTH) / self.zoom * 0.8;
            }
        }
    }

    /// Render toolbar with playback controls
    fn render_toolbar(&mut self, ui: &mut egui::Ui, sequence: &mut Sequence) {
        ui.horizontal(|ui| {
            // Playback controls
            let play_icon = if self.playback.is_playing() { "⏸" } else { "▶" };
            if ui.button(play_icon).on_hover_text("Play/Pause (Space)").clicked() {
                self.playback.toggle_playback();
            }

            if ui.button("⏹").on_hover_text("Stop").clicked() {
                self.playback.stop();
            }

            if ui.button("⏮").on_hover_text("Go to Start").clicked() {
                self.playback.seek(0.0);
            }

            if ui.button("⏭").on_hover_text("Go to End").clicked() {
                self.playback.seek(sequence.duration);
            }

            ui.separator();

            // Time display
            let time = self.playback.time;
            let minutes = (time / 60.0) as u32;
            let seconds = (time % 60.0) as u32;
            let frames = ((time % 1.0) * sequence.frame_rate) as u32;
            ui.monospace(format!("{:02}:{:02}:{:02}", minutes, seconds, frames));

            ui.separator();

            // Playback speed
            ui.label("Speed:");
            ui.add(egui::DragValue::new(&mut self.playback.speed)
                .range(0.1..=4.0)
                .speed(0.1)
                .suffix("x"));

            ui.separator();

            // View mode toggle
            ui.selectable_value(&mut self.view_mode, ViewMode::Dopesheet, "Dopesheet");
            ui.selectable_value(&mut self.view_mode, ViewMode::CurveEditor, "Curves");

            ui.separator();

            // Snap toggle
            ui.checkbox(&mut self.snap_enabled, "Snap");
            if self.snap_enabled {
                ui.add(egui::DragValue::new(&mut self.snap_interval)
                    .range(0.01..=1.0)
                    .speed(0.01)
                    .suffix("s"));
            }

            ui.separator();

            // Zoom controls
            ui.label("Zoom:");
            if ui.button("-").clicked() {
                self.zoom = (self.zoom * 0.8).max(MIN_ZOOM);
            }
            ui.add(egui::DragValue::new(&mut self.zoom)
                .range(MIN_ZOOM..=MAX_ZOOM)
                .speed(1.0));
            if ui.button("+").clicked() {
                self.zoom = (self.zoom * 1.25).min(MAX_ZOOM);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!(
                    "{} tracks | Duration: {:.2}s | {} fps",
                    sequence.track_count(),
                    sequence.duration,
                    sequence.frame_rate
                ));
            });
        });
    }

    /// Render timeline header with time ruler
    fn render_timeline_header(&mut self, ui: &mut egui::Ui, rect: Rect, sequence: &Sequence) {
        let painter = ui.painter_at(rect);

        // Background
        painter.rect_filled(rect, 0.0, Color32::from_gray(40));

        // Draw time ruler
        let visible_start = self.scroll_offset;
        let visible_end = self.scroll_offset + (rect.width() - TRACK_HEADER_WIDTH) / self.zoom;

        // Calculate tick interval based on zoom
        let tick_interval = if self.zoom > 200.0 {
            0.1
        } else if self.zoom > 100.0 {
            0.5
        } else if self.zoom > 50.0 {
            1.0
        } else {
            5.0
        };

        let major_interval = tick_interval * 5.0;

        // Draw ticks
        let first_tick = (visible_start / tick_interval).floor() * tick_interval;
        let mut time = first_tick;

        while time <= visible_end {
            let x = self.time_to_x(time);

            if x >= TRACK_HEADER_WIDTH && x <= rect.max.x {
                let is_major = (time / major_interval).fract().abs() < 0.001;

                let tick_height = if is_major { 12.0 } else { 6.0 };
                let tick_color = if is_major {
                    Color32::from_gray(180)
                } else {
                    Color32::from_gray(100)
                };

                painter.line_segment(
                    [Pos2::new(x, rect.max.y - tick_height), Pos2::new(x, rect.max.y)],
                    Stroke::new(1.0, tick_color),
                );

                // Draw time label for major ticks
                if is_major {
                    let label = if time < 60.0 {
                        format!("{:.1}s", time)
                    } else {
                        format!("{}:{:02}", (time / 60.0) as u32, (time % 60.0) as u32)
                    };

                    painter.text(
                        Pos2::new(x + 2.0, rect.min.y + 4.0),
                        egui::Align2::LEFT_TOP,
                        label,
                        egui::FontId::monospace(10.0),
                        Color32::from_gray(180),
                    );
                }
            }

            time += tick_interval;
        }

        // Draw playhead in header
        let playhead_x = self.time_to_x(self.playback.time);
        if playhead_x >= TRACK_HEADER_WIDTH && playhead_x <= rect.max.x {
            // Playhead triangle
            let triangle = vec![
                Pos2::new(playhead_x, rect.max.y - 8.0),
                Pos2::new(playhead_x - 6.0, rect.max.y),
                Pos2::new(playhead_x + 6.0, rect.max.y),
            ];
            painter.add(egui::Shape::convex_polygon(
                triangle,
                Color32::from_rgb(255, 100, 100),
                Stroke::NONE,
            ));
        }

        // Handle playhead dragging in header
        let header_response = ui.interact(rect, ui.id().with("timeline_header"), Sense::drag());
        if header_response.drag_started() {
            self.drag_op = DragOperation::Playhead;
        }
        if header_response.dragged() {
            if let DragOperation::Playhead = self.drag_op {
                let mouse_pos = header_response.interact_pointer_pos().unwrap_or(rect.center());
                let time = self.snap_time(self.x_to_time(mouse_pos.x).max(0.0));
                self.playback.seek(time.min(sequence.duration));
            }
        }
        if header_response.drag_stopped() {
            self.drag_op = DragOperation::None;
        }
    }

    /// Render tracks area
    fn render_tracks_area(&mut self, ui: &mut egui::Ui, rect: Rect, sequence: &mut Sequence) {
        let painter = ui.painter_at(rect);

        // Background
        painter.rect_filled(rect, 0.0, Color32::from_gray(30));

        // Track header background
        let header_rect = Rect::from_min_size(
            rect.min,
            Vec2::new(TRACK_HEADER_WIDTH, rect.height()),
        );
        painter.rect_filled(header_rect, 0.0, Color32::from_gray(35));

        // Collect track IDs to avoid borrow issues
        let track_ids: Vec<TrackId> = sequence.tracks().map(|t| t.id).collect();

        // Render each track
        let mut y = rect.min.y - self.vertical_scroll;

        for (idx, track_id) in track_ids.iter().enumerate() {
            if y > rect.max.y {
                break;
            }

            if y + TRACK_HEIGHT > rect.min.y {
                let track_rect = Rect::from_min_size(
                    Pos2::new(rect.min.x, y),
                    Vec2::new(rect.width(), TRACK_HEIGHT),
                );

                // Get track data
                if let Some(track) = sequence.track(*track_id) {
                    let is_selected = self.selection.tracks.contains(track_id);
                    self.render_track(ui, painter.clone(), track_rect, track, is_selected, idx);
                }
            }

            y += TRACK_HEIGHT;
        }

        // Draw playhead line
        let playhead_x = self.time_to_x(self.playback.time);
        if playhead_x >= TRACK_HEADER_WIDTH && playhead_x <= rect.max.x {
            painter.line_segment(
                [Pos2::new(playhead_x, rect.min.y), Pos2::new(playhead_x, rect.max.y)],
                Stroke::new(PLAYHEAD_WIDTH, Color32::from_rgb(255, 100, 100)),
            );
        }

        // Draw loop region if set
        if let (Some(start), Some(end)) = (self.playback.loop_start, self.playback.loop_end) {
            let start_x = self.time_to_x(start);
            let end_x = self.time_to_x(end);

            let loop_rect = Rect::from_min_max(
                Pos2::new(start_x.max(TRACK_HEADER_WIDTH), rect.min.y),
                Pos2::new(end_x.min(rect.max.x), rect.max.y),
            );

            painter.rect_filled(loop_rect, 0.0, Color32::from_rgba_unmultiplied(100, 150, 255, 30));
        }
    }

    /// Render a single track
    fn render_track(
        &mut self,
        _ui: &mut egui::Ui,
        painter: egui::Painter,
        rect: Rect,
        track: &Track,
        is_selected: bool,
        index: usize,
    ) {
        // Alternating background
        let bg_color = if index % 2 == 0 {
            Color32::from_gray(32)
        } else {
            Color32::from_gray(28)
        };

        // Selection highlight
        let bg_color = if is_selected {
            Color32::from_rgba_unmultiplied(100, 150, 255, 40)
        } else {
            bg_color
        };

        painter.rect_filled(rect, 0.0, bg_color);

        // Track header
        let header_rect = Rect::from_min_size(
            rect.min,
            Vec2::new(TRACK_HEADER_WIDTH, TRACK_HEIGHT),
        );

        // Track color indicator
        let color = track.effective_color();
        let color_rect = Rect::from_min_size(
            header_rect.min,
            Vec2::new(4.0, TRACK_HEIGHT),
        );
        painter.rect_filled(color_rect, 0.0, Color32::from_rgb(color[0], color[1], color[2]));

        // Track name
        let name_pos = Pos2::new(header_rect.min.x + 10.0, header_rect.center().y);
        let text_color = if track.muted {
            Color32::from_gray(100)
        } else {
            Color32::from_gray(200)
        };
        painter.text(
            name_pos,
            egui::Align2::LEFT_CENTER,
            &track.name,
            egui::FontId::proportional(12.0),
            text_color,
        );

        // Track type badge
        let type_text = track.track_type.name();
        painter.text(
            Pos2::new(header_rect.max.x - 50.0, header_rect.center().y),
            egui::Align2::LEFT_CENTER,
            type_text,
            egui::FontId::proportional(10.0),
            Color32::from_gray(120),
        );

        // Mute/Lock indicators
        if track.muted {
            painter.text(
                Pos2::new(header_rect.max.x - 20.0, header_rect.center().y),
                egui::Align2::CENTER_CENTER,
                "M",
                egui::FontId::proportional(10.0),
                Color32::from_rgb(255, 150, 100),
            );
        }

        if track.locked {
            painter.text(
                Pos2::new(header_rect.max.x - 8.0, header_rect.center().y),
                egui::Align2::CENTER_CENTER,
                "🔒",
                egui::FontId::proportional(10.0),
                Color32::from_gray(150),
            );
        }

        // Draw keyframes in content area
        let content_rect = Rect::from_min_max(
            Pos2::new(TRACK_HEADER_WIDTH, rect.min.y),
            rect.max,
        );

        match self.view_mode {
            ViewMode::Dopesheet => {
                self.render_keyframes_dopesheet(&painter, content_rect, track);
            }
            ViewMode::CurveEditor => {
                self.render_keyframes_curves(&painter, content_rect, track);
            }
        }

        // Separator line
        painter.line_segment(
            [Pos2::new(rect.min.x, rect.max.y), Pos2::new(rect.max.x, rect.max.y)],
            Stroke::new(1.0, Color32::from_gray(45)),
        );
    }

    /// Render keyframes in dopesheet mode
    fn render_keyframes_dopesheet(&self, painter: &egui::Painter, rect: Rect, track: &Track) {
        let center_y = rect.center().y;

        for keyframe in &track.keyframes {
            let x = self.time_to_x(keyframe.time);

            if x < rect.min.x || x > rect.max.x {
                continue;
            }

            let is_selected = self.selection.keyframes.contains(&(track.id, keyframe.id));

            // Diamond shape for keyframe
            let half_size = KEYFRAME_SIZE / 2.0;
            let diamond = vec![
                Pos2::new(x, center_y - half_size),
                Pos2::new(x + half_size, center_y),
                Pos2::new(x, center_y + half_size),
                Pos2::new(x - half_size, center_y),
            ];

            let color = track.effective_color();
            let fill_color = if is_selected {
                Color32::from_rgb(255, 200, 100)
            } else {
                Color32::from_rgb(color[0], color[1], color[2])
            };

            let stroke = if is_selected {
                Stroke::new(2.0, Color32::WHITE)
            } else {
                Stroke::new(1.0, Color32::from_gray(80))
            };

            painter.add(egui::Shape::convex_polygon(diamond, fill_color, stroke));

            // Interpolation indicator
            match keyframe.interpolation {
                InterpolationMode::Constant => {
                    painter.circle_filled(
                        Pos2::new(x, center_y),
                        2.0,
                        Color32::from_gray(60),
                    );
                }
                InterpolationMode::Bezier => {
                    painter.circle_stroke(
                        Pos2::new(x, center_y),
                        2.0,
                        Stroke::new(1.0, Color32::from_rgb(100, 200, 255)),
                    );
                }
                _ => {}
            }
        }
    }

    /// Render keyframes in curve editor mode
    fn render_keyframes_curves(&self, painter: &egui::Painter, rect: Rect, track: &Track) {
        // Only render float-valued keyframes in curve mode
        let float_keyframes: Vec<_> = track.keyframes.iter()
            .filter_map(|k| k.value.as_float().map(|v| (k, v)))
            .collect();

        if float_keyframes.is_empty() {
            return;
        }

        let center_y = rect.center().y;

        // Draw curve
        let mut points = Vec::new();
        for (keyframe, value) in &float_keyframes {
            let x = self.time_to_x(keyframe.time);
            let y = center_y - (value - self.curve_offset) * self.curve_scale / 100.0;
            points.push(Pos2::new(x, y.clamp(rect.min.y, rect.max.y)));
        }

        if points.len() >= 2 {
            let color = track.effective_color();
            painter.add(egui::Shape::line(
                points.clone(),
                Stroke::new(2.0, Color32::from_rgb(color[0], color[1], color[2])),
            ));
        }

        // Draw keyframe points
        for (idx, (keyframe, _value)) in float_keyframes.iter().enumerate() {
            if idx >= points.len() {
                continue;
            }

            let pos = points[idx];
            let is_selected = self.selection.keyframes.contains(&(track.id, keyframe.id));

            let color = if is_selected {
                Color32::from_rgb(255, 200, 100)
            } else {
                Color32::WHITE
            };

            painter.circle_filled(pos, 4.0, color);
            painter.circle_stroke(pos, 4.0, Stroke::new(1.0, Color32::from_gray(80)));
        }
    }

    /// Handle input events
    fn handle_input(&mut self, ui: &mut egui::Ui, rect: Rect, sequence: &mut Sequence) {
        let response = ui.interact(rect, ui.id().with("sequencer_input"), Sense::click_and_drag());

        // Keyboard shortcuts
        if response.has_focus() || ui.input(|i| i.key_pressed(egui::Key::Space)) {
            ui.input(|input| {
                if input.key_pressed(egui::Key::Space) {
                    self.playback.toggle_playback();
                }

                if input.key_pressed(egui::Key::Home) {
                    self.playback.seek(0.0);
                }

                if input.key_pressed(egui::Key::End) {
                    self.playback.seek(sequence.duration);
                }

                if input.key_pressed(egui::Key::Delete) {
                    // Delete selected keyframes
                    for (track_id, keyframe_id) in &self.selection.keyframes {
                        if let Some(track) = sequence.track_mut(*track_id) {
                            track.remove_keyframe(*keyframe_id);
                        }
                    }
                    self.selection.keyframes.clear();
                }
            });
        }

        // Scroll wheel for zoom and pan
        ui.input(|input| {
            let scroll = input.smooth_scroll_delta;

            if input.modifiers.ctrl {
                // Zoom with Ctrl+scroll
                let zoom_delta = scroll.y * 0.01;
                self.zoom = (self.zoom * (1.0 + zoom_delta)).clamp(MIN_ZOOM, MAX_ZOOM);
            } else if input.modifiers.shift {
                // Horizontal scroll with Shift
                self.scroll_offset = (self.scroll_offset - scroll.y / self.zoom).max(0.0);
            } else {
                // Vertical scroll
                self.vertical_scroll = (self.vertical_scroll - scroll.y).max(0.0);
            }
        });

        // Middle mouse pan
        if response.drag_started_by(egui::PointerButton::Middle) {
            self.drag_op = DragOperation::Pan { start_scroll: self.scroll_offset };
        }

        if response.dragged_by(egui::PointerButton::Middle) {
            if let DragOperation::Pan { start_scroll } = self.drag_op {
                let delta = response.drag_delta();
                self.scroll_offset = (start_scroll - delta.x / self.zoom).max(0.0);
            }
        }

        if response.drag_stopped_by(egui::PointerButton::Middle)
            && matches!(self.drag_op, DragOperation::Pan { .. }) {
                self.drag_op = DragOperation::None;
            }
    }
}

impl Default for SequencerState {
    fn default() -> Self {
        Self::new()
    }
}

/// Sequencer panel for integration with editor
pub struct SequencerPanel {
    /// Panel name
    pub name: String,
    /// Active sequence
    pub sequence: Sequence,
    /// Editor state
    pub state: SequencerState,
    /// Whether changes are unsaved
    pub dirty: bool,
}

impl SequencerPanel {
    /// Create a new sequencer panel
    pub fn new(name: impl Into<String>) -> Self {
        let mut sequence = Sequence::new("Main Sequence");

        // Add some demo tracks
        let mut transform_track = Track::new("Camera Transform", TrackType::Transform);
        transform_track.add_keyframe(crate::keyframe::Keyframe::new(0.0, KeyframeValue::Vec3([0.0, 0.0, 0.0])));
        transform_track.add_keyframe(crate::keyframe::Keyframe::new(2.0, KeyframeValue::Vec3([5.0, 2.0, 0.0])));
        transform_track.add_keyframe(crate::keyframe::Keyframe::new(5.0, KeyframeValue::Vec3([0.0, 5.0, 10.0])));
        sequence.add_track(transform_track);

        let mut property_track = Track::new("Light Intensity", TrackType::Property);
        property_track.add_keyframe(crate::keyframe::Keyframe::new(0.0, KeyframeValue::Float(1.0)));
        property_track.add_keyframe(crate::keyframe::Keyframe::new(1.5, KeyframeValue::Float(0.2)));
        property_track.add_keyframe(crate::keyframe::Keyframe::new(3.0, KeyframeValue::Float(1.0)));
        sequence.add_track(property_track);

        let mut event_track = Track::new("Events", TrackType::Event);
        event_track.add_keyframe(crate::keyframe::Keyframe::new(1.0, KeyframeValue::Event("explosion".to_string())));
        event_track.add_keyframe(crate::keyframe::Keyframe::new(4.0, KeyframeValue::Event("door_open".to_string())));
        sequence.add_track(event_track);

        let audio_track = Track::new("Background Music", TrackType::Audio);
        sequence.add_track(audio_track);

        let camera_track = Track::new("Camera Settings", TrackType::Camera);
        sequence.add_track(camera_track);

        Self {
            name: name.into(),
            sequence,
            state: SequencerState::new(),
            dirty: false,
        }
    }

    /// Update playback (call each frame)
    pub fn update(&mut self, delta_time: f32) {
        self.state.playback.update(delta_time, &self.sequence);
    }

    /// Render the panel UI
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.state.ui(ui, &mut self.sequence);
    }
}
