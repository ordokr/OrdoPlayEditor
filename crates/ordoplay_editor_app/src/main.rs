// SPDX-License-Identifier: MIT OR Apache-2.0
//! OrdoPlay Editor - State-of-the-Art Game Engine Editor
//!
//! A comprehensive game development environment featuring:
//! - Viewport with full OrdoPlay rendering (Nanite, Lumen)
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
//! the OrdoPlay runtime via shared crates. It uses egui for UI with
//! egui_dock for panel docking.

mod app;
mod commands;
mod history;
mod menus;
mod panel_types;
mod panels;
mod state;
mod theme;
mod thumbnail;
mod tools;
mod viewport_renderer;

use app::EditorApp;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("ordoplay_editor_app=debug".parse().unwrap())
                .add_directive("wgpu=warn".parse().unwrap())
                .add_directive("naga=warn".parse().unwrap()),
        )
        .init();

    tracing::info!("Starting OrdoPlay Editor v{}", env!("CARGO_PKG_VERSION"));

    // Run the editor application
    if let Err(e) = EditorApp::run() {
        tracing::error!("Editor crashed: {e}");
        std::process::exit(1);
    }
}
