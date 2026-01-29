// SPDX-License-Identifier: MIT OR Apache-2.0
//! Project Settings panel - Configure project-wide settings.

use crate::project::{
    BuildConfiguration, QualityLevel, TargetPlatform, TextureCompression, InputType,
};
use crate::state::EditorState;

/// Category tabs for project settings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsCategory {
    #[default]
    Project,
    Build,
    Scenes,
    Physics,
    Graphics,
    Audio,
    Input,
}

impl SettingsCategory {
    pub fn all() -> &'static [SettingsCategory] {
        &[
            SettingsCategory::Project,
            SettingsCategory::Build,
            SettingsCategory::Scenes,
            SettingsCategory::Physics,
            SettingsCategory::Graphics,
            SettingsCategory::Audio,
            SettingsCategory::Input,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            SettingsCategory::Project => "Project",
            SettingsCategory::Build => "Build",
            SettingsCategory::Scenes => "Scenes",
            SettingsCategory::Physics => "Physics",
            SettingsCategory::Graphics => "Graphics",
            SettingsCategory::Audio => "Audio",
            SettingsCategory::Input => "Input",
        }
    }
}

/// Project Settings panel (shown as a window)
pub struct ProjectSettingsPanel {
    /// Whether the window is open
    pub open: bool,
    /// Currently selected category
    pub current_category: SettingsCategory,
    /// New axis name buffer
    pub new_axis_name: String,
}

impl ProjectSettingsPanel {
    pub fn new() -> Self {
        Self {
            open: false,
            current_category: SettingsCategory::default(),
            new_axis_name: String::new(),
        }
    }

    /// Show the project settings window
    pub fn show(&mut self, ctx: &egui::Context, state: &mut EditorState) {
        if !self.open {
            return;
        }

        let mut open = self.open;
        egui::Window::new("Project Settings")
            .open(&mut open)
            .default_size([700.0, 500.0])
            .resizable(true)
            .show(ctx, |ui| {
                self.ui(ui, state);
            });
        self.open = open;
    }

    fn ui(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        if !state.project_manager.is_project_open() {
            ui.label("No project open. Create or open a project first.");
            return;
        }

        // Category tabs
        ui.horizontal(|ui| {
            for category in SettingsCategory::all() {
                if ui.selectable_label(self.current_category == *category, category.name()).clicked() {
                    self.current_category = *category;
                }
            }
        });

        // Save button and status
        ui.horizontal(|ui| {
            if ui.button("Save Settings").clicked() {
                if let Err(e) = state.project_manager.save_project() {
                    tracing::error!("Failed to save: {}", e);
                }
            }
            if state.project_manager.has_unsaved_changes() {
                ui.label(egui::RichText::new("*Modified").color(egui::Color32::YELLOW));
            }
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            match self.current_category {
                SettingsCategory::Project => self.project_ui(ui, state),
                SettingsCategory::Build => self.build_ui(ui, state),
                SettingsCategory::Scenes => self.scenes_ui(ui, state),
                SettingsCategory::Physics => self.physics_ui(ui, state),
                SettingsCategory::Graphics => self.graphics_ui(ui, state),
                SettingsCategory::Audio => self.audio_ui(ui, state),
                SettingsCategory::Input => self.input_ui(ui, state),
            }
        });
    }

    fn project_ui(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        ui.heading("Project Information");

        let mut dirty = false;

        dirty |= ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut state.project_manager.settings.metadata.name).changed()
        }).inner;

        dirty |= ui.horizontal(|ui| {
            ui.label("Version:");
            ui.text_edit_singleline(&mut state.project_manager.settings.metadata.version).changed()
        }).inner;

        dirty |= ui.horizontal(|ui| {
            ui.label("Company:");
            ui.text_edit_singleline(&mut state.project_manager.settings.metadata.company).changed()
        }).inner;

        ui.label("Description:");
        dirty |= ui.text_edit_multiline(&mut state.project_manager.settings.metadata.description).changed();

        if dirty {
            state.project_manager.mark_dirty();
        }

        if let Some(dir) = &state.project_manager.project_dir {
            ui.separator();
            ui.label(format!("Location: {}", dir.display()));
        }
    }

    fn build_ui(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        ui.heading("Build Configuration");

        let mut dirty = false;

        // Configuration
        let current_config = state.project_manager.settings.build_configuration;
        ui.horizontal(|ui| {
            ui.label("Configuration:");
            if ui.selectable_label(current_config == BuildConfiguration::Debug, "Debug").clicked() {
                state.project_manager.settings.build_configuration = BuildConfiguration::Debug;
                dirty = true;
            }
            if ui.selectable_label(current_config == BuildConfiguration::Release, "Release").clicked() {
                state.project_manager.settings.build_configuration = BuildConfiguration::Release;
                dirty = true;
            }
        });

        // Platform
        let current_platform = state.project_manager.settings.target_platform;
        ui.horizontal(|ui| {
            ui.label("Platform:");
            for platform in TargetPlatform::all() {
                if ui.selectable_label(current_platform == *platform, platform.display_name()).clicked() {
                    state.project_manager.settings.target_platform = *platform;
                    dirty = true;
                }
            }
        });

        ui.separator();
        ui.heading("Platform Settings");

        let platform = state.project_manager.settings.target_platform;
        let ps = state.project_manager.settings.get_platform_settings_mut(platform);

        dirty |= ui.horizontal(|ui| {
            ui.label("Enabled:");
            ui.checkbox(&mut ps.enabled, "").changed()
        }).inner;

        dirty |= ui.horizontal(|ui| {
            ui.label("Compress Assets:");
            ui.checkbox(&mut ps.compress_assets, "").changed()
        }).inner;

        dirty |= ui.horizontal(|ui| {
            ui.label("Debug Symbols:");
            ui.checkbox(&mut ps.include_debug_symbols, "").changed()
        }).inner;

        // Texture compression
        let current_tc = ps.texture_compression;
        ui.horizontal(|ui| {
            ui.label("Texture Compression:");
            for tc in [TextureCompression::None, TextureCompression::BC, TextureCompression::Astc, TextureCompression::ETC2] {
                let name = match tc {
                    TextureCompression::None => "None",
                    TextureCompression::BC => "BC",
                    TextureCompression::Astc => "ASTC",
                    TextureCompression::ETC2 => "ETC2",
                };
                if ui.selectable_label(current_tc == tc, name).clicked() {
                    state.project_manager.settings.get_platform_settings_mut(platform).texture_compression = tc;
                    dirty = true;
                }
            }
        });

        if dirty {
            state.project_manager.mark_dirty();
        }

        ui.separator();

        // Show build progress if building
        if state.build_manager.is_building() {
            if let Some(progress) = state.build_manager.get_progress() {
                ui.horizontal(|ui| {
                    ui.label(format!("Building: {}", progress.step));
                    if ui.small_button("Cancel").clicked() {
                        state.build_manager.cancel();
                    }
                });
                ui.add(egui::ProgressBar::new(progress.progress as f32 / 100.0));
            }
        } else {
            // Show build button and last result
            if ui.button("Build Project").clicked() {
                if let Some(project_dir) = state.project_manager.project_dir.clone() {
                    tracing::info!("Starting build for {:?}", platform);
                    state.build_manager.start_build(
                        &state.project_manager.settings,
                        &project_dir,
                    );
                } else {
                    tracing::warn!("No project directory set");
                }
            }

            // Show last build result
            if let Some(ref result) = state.build_manager.last_result {
                match result {
                    crate::build::BuildResult::Success { output_dir, build_time_secs, assets_processed, scenes_processed } => {
                        ui.label(egui::RichText::new(format!(
                            "Build successful: {} scenes, {} assets in {:.1}s",
                            scenes_processed, assets_processed, build_time_secs
                        )).color(egui::Color32::GREEN));
                        ui.label(format!("Output: {}", output_dir.display()));
                    }
                    crate::build::BuildResult::Cancelled => {
                        ui.label(egui::RichText::new("Build cancelled").color(egui::Color32::YELLOW));
                    }
                    crate::build::BuildResult::Failed(err) => {
                        ui.label(egui::RichText::new(format!("Build failed: {}", err)).color(egui::Color32::RED));
                    }
                }
            }
        }
    }

    fn scenes_ui(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        ui.heading("Scene Management");

        // Startup scene
        let startup = state.project_manager.settings.scenes.startup_scene.clone();
        ui.horizontal(|ui| {
            ui.label("Startup Scene:");
            if let Some(ref path) = startup {
                ui.label(format!("{}", path.display()));
                if ui.small_button("Clear").clicked() {
                    state.project_manager.settings.scenes.startup_scene = None;
                    state.project_manager.mark_dirty();
                }
            } else {
                ui.label("(None)");
            }
        });

        ui.separator();
        ui.label(egui::RichText::new("Build Scenes:").strong());

        let scenes: Vec<_> = state.project_manager.settings.scenes.build_scenes
            .iter()
            .map(|s| (s.path.clone(), s.enabled))
            .collect();

        if scenes.is_empty() {
            ui.label("No scenes in build.");
        }

        let mut action: Option<(usize, &str)> = None;
        for (i, (path, enabled)) in scenes.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("{}.", i));
                let color = if *enabled {
                    ui.style().visuals.text_color()
                } else {
                    ui.style().visuals.weak_text_color()
                };
                ui.label(egui::RichText::new(format!("{}", path.display())).color(color));

                if ui.small_button("^").clicked() && i > 0 {
                    action = Some((i, "up"));
                }
                if ui.small_button("v").clicked() && i < scenes.len() - 1 {
                    action = Some((i, "down"));
                }
                if ui.small_button(if *enabled { "D" } else { "E" }).clicked() {
                    action = Some((i, "toggle"));
                }
                if ui.small_button("X").clicked() {
                    action = Some((i, "remove"));
                }
                if startup.as_ref() != Some(path)
                    && ui.small_button("*").on_hover_text("Set startup").clicked() {
                        action = Some((i, "startup"));
                    }
            });
        }

        if let Some((i, act)) = action {
            match act {
                "up" => state.project_manager.settings.move_scene_up(i),
                "down" => state.project_manager.settings.move_scene_down(i),
                "toggle" => {
                    state.project_manager.settings.scenes.build_scenes[i].enabled =
                        !state.project_manager.settings.scenes.build_scenes[i].enabled;
                }
                "remove" => {
                    state.project_manager.settings.scenes.build_scenes.remove(i);
                }
                "startup" => {
                    let path = state.project_manager.settings.scenes.build_scenes[i].path.clone();
                    state.project_manager.settings.scenes.startup_scene = Some(path);
                }
                _ => {}
            }
            state.project_manager.mark_dirty();
        }
    }

    fn physics_ui(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        ui.heading("Physics Settings");

        let physics = &mut state.project_manager.settings.physics;
        let mut dirty = false;

        dirty |= ui.horizontal(|ui| {
            ui.label("Gravity X:");
            ui.add(egui::DragValue::new(&mut physics.gravity[0]).speed(0.1)).changed()
        }).inner;
        dirty |= ui.horizontal(|ui| {
            ui.label("Gravity Y:");
            ui.add(egui::DragValue::new(&mut physics.gravity[1]).speed(0.1)).changed()
        }).inner;
        dirty |= ui.horizontal(|ui| {
            ui.label("Gravity Z:");
            ui.add(egui::DragValue::new(&mut physics.gravity[2]).speed(0.1)).changed()
        }).inner;

        dirty |= ui.horizontal(|ui| {
            ui.label("Fixed Timestep:");
            ui.add(egui::DragValue::new(&mut physics.fixed_timestep).speed(0.001).range(0.001..=0.1)).changed()
        }).inner;

        dirty |= ui.horizontal(|ui| {
            ui.label("Max Substeps:");
            ui.add(egui::DragValue::new(&mut physics.max_substeps).range(1..=32)).changed()
        }).inner;

        dirty |= ui.horizontal(|ui| {
            ui.label("Default Friction:");
            ui.add(egui::Slider::new(&mut physics.default_friction, 0.0..=1.0)).changed()
        }).inner;

        dirty |= ui.horizontal(|ui| {
            ui.label("Default Bounciness:");
            ui.add(egui::Slider::new(&mut physics.default_bounciness, 0.0..=1.0)).changed()
        }).inner;

        dirty |= ui.horizontal(|ui| {
            ui.label("Continuous Collision:");
            ui.checkbox(&mut physics.continuous_collision, "").changed()
        }).inner;

        if dirty {
            state.project_manager.mark_dirty();
        }
    }

    fn graphics_ui(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        ui.heading("Graphics Settings");

        let graphics = &mut state.project_manager.settings.graphics;
        let mut dirty = false;

        // Quality level
        let current_quality = graphics.default_quality;
        ui.horizontal(|ui| {
            ui.label("Quality:");
            for level in QualityLevel::all() {
                if ui.selectable_label(current_quality == *level, level.display_name()).clicked() {
                    graphics.default_quality = *level;
                    dirty = true;
                }
            }
        });

        dirty |= ui.horizontal(|ui| {
            ui.label("Width:");
            ui.add(egui::DragValue::new(&mut graphics.default_width).range(640..=7680)).changed()
        }).inner;

        dirty |= ui.horizontal(|ui| {
            ui.label("Height:");
            ui.add(egui::DragValue::new(&mut graphics.default_height).range(480..=4320)).changed()
        }).inner;

        dirty |= ui.horizontal(|ui| {
            ui.label("Fullscreen:");
            ui.checkbox(&mut graphics.fullscreen, "").changed()
        }).inner;

        dirty |= ui.horizontal(|ui| {
            ui.label("Resizable:");
            ui.checkbox(&mut graphics.resizable, "").changed()
        }).inner;

        dirty |= ui.horizontal(|ui| {
            ui.label("VSync:");
            ui.checkbox(&mut graphics.vsync, "").changed()
        }).inner;

        dirty |= ui.horizontal(|ui| {
            ui.label("Target FPS:");
            ui.add(egui::DragValue::new(&mut graphics.target_frame_rate).range(0..=240)).changed()
        }).inner;

        // MSAA
        let current_msaa = graphics.msaa_samples;
        ui.horizontal(|ui| {
            ui.label("MSAA:");
            for samples in [1, 2, 4, 8] {
                if ui.selectable_label(current_msaa == samples, format!("{}x", samples)).clicked() {
                    graphics.msaa_samples = samples;
                    dirty = true;
                }
            }
        });

        dirty |= ui.horizontal(|ui| {
            ui.label("Shadow Quality:");
            ui.add(egui::Slider::new(&mut graphics.shadow_quality, 0..=4)).changed()
        }).inner;

        dirty |= ui.horizontal(|ui| {
            ui.label("Shadow Distance:");
            ui.add(egui::DragValue::new(&mut graphics.shadow_distance).range(10.0..=1000.0)).changed()
        }).inner;

        if dirty {
            state.project_manager.mark_dirty();
        }
    }

    fn audio_ui(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        ui.heading("Audio Settings");

        let audio = &mut state.project_manager.settings.audio;
        let mut dirty = false;

        dirty |= ui.horizontal(|ui| {
            ui.label("Master Volume:");
            ui.add(egui::Slider::new(&mut audio.master_volume, 0.0..=1.0)).changed()
        }).inner;

        dirty |= ui.horizontal(|ui| {
            ui.label("Music Volume:");
            ui.add(egui::Slider::new(&mut audio.music_volume, 0.0..=1.0)).changed()
        }).inner;

        dirty |= ui.horizontal(|ui| {
            ui.label("SFX Volume:");
            ui.add(egui::Slider::new(&mut audio.sfx_volume, 0.0..=1.0)).changed()
        }).inner;

        dirty |= ui.horizontal(|ui| {
            ui.label("Max Sources:");
            ui.add(egui::DragValue::new(&mut audio.max_audio_sources).range(8..=128)).changed()
        }).inner;

        dirty |= ui.horizontal(|ui| {
            ui.label("Doppler Scale:");
            ui.add(egui::DragValue::new(&mut audio.doppler_scale).range(0.0..=10.0)).changed()
        }).inner;

        dirty |= ui.horizontal(|ui| {
            ui.label("Speed of Sound:");
            ui.add(egui::DragValue::new(&mut audio.speed_of_sound).range(100.0..=1000.0)).changed()
        }).inner;

        if dirty {
            state.project_manager.mark_dirty();
        }
    }

    fn input_ui(&mut self, ui: &mut egui::Ui, state: &mut EditorState) {
        ui.heading("Input Axes");

        // Add new axis
        ui.horizontal(|ui| {
            ui.label("New:");
            ui.text_edit_singleline(&mut self.new_axis_name);
            if ui.button("Add").clicked() && !self.new_axis_name.is_empty() {
                state.project_manager.settings.input.axes.push(crate::project::InputAxis {
                    name: self.new_axis_name.clone(),
                    ..Default::default()
                });
                self.new_axis_name.clear();
                state.project_manager.mark_dirty();
            }
        });

        ui.separator();

        // Copy axis info to avoid borrow issues
        let axes_info: Vec<(String, InputType)> = state.project_manager.settings.input.axes
            .iter()
            .map(|a| (a.name.clone(), a.input_type))
            .collect();

        let mut remove_idx: Option<usize> = None;
        let mut dirty = false;

        for (i, (name, input_type)) in axes_info.iter().enumerate() {
            let id = egui::Id::new(format!("axis_{}", i));
            egui::CollapsingHeader::new(name)
                .id_salt(id)
                .show(ui, |ui| {
                    let axis = &mut state.project_manager.settings.input.axes[i];

                    dirty |= ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut axis.name).changed()
                    }).inner;

                    // Input type selector
                    ui.horizontal(|ui| {
                        ui.label("Type:");
                        if ui.selectable_label(*input_type == InputType::Keyboard, "Keyboard").clicked() {
                            axis.input_type = InputType::Keyboard;
                            dirty = true;
                        }
                        if ui.selectable_label(*input_type == InputType::Mouse, "Mouse").clicked() {
                            axis.input_type = InputType::Mouse;
                            dirty = true;
                        }
                        if ui.selectable_label(*input_type == InputType::Joystick, "Joystick").clicked() {
                            axis.input_type = InputType::Joystick;
                            dirty = true;
                        }
                    });

                    if axis.input_type == InputType::Keyboard {
                        dirty |= ui.horizontal(|ui| {
                            ui.label("Positive:");
                            ui.text_edit_singleline(&mut axis.positive_button).changed()
                        }).inner;
                        dirty |= ui.horizontal(|ui| {
                            ui.label("Negative:");
                            ui.text_edit_singleline(&mut axis.negative_button).changed()
                        }).inner;
                    }

                    dirty |= ui.horizontal(|ui| {
                        ui.label("Sensitivity:");
                        ui.add(egui::DragValue::new(&mut axis.sensitivity).range(0.01..=10.0)).changed()
                    }).inner;

                    dirty |= ui.horizontal(|ui| {
                        ui.label("Dead Zone:");
                        ui.add(egui::DragValue::new(&mut axis.dead_zone).range(0.0..=1.0)).changed()
                    }).inner;

                    if ui.small_button("Remove").clicked() {
                        remove_idx = Some(i);
                    }
                });
        }

        if let Some(i) = remove_idx {
            state.project_manager.settings.input.axes.remove(i);
            dirty = true;
        }

        if dirty {
            state.project_manager.mark_dirty();
        }
    }
}

impl Default for ProjectSettingsPanel {
    fn default() -> Self {
        Self::new()
    }
}
