# OrdoPlay Editor - Implementation Task List

## Overview

This task list is organized by implementation phase, with each task including:
- **Source**: Where to copy/adapt code from
- **Priority**: P0 (critical), P1 (high), P2 (medium), P3 (nice-to-have)
- **Dependencies**: Tasks that must complete first
- **Estimate**: Rough effort estimate

---

## Phase 0: Workspace Setup

### P0-001: Create Editor Workspace
- **Priority**: P0
- **Source**: New
- **Dependencies**: None
- **Tasks**:
  - [ ] Create `C:\Users\Administrator\Desktop\OrdoPlayEditor\`
  - [ ] Create workspace `Cargo.toml`
  - [ ] Set up workspace members
  - [ ] Configure path dependencies to `../OrdoPlay/crates/*`
  - [ ] Copy workspace lint configuration from OrdoPlay

### P0-002: Create ordoplay_editor_app Crate
- **Priority**: P0
- **Source**: New
- **Dependencies**: P0-001
- **Tasks**:
  - [ ] Create `crates/ordoplay_editor_app/Cargo.toml`
  - [ ] Add dependencies: ordoplay_render, ordoplay_ui, ordoplay_editor, egui, egui_dock
  - [ ] Create `src/main.rs` entry point
  - [ ] Create `src/app.rs` with EditorApp struct
  - [ ] Verify compilation

### P0-003: Create ordoplay_editor_core Crate (in main workspace)
- **Priority**: P0
- **Source**: Ambient reference + New
- **Dependencies**: None
- **Tasks**:
  - [ ] Create `OrdoPlay/crates/ordoplay_editor_core/Cargo.toml`
  - [ ] Create `src/lib.rs`
  - [ ] Create `src/selection.rs` (port from Ambient Selection struct)
  - [ ] Create `src/commands.rs` (EditorCommand trait)
  - [ ] Create `src/gizmo_modes.rs` (TransformMode enum)
  - [ ] Add to OrdoPlay workspace members

---

## Phase 1: MVP Editor Shell

### P1-001: Egui Integration
- **Priority**: P0
- **Source**: bevy_egui patterns
- **Dependencies**: P0-002
- **Tasks**:
  - [ ] Set up egui context within OrdoPlay render loop
  - [ ] Create EguiRenderPass for ordoplay_render
  - [ ] Handle input forwarding to egui
  - [ ] Test basic egui rendering

### P1-002: Docking System
- **Priority**: P0
- **Source**: egui_dock
- **Dependencies**: P1-001
- **Tasks**:
  - [ ] Integrate egui_dock or egui_tiles
  - [ ] Create DockState for panel management
  - [ ] Implement layout serialization/restoration
  - [ ] Create default layout preset

### P1-003: Panel Framework
- **Priority**: P0
- **Source**: New
- **Dependencies**: P1-002
- **Tasks**:
  - [ ] Create EditorPanel trait
  - [ ] Create PanelRegistry
  - [ ] Implement panel add/remove/show/hide
  - [ ] Create placeholder panels (Viewport, Hierarchy, Inspector, AssetBrowser, Console)

### P1-004: Viewport Panel
- **Priority**: P0
- **Source**: ordoplay_render
- **Dependencies**: P1-003
- **Tasks**:
  - [ ] Create ViewportPanel struct
  - [ ] Render ordoplay_render output to egui::TextureHandle
  - [ ] Handle viewport resize
  - [ ] Wire mouse events for camera/picking
  - [ ] Add viewport toolbar (play, pause, render settings)

### P1-005: Editor Camera
- **Priority**: P0
- **Source**: New (reference ordoplay_camera)
- **Dependencies**: P1-004
- **Tasks**:
  - [ ] Create EditorCamera struct
  - [ ] Implement orbit controls (Alt+LMB)
  - [ ] Implement pan controls (MMB)
  - [ ] Implement zoom controls (Scroll)
  - [ ] Implement focus-on-selection (F key)
  - [ ] Add camera speed settings

### P1-006: Selection System
- **Priority**: P0
- **Source**: ordoplay_editor_core + ordoplay_picking
- **Dependencies**: P0-003, P1-004
- **Tasks**:
  - [ ] Create EditorState with Selection
  - [ ] Wire ordoplay_picking to viewport
  - [ ] Implement click-to-select
  - [ ] Implement Shift+Click for add
  - [ ] Implement Ctrl+Click for remove
  - [ ] Implement box selection (drag)
  - [ ] Visual feedback (selection outline/highlight)

### P1-007: Hierarchy Panel
- **Priority**: P0
- **Source**: ordoplay_scene
- **Dependencies**: P1-003, P1-006
- **Tasks**:
  - [ ] Create HierarchyPanel struct
  - [ ] Render entity tree from ordoplay_scene
  - [ ] Click-to-select entities
  - [ ] Drag-and-drop for reparenting
  - [ ] Right-click context menu (create, delete, duplicate)
  - [ ] Search/filter functionality
  - [ ] Show/hide toggle per entity

### P1-008: Basic Inspector
- **Priority**: P0
- **Source**: ordoplay_core/reflect + bevy-inspector-egui patterns
- **Dependencies**: P1-003, P1-006
- **Tasks**:
  - [ ] Create InspectorPanel struct
  - [ ] Iterate selected entity components via Reflect
  - [ ] Render primitive fields (bool, int, float, string)
  - [ ] Render Vec2, Vec3, Vec4, Quat, Color
  - [ ] Render enums as dropdowns
  - [ ] Render structs as collapsible sections
  - [ ] Render collections (Vec, HashMap) as expandable lists

### P1-009: Property Editing
- **Priority**: P0
- **Source**: New (leverage ordoplay_editor::History)
- **Dependencies**: P1-008
- **Tasks**:
  - [ ] Create PropertyEditCommand
  - [ ] Capture before/after state on edit
  - [ ] Submit to ordoplay_editor::History
  - [ ] Wire undo (Ctrl+Z) shortcut
  - [ ] Wire redo (Ctrl+Y) shortcut
  - [ ] Show undo/redo descriptions in Edit menu

### P1-010: Transform Gizmos
- **Priority**: P0
- **Source**: ordoplay_gizmos
- **Dependencies**: P1-006
- **Tasks**:
  - [ ] Create TranslateGizmo (XYZ axes + planes)
  - [ ] Create RotateGizmo (XYZ rotation arcs)
  - [ ] Create ScaleGizmo (XYZ axes + uniform)
  - [ ] Implement gizmo picking/interaction
  - [ ] Wire to TransformCommand + undo/redo
  - [ ] Keyboard shortcuts (W=translate, E=rotate, R=scale)
  - [ ] Local/Global coordinate toggle

### P1-011: Scene Save/Load
- **Priority**: P0
- **Source**: ordoplay_scene
- **Dependencies**: P1-007
- **Tasks**:
  - [ ] File > New Scene
  - [ ] File > Open Scene (file dialog)
  - [ ] File > Save Scene
  - [ ] File > Save Scene As
  - [ ] Recent scenes list
  - [ ] Unsaved changes warning on close

---

## Phase 2: Asset Pipeline

### P2-001: Asset Browser Panel
- **Priority**: P1
- **Source**: ordoplay_asset
- **Dependencies**: P1-003
- **Tasks**:
  - [ ] Create AssetBrowserPanel struct
  - [ ] Directory tree navigation
  - [ ] Grid view for assets
  - [ ] List view for assets
  - [ ] Asset type icons
  - [ ] Search/filter by name and type
  - [ ] Breadcrumb navigation

### P2-002: Thumbnail Generation
- **Priority**: P1
- **Source**: New
- **Dependencies**: P2-001, ordoplay_render
- **Tasks**:
  - [ ] Create ThumbnailGenerator system
  - [ ] Render 3D model thumbnails
  - [ ] Generate texture previews
  - [ ] Cache thumbnails to disk
  - [ ] Async thumbnail loading

### P2-003: Asset Import Settings
- **Priority**: P1
- **Source**: New
- **Dependencies**: P2-001
- **Tasks**:
  - [ ] Create import settings UI framework
  - [ ] Texture import settings (compression, mipmaps)
  - [ ] Model import settings (scale, axis)
  - [ ] Audio import settings (compression, streaming)
  - [ ] Reimport functionality

### P2-004: Drag-Drop Asset Placement
- **Priority**: P1
- **Source**: New
- **Dependencies**: P2-001, P1-004
- **Tasks**:
  - [ ] Drag from asset browser to viewport
  - [ ] Preview during drag
  - [ ] Raycast for placement position
  - [ ] Spawn prefab/model on drop
  - [ ] Create appropriate SpawnCommand

### P2-005: Hot Reload Integration
- **Priority**: P1
- **Source**: ordoplay_asset
- **Dependencies**: P2-001
- **Tasks**:
  - [ ] Wire ordoplay_asset file watcher
  - [ ] Reload textures on change
  - [ ] Reload shaders on change
  - [ ] Reload materials on change
  - [ ] Status notification on reload

---

## Phase 3: Prefab System

### P3-001: Prefab Override Tracking
- **Priority**: P1
- **Source**: ordoplay_editor_core
- **Dependencies**: P1-008
- **Tasks**:
  - [ ] Create PrefabOverride struct
  - [ ] Track field-level changes from prefab
  - [ ] Store overrides in scene file
  - [ ] Visual indicator for overridden properties in inspector

### P3-002: Prefab Edit Mode
- **Priority**: P1
- **Source**: New (Unity pattern)
- **Dependencies**: P3-001
- **Tasks**:
  - [ ] "Open Prefab" action
  - [ ] Isolated prefab editing context
  - [ ] Save changes to prefab asset
  - [ ] Back to scene navigation

### P3-003: Nested Prefabs
- **Priority**: P1
- **Source**: ordoplay_blueprint
- **Dependencies**: P3-001
- **Tasks**:
  - [ ] Support prefabs containing prefabs
  - [ ] Recursive override resolution
  - [ ] Break prefab link action
  - [ ] Create prefab from selection

### P3-004: Apply/Revert Overrides
- **Priority**: P1
- **Source**: New
- **Dependencies**: P3-001
- **Tasks**:
  - [ ] "Apply Override to Prefab" action
  - [ ] "Revert Override" action
  - [ ] "Apply All" / "Revert All"
  - [ ] Context menu integration

---

## Phase 4: Graph Framework

### P4-001: Base Graph Framework
- **Priority**: P1
- **Source**: ordoplay_graph + ordoplay_materialx
- **Dependencies**: P1-003
- **Tasks**:
  - [ ] Create `ordoplay_editor_graph` crate
  - [ ] GraphNode trait with ports
  - [ ] GraphPort (input/output, typed)
  - [ ] GraphConnection (edge)
  - [ ] Graph evaluation scheduler
  - [ ] Serialization format

### P4-002: Graph UI Renderer
- **Priority**: P1
- **Source**: New (egui)
- **Dependencies**: P4-001, P1-001
- **Tasks**:
  - [ ] Node rendering (header, ports, body)
  - [ ] Connection rendering (bezier curves)
  - [ ] Pan and zoom navigation
  - [ ] Node selection and multi-select
  - [ ] Node drag
  - [ ] Connection drag-to-create
  - [ ] Right-click node creation menu
  - [ ] Minimap

### P4-003: Material Graph
- **Priority**: P1
- **Source**: ordoplay_materialx
- **Dependencies**: P4-002
- **Tasks**:
  - [ ] Register MaterialX node types
  - [ ] Material preview viewport
  - [ ] Compile to WGSL on change
  - [ ] Save material graph asset

### P4-004: Gameplay Graph (Visual Scripting)
- **Priority**: P2
- **Source**: New (Unreal Blueprints pattern)
- **Dependencies**: P4-002
- **Tasks**:
  - [ ] Execution flow nodes
  - [ ] Event nodes (OnStart, OnUpdate, OnCollision)
  - [ ] Variable get/set nodes
  - [ ] Math nodes
  - [ ] Flow control (branch, loop, sequence)
  - [ ] Entity reference nodes
  - [ ] Component access nodes

### P4-005: Animation State Machine Graph
- **Priority**: P2
- **Source**: ordoplay_animation
- **Dependencies**: P4-002
- **Tasks**:
  - [ ] State nodes
  - [ ] Transition edges with conditions
  - [ ] Blend nodes
  - [ ] Animation preview
  - [ ] Parameter binding

---

## Phase 5: Sequencer/Timeline

### P5-001: Sequencer Framework
- **Priority**: P2
- **Source**: New
- **Dependencies**: P1-003
- **Tasks**:
  - [ ] Create `ordoplay_editor_sequencer` crate
  - [ ] Track base class
  - [ ] Keyframe data structure
  - [ ] Time cursor management
  - [ ] Playback state

### P5-002: Sequencer UI
- **Priority**: P2
- **Source**: New (egui)
- **Dependencies**: P5-001
- **Tasks**:
  - [ ] Timeline header (time ruler)
  - [ ] Track list panel
  - [ ] Track content rendering
  - [ ] Keyframe rendering
  - [ ] Time cursor dragging
  - [ ] Zoom controls
  - [ ] Playback controls

### P5-003: Track Types
- **Priority**: P2
- **Source**: ordoplay_animation, ordoplay_audio
- **Dependencies**: P5-001
- **Tasks**:
  - [ ] Transform track (position, rotation, scale)
  - [ ] Property track (any reflected property)
  - [ ] Event track (fire callbacks at time)
  - [ ] Audio track
  - [ ] Camera track (cuts, blends)

### P5-004: Keyframe Editing
- **Priority**: P2
- **Source**: New
- **Dependencies**: P5-002
- **Tasks**:
  - [ ] Insert keyframe at current time
  - [ ] Delete keyframe
  - [ ] Move keyframe
  - [ ] Copy/paste keyframes
  - [ ] Keyframe interpolation modes (linear, bezier, step)

### P5-005: Curve Editor
- **Priority**: P2
- **Source**: New
- **Dependencies**: P5-004
- **Tasks**:
  - [ ] Curve visualization
  - [ ] Tangent handles
  - [ ] Multi-curve view
  - [ ] Curve fitting tools

---

## Phase 6: World Building

### P6-001: World Partition System
- **Priority**: P2
- **Source**: New (Unreal pattern)
- **Dependencies**: P1-011
- **Tasks**:
  - [ ] Cell grid definition
  - [ ] Entity-to-cell assignment
  - [ ] Cell serialization to separate files
  - [ ] Cell streaming based on camera

### P6-002: Data Layers
- **Priority**: P2
- **Source**: New (Unreal pattern)
- **Dependencies**: P6-001
- **Tasks**:
  - [ ] Layer asset definition
  - [ ] Entity layer assignment
  - [ ] Layer visibility toggle in editor
  - [ ] Runtime layer loading API

### P6-003: HLOD Pipeline
- **Priority**: P3
- **Source**: New
- **Dependencies**: P6-001
- **Tasks**:
  - [ ] HLOD cluster generation
  - [ ] Proxy mesh generation
  - [ ] Streaming integration
  - [ ] LOD transition settings

---

## Phase 7: Collaboration

### P7-001: CRDT Implementation
- **Priority**: P3
- **Source**: New
- **Dependencies**: All prior phases
- **Tasks**:
  - [ ] Create `ordoplay_editor_collab` crate
  - [ ] LWW (Last-Writer-Wins) register for properties
  - [ ] OR-Set for entity lists
  - [ ] Operation serialization

### P7-002: Networking Layer
- **Priority**: P3
- **Source**: ordoplay_net
- **Dependencies**: P7-001
- **Tasks**:
  - [ ] WebSocket server for local collaboration
  - [ ] Client connection management
  - [ ] Operation broadcast
  - [ ] State synchronization on connect

### P7-003: Presence System
- **Priority**: P3
- **Source**: New
- **Dependencies**: P7-002
- **Tasks**:
  - [ ] User cursor visualization
  - [ ] Selection sharing
  - [ ] "Jump to user" navigation
  - [ ] User list panel

---

## Phase 8: Polish

### P8-001: Command Palette
- **Priority**: P1
- **Source**: New
- **Dependencies**: P1-003
- **Tasks**:
  - [ ] Ctrl+P/Cmd+P shortcut
  - [ ] Fuzzy search over commands
  - [ ] Recent commands
  - [ ] Categorized results

### P8-002: Keyboard Shortcuts
- **Priority**: P1
- **Source**: ordoplay_input
- **Dependencies**: P1-003
- **Tasks**:
  - [ ] Shortcut registry
  - [ ] Customizable shortcuts
  - [ ] Shortcut settings UI
  - [ ] Context-aware shortcuts (viewport vs panel)

### P8-003: Theme System
- **Priority**: P2
- **Source**: New (egui)
- **Dependencies**: P1-001
- **Tasks**:
  - [ ] Dark theme (default)
  - [ ] Light theme
  - [ ] Custom color configuration
  - [ ] Theme save/load

### P8-004: Profiler Panel
- **Priority**: P1
- **Source**: ordoplay_profiler
- **Dependencies**: P1-003
- **Tasks**:
  - [ ] Frame time graph
  - [ ] CPU scope view
  - [ ] GPU scope view
  - [ ] Capture controls
  - [ ] Export to Chrome trace

### P8-005: Console Panel
- **Priority**: P1
- **Source**: New
- **Dependencies**: P1-003
- **Tasks**:
  - [ ] Log level filtering
  - [ ] Search in logs
  - [ ] Clear logs
  - [ ] Click-to-jump-to-source
  - [ ] Command input (future scripting)

---

## Summary: Priority Distribution

| Priority | Count | Description |
|----------|-------|-------------|
| P0 | 14 | Critical MVP tasks |
| P1 | 21 | High priority features |
| P2 | 12 | Medium priority features |
| P3 | 4 | Nice-to-have / future |

## Recommended Build Order

### Sprint 1 (Week 1-2): Workspace + Shell
- P0-001, P0-002, P0-003
- P1-001, P1-002, P1-003
- P1-004, P1-005

### Sprint 2 (Week 3-4): Core Editing
- P1-006, P1-007, P1-008
- P1-009, P1-010, P1-011

### Sprint 3 (Week 5-6): Assets + Prefabs
- P2-001, P2-002, P2-003, P2-004, P2-005
- P3-001, P3-002

### Sprint 4 (Week 7-8): Graph + Polish
- P4-001, P4-002, P4-003
- P8-001, P8-002, P8-004, P8-005

### Future Sprints
- P4-004, P4-005 (Visual Scripting)
- P5-* (Sequencer)
- P6-* (World Building)
- P7-* (Collaboration)
- P3-003, P3-004 (Advanced Prefabs)
- P8-003 (Themes)

---

*Task List Version: 1.0*
*Created: January 17, 2026*
