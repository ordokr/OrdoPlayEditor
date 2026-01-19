// SPDX-License-Identifier: MIT OR Apache-2.0
//! Shared panel type definitions.

/// Panel types that can be docked in the editor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelType {
    /// 3D viewport with scene rendering
    Viewport,
    /// Entity hierarchy tree
    Hierarchy,
    /// Component/property inspector
    Inspector,
    /// Asset browser
    AssetBrowser,
    /// Console/log output
    Console,
    /// Performance profiler
    Profiler,
    /// Material graph editor
    MaterialGraph,
    /// Gameplay graph (visual scripting)
    GameplayGraph,
    /// Animation sequencer
    Sequencer,
}

impl PanelType {
    /// Get the display name for this panel type
    pub fn name(&self) -> &'static str {
        match self {
            Self::Viewport => "Viewport",
            Self::Hierarchy => "Hierarchy",
            Self::Inspector => "Inspector",
            Self::AssetBrowser => "Asset Browser",
            Self::Console => "Console",
            Self::Profiler => "Profiler",
            Self::MaterialGraph => "Material Graph",
            Self::GameplayGraph => "Gameplay Graph",
            Self::Sequencer => "Sequencer",
        }
    }

    /// Get the icon for this panel type
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Viewport => "\u{1f3a5}",      // camera
            Self::Hierarchy => "\u{1f4c2}",     // folder
            Self::Inspector => "\u{2699}",      // cog
            Self::AssetBrowser => "\u{1f4c1}",  // folder
            Self::Console => "\u{1f4bb}",       // terminal
            Self::Profiler => "\u{1f4ca}",      // chart
            Self::MaterialGraph => "\u{1f3a8}", // palette
            Self::GameplayGraph => "\u{1f500}", // branch
            Self::Sequencer => "\u{1f3ac}",     // film
        }
    }
}
