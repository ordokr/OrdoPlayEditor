// SPDX-License-Identifier: MIT OR Apache-2.0
//! Editor panel implementations.

mod viewport;
mod hierarchy;
mod inspector;
mod asset_browser;
mod console;
mod profiler;

pub use viewport::ViewportPanel;
pub use hierarchy::HierarchyPanel;
pub use inspector::InspectorPanel;
pub use asset_browser::AssetBrowserPanel;
pub use console::ConsolePanel;
pub use profiler::ProfilerPanel;
