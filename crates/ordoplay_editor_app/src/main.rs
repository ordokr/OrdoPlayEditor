// SPDX-License-Identifier: MIT OR Apache-2.0
//! `OrdoPlay` Editor - State-of-the-Art Game Engine Editor
//!
//! A comprehensive game development environment featuring:
//! - Viewport with 3D rendering
//! - Hierarchy and Inspector panels
//! - Asset browser with hot-reload
//! - Material graph editor
//! - Visual scripting (gameplay graphs)
//! - Timeline/Sequencer
//! - Full undo/redo support
//!
//! ## Architecture
//!
//! The editor is built as a standalone application that communicates with
//! the `OrdoPlay` runtime via shared crates. It uses egui for UI with
//! `egui_dock` for panel docking.

mod app;
mod audio;
mod build;
mod commands;
mod components;
mod file_watcher;
mod history;
mod hot_reload;
mod menus;
mod panel_types;
mod panels;
mod physics;
mod play_mode;
mod prefab;
mod project;
mod state;
mod theme;
mod thumbnail;
mod tools;
mod viewport_renderer;

use app::EditorApp;
use panels::console::TracingBridge;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

fn main() {
    // Create the console tracing bridge (channel pair)
    let (bridge_layer, tracing_rx) = TracingBridge::new();

    // Initialize logging with both the fmt layer and the console bridge layer
    let env_filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive("ordoplay_editor_app=debug".parse().unwrap())
        .add_directive("wgpu=warn".parse().unwrap())
        .add_directive("naga=warn".parse().unwrap());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .with(bridge_layer)
        .init();

    tracing::info!("Starting OrdoPlay Editor v{}", env!("CARGO_PKG_VERSION"));

    // Run the editor application, passing the tracing receiver to the console panel
    if let Err(e) = EditorApp::run_with_tracing_receiver(Some(tracing_rx)) {
        tracing::error!("Editor crashed: {e}");
        std::process::exit(1);
    }
}
