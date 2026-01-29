# OrdoPlay Editor - Production Roadmap

> **Version**: 3.0
> **Last Updated**: January 29, 2026
> **Target Release**: v1.0 Production
> **Audit Status**: Full codebase audit completed - all findings incorporated

---

## Executive Summary

### Core Purpose

**OrdoPlayEditor** is a next-generation, production-grade game engine editor built entirely in Rust. Its mission is to provide game developers with a **unified, performant, and extensible** content creation environment that rivals and exceeds the capabilities of industry leaders (Unreal Engine 5, Unity 6, Godot 4, O3DE).

**Key Differentiators**:
1. **Memory-Safe Architecture** - Built in Rust for reliability and performance
2. **Unified Graph Framework** - Single system for materials, gameplay, animation, and VFX
3. **Real-Time Collaboration** - CRDT-based multi-user editing from day one architecture
4. **Hot-Reload Everything** - Assets, scripts, shaders, scenes - no restart needed
5. **Copy-on-Write Undo** - O(1) snapshots for unlimited undo history
6. **Cross-Platform** - Windows, macOS, Linux from single codebase

### Current Status (as of January 29, 2026) - Audit-Corrected

> The previous roadmap overstated completion percentages. The following reflects
> ground-truth findings from a line-by-line source audit.

| Component | Claimed | Actual | Critical Gaps |
|-----------|---------|--------|---------------|
| Editor Shell (egui_dock) | 90% | **75%** | Cut/Copy/Paste no-ops, Tools menu no-ops, no native file dialogs, no persistence of layout/prefs, continuous redraw (no idle) |
| Viewport Panel | 80% | **40%** | Only renders grid+axis lines (no meshes, lights, materials, or entity visualization), hardcoded 1.0 bounding sphere picking, no box/lasso select, gizmo_op field unused |
| Hierarchy Panel | 85% | **80%** | Shallow search filter (not recursive), no keyboard nav, no sibling reorder, no Shift+range select |
| Inspector Panel | 75% | **35%** | **Component properties are read-only labels** - cannot edit light intensity, mesh reference, rigidbody mass, etc. Color picker writes to local var never applied back. property_drawer.rs infrastructure built but zero consumers |
| Asset Browser | 70% | **55%** | Rename/Delete/Show-in-Explorer are stubs, no drag-drop to viewport, favorites/recents not persisted, dead render_grid_item method, grid nav skips history |
| Console | N/A | **45%** | Mock chrono (always "00:00:00"), no tracing integration, reload command is fake, pending_jump never set |
| Profiler | N/A | **10%** | **All data is mock/hardcoded fake values** - no real profiling instrumentation whatsoever |
| Undo/Redo System | 90% | **70%** | No memory limit enforcement, no operation coalescing, TransformData quaternion conversion is wrong (euler stuffed as w=1.0), DuplicateCommand loses children |
| Graph Framework | 60% | **30%** | Core data model complete, but: zero NodeEvaluator implementations, no WGSL compilation, connection creation from port drag not wired, right-click context menu not implemented, connection hover detection dead code, save/load not implemented |
| Sequencer Framework | 50% | **30%** | Data model + playback controller complete, but: keyframe drag not functional (DragOperation declared but never entered), no box select, no context menus, no waveform rendering, bezier tangent eval falls back to linear, no undo integration |
| Material Graph Nodes | N/A | **70%** | 50+ node types registered, but Save is no-op, Compile is no-op, no evaluators exist |
| Gameplay Graph Nodes | N/A | **5%** | Only 4 nodes (begin_play, tick, branch, print_string) |
| Animation Graph Nodes | N/A | **5%** | Only 3 nodes (state, blend, output) |
| VFX Graph Nodes | N/A | **5%** | Only 4 nodes (spawn, init_pos, init_vel, output) |
| Play Mode | N/A | **40%** | State machine works, scene backup/restore works, but no integration with physics/audio/scripts |
| Physics | N/A | **25%** | Basic simulation exists but: capsule=sphere, AABB only (no OBB), no friction response, no raycasts, no joints, O(n^2) broadphase, not integrated into app |
| Audio | N/A | **20%** | rodio backend behind feature flag, spatial=linear falloff only, hardcoded SFx channel, no doppler, not integrated |
| Build System | N/A | **10%** | Pipeline structure exists but: textures just copied, blocking only, no real asset processing, no executable generation |
| Prefab System | N/A | **30%** | Data model + serialization work, but: overrides never applied, nested prefabs not resolved, instantiation has child ID bug |
| Project System | N/A | **40%** | Comprehensive settings model but: input/graphics/audio settings unconsumed, no version migration |
| File Watcher | N/A | **50%** | Functional watcher but: rename events not mapped, not integrated into app |
| Hot Reload | N/A | **15%** | Event detection framework but: debounce bug discards events, no actual reload logic |
| Components | N/A | **60%** | 11 component types defined with good structs, but no ParticleSystem/Animator/UI/CharacterController |
| Collaboration | 0% | **0%** | Not started |

### Untracked Files Not Yet Wired Into App

The following source files exist but are **not declared as modules in main.rs** and are completely disconnected from the running application:

- `audio.rs` - Audio engine (rodio)
- `build.rs` - Build pipeline
- `components.rs` - Component definitions
- `file_watcher.rs` - Filesystem watching
- `hot_reload.rs` - Hot reload event system
- `physics.rs` - Physics simulation
- `play_mode.rs` - Play/pause/stop state
- `prefab.rs` - Prefab system
- `project.rs` - Project/settings management

**Integration of these modules is a prerequisite for Phase 0 completion.**

---

## Part 1: Architectural Principles

### 1.1 Design Philosophy

```
┌─────────────────────────────────────────────────────────────────┐
│                    SOTA EDITOR PRINCIPLES                       │
├─────────────────────────────────────────────────────────────────┤
│ 1. TIME-TO-FEEDBACK     │ Every change visible in <100ms       │
│ 2. NON-DESTRUCTIVE      │ Never lose work, always reversible   │
│ 3. SEARCH-FIRST         │ Everything findable via Ctrl+P       │
│ 4. GRAPH-BASED          │ Visual authoring for all domains     │
│ 5. COLLABORATION-READY  │ Multi-user from architecture up      │
│ 6. EXTENSIBLE           │ Plugins for everything               │
│ 7. OBSERVABLE           │ Built-in profiling and diagnostics   │
│ 8. ACCESSIBLE           │ Keyboard-first, screen reader ready  │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 Technology Stack

| Layer | Technology | Rationale |
|-------|------------|-----------|
| Language | Rust 1.80+ | Memory safety, performance, fearless concurrency |
| UI Framework | egui 0.30 + egui_dock 0.15 | Immediate-mode, rapid iteration |
| Rendering | wgpu | Cross-platform GPU abstraction |
| Windowing | winit 0.30 | Native window management |
| Serialization | serde + RON | Human-readable, diffable scene files |
| Async | tokio | Multi-threaded async operations |
| Logging | tracing | Structured diagnostics |
| File Watching | notify 7.0 | Filesystem change detection |
| Audio | rodio (feature-gated) | Audio playback |

### 1.3 Crate Architecture

```
ordoplay_editor_app          # Main binary - panels, menus, tools
├── ordoplay_editor_graph    # Unified node graph framework
├── ordoplay_editor_sequencer # Timeline and keyframe editing
├── ordoplay_editor_collab   # Real-time collaboration (Post v1.0)
└── [OrdoPlay Engine Crates] # Via path dependency (future)
    ├── ordoplay_render      # Viewport rendering
    ├── ordoplay_scene       # Scene serialization
    ├── ordoplay_asset       # Asset pipeline
    ├── ordoplay_picking     # Selection system
    └── ordoplay_editor      # Undo/redo core
```

---

## Part 2: Release Milestones

### Milestone Overview

```
┌──────────────────────────────────────────────────────────────────────────┐
│ RELEASE TIMELINE                                                         │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Alpha 0.1 ──► Alpha 0.2 ──► Alpha 0.3 ──► Beta 0.5 ──► Beta 0.7      │
│  (Week 6)      (Week 12)     (Week 18)     (Week 26)    (Week 34)      │
│                                                                          │
│  Foundation    3D Render     Asset          Graphs &     Runtime        │
│  + Core Edit   + Scene Viz   Pipeline       Sequencer    Systems        │
│                                                                          │
│  ──► RC 0.9 ──► v1.0                                                    │
│     (Week 42)   (Week 48)                                               │
│                                                                          │
│     Polish &    Production                                              │
│     Stability   Release                                                 │
└──────────────────────────────────────────────────────────────────────────┘
```

### Alpha 0.1 - Foundation & Core Editing (Current -> Week 6)
- Wire all untracked modules into main app
- Fix all known bugs (quaternion conversion, debounce loss, child ID bug, etc.)
- Inspector: editable component properties (not read-only labels)
- Asset browser: functional rename/delete/show-in-explorer
- Console: real timestamps, tracing integration
- Zero compiler warnings, remove all `#![allow(dead_code)]`

### Alpha 0.2 - 3D Scene Visualization (Week 7-12)
- Mesh rendering in viewport (load and display 3D models)
- Directional/point/spot lights with shadow maps
- Entity visualization (selection outlines, bounding boxes)
- Per-entity bounding volume picking (not hardcoded radius)
- Native OS file dialogs (rfd crate)

### Alpha 0.3 - Asset Pipeline & Hot Reload (Week 13-18)
- Import pipeline with per-type settings
- File watcher integrated, hot reload functional
- Thumbnail generation for all asset types (models, audio, materials)
- Drag-drop assets to viewport/hierarchy
- Texture compression and mipmap generation

### Beta 0.5 - Creative Tools (Week 19-26)
- Material graph with WGSL compilation and live preview
- Gameplay graph with functional evaluation
- Sequencer with keyframe dragging, context menus, undo
- Animation state machine graph
- Prefab system with override application

### Beta 0.7 - Runtime Systems (Week 27-34)
- Scripting integration (embedded language)
- Input action mapping system
- Particle/VFX system (basic CPU emitter)
- Runtime UI/HUD system
- Build/export pipeline (single-platform)
- Play mode with physics + audio + script integration

### RC 0.9 - Scale & Polish (Week 35-42)
- Plugin/extension API
- Command palette, customizable shortcuts
- Real profiler integration (tracing instrumentation)
- Debug draw API, runtime entity inspector
- Performance optimization, memory limits on undo
- World partition system

### v1.0 - Production Release (Week 43-48)
- Documentation complete with tooltips
- Sample project (3D game vertical slice)
- CLI/headless build mode for CI
- All critical bugs resolved
- Performance benchmarks met
- Test coverage targets met

---

## Part 3: Detailed Phase Breakdown

### Phase 0: Foundation Completion [CURRENT - CRITICAL]
**Status**: Needs significant work
**Goal**: Wire everything together, fix all known bugs, make existing code production-quality

#### 0.1 Module Integration (BLOCKING)

| ID | Task | Priority | Status | Details |
|----|------|----------|--------|---------|
| F-001 | Wire untracked modules into main.rs | P0 | 🔲 | Declare audio, build, components, file_watcher, hot_reload, physics, play_mode, prefab, project as modules |
| F-002 | Remove all `#![allow(dead_code)]` | P0 | 🔲 | Every file uses this; remove and fix/delete genuinely dead code |
| F-003 | Fix TransformData quaternion bug | P0 | 🔲 | commands.rs stores euler angles with w=1.0 -- implement proper euler-to-quat conversion |
| F-004 | Fix hot_reload debounce bug | P0 | 🔲 | `poll()` uses `drain(..)` then filters, discarding events not past debounce window |
| F-005 | Fix prefab instantiate child ID bug | P0 | 🔲 | `instantiate()` returns EntityData.children with IDs that don't match actual children |
| F-006 | Fix DuplicateCommand child loss | P0 | 🔲 | Children cleared on duplicate (line 385/877) -- implement deep hierarchy duplication |
| F-007 | Fix console mock chrono | P0 | 🔲 | `Local::now().format()` always returns "00:00:00" -- use real timestamps |
| F-008 | Fix asset browser grid nav history | P0 | 🔲 | Grid view sets `current_path` directly, skipping `navigate_to()` history |
| F-009 | Fix inspector color picker | P0 | 🔲 | `color_edit_button_srgba_unmultiplied` writes to local array never applied back |
| F-010 | Delete dead code: render_grid_item | P0 | 🔲 | Superseded method in asset_browser.rs (lines 1146-1224) |
| F-011 | Delete dead code: DeviceExt trait | P0 | 🔲 | Unused trait in viewport_renderer.rs (lines 491-494) |
| F-012 | Fix `to_operation` dead code | P1 | 🔲 | Methods on all commands are never called -- either wire up or remove |

#### 0.2 Inspector: Editable Component Properties (CRITICAL GAP)

| ID | Task | Priority | Status | Details |
|----|------|----------|--------|---------|
| F-020 | Wire property_drawer.rs to inspector | P0 | 🔲 | Complete infrastructure exists (707 lines) with zero consumers |
| F-021 | Editable Light properties | P0 | 🔲 | Currently read-only labels: intensity, color, range, type, shadows |
| F-022 | Editable Camera properties | P0 | 🔲 | FOV, near/far, orthographic size |
| F-023 | Editable MeshRenderer properties | P0 | 🔲 | Mesh path, material path (with asset browser integration) |
| F-024 | Editable Rigidbody properties | P0 | 🔲 | Mass, drag, angular drag, use gravity, is kinematic, constraints |
| F-025 | Editable Collider properties | P0 | 🔲 | Size/radius/height for Box/Sphere/Capsule, is trigger, material |
| F-026 | Editable AudioSource properties | P0 | 🔲 | Clip, volume, pitch, spatial, loop, play on awake |
| F-027 | Editable Script properties | P0 | 🔲 | Script path, enabled flag |
| F-028 | Component property undo support | P0 | 🔲 | PropertyEditCommand only handles Transform/name/active/is_static -- extend to all component fields |

#### 0.3 Asset Browser Completion

| ID | Task | Priority | Status | Details |
|----|------|----------|--------|---------|
| F-030 | Implement "Show in Explorer" | P0 | 🔲 | Context menu item currently does nothing |
| F-031 | Implement "Rename" | P0 | 🔲 | Context menu item currently does nothing |
| F-032 | Implement "Delete" | P0 | 🔲 | Context menu item currently does nothing (with confirmation dialog) |
| F-033 | Persist favorites/recent | P1 | 🔲 | Currently memory-only, lost on restart |
| F-034 | Drag-drop to viewport/hierarchy | P1 | 🔲 | `dragging_asset` field exists but not functional |

#### 0.4 Console Integration

| ID | Task | Priority | Status | Details |
|----|------|----------|--------|---------|
| F-040 | Wire tracing subscriber to console | P0 | 🔲 | Console only receives logs through explicit `log()` calls, not tracing output |
| F-041 | Real timestamps | P0 | 🔲 | Replace mock chrono module with actual time formatting |
| F-042 | Remove unused fields | P1 | 🔲 | `min_level`, `pending_jump`, `LogLevel::name()` are dead |

#### 0.5 Editor Shell Fixes

| ID | Task | Priority | Status | Details |
|----|------|----------|--------|---------|
| F-050 | Implement Cut/Copy/Paste | P0 | 🔲 | Menu items exist but are no-ops in app.rs (lines 956-964) |
| F-051 | Wire Tools menu items | P0 | 🔲 | Material Editor/Gameplay Graph/Sequencer/Profiler buttons do nothing |
| F-052 | Handle CloseRequested properly | P0 | 🔲 | Currently exits immediately without checking unsaved changes |
| F-053 | Idle optimization | P1 | 🔲 | `about_to_wait` requests continuous redraws regardless of changes |
| F-054 | Native file dialogs | P1 | 🔲 | Replace text input with OS file picker (rfd crate) |
| F-055 | Persist editor layout | P1 | 🔲 | Window positions, dock layout, theme, shortcuts lost on restart |
| F-056 | Persist theme settings | P1 | 🔲 | Theme resets to default every launch |
| F-057 | Persist shortcut customizations | P1 | 🔲 | No save/load for custom key bindings |

#### 0.6 Undo System Hardening

| ID | Task | Priority | Status | Details |
|----|------|----------|--------|---------|
| F-060 | Memory limit enforcement | P1 | 🔲 | `memory_used` tracked but never used to evict old operations |
| F-061 | Operation coalescing | P1 | 🔲 | Rapid property drag creates individual undo entries -- coalesce rapid same-property edits |
| F-062 | Fix `apply_snapshot` fragility | P1 | 🔲 | Trial-and-error deserialization trying multiple types sequentially |

**Definition of Done**:
- [ ] Zero compiler warnings with no `#![allow(dead_code)]` suppressions
- [ ] All untracked modules wired into main.rs and functional
- [ ] All component properties editable in inspector (not read-only labels)
- [ ] All asset browser context menu actions functional
- [ ] Console shows real tracing output with real timestamps
- [ ] All known bugs fixed (quaternion, debounce, child IDs, etc.)
- [ ] Unsaved changes dialog on close
- [ ] Can create, edit, save, and reload scenes with full undo/redo

---

### Phase 1: 3D Scene Visualization
**Target**: Alpha 0.2 release
**Prerequisite**: Phase 0 complete
**Goal**: Users can see their scene in 3D, not just a grid

> **Current state**: viewport_renderer.rs only draws XZ grid and RGB axis lines.
> There is no mesh rendering, no lighting, no entity visualization. This is the
> single largest gap preventing the editor from being useful.

#### 1.1 Mesh Rendering Pipeline

| ID | Task | Priority | Details |
|----|------|----------|---------|
| R-001 | GLTF/GLB model loading | P0 | Load 3D models via gltf crate, extract vertex/index buffers |
| R-002 | Mesh render pipeline | P0 | wgpu render pipeline with vertex/fragment shaders for triangle meshes |
| R-003 | PBR material support | P0 | Albedo, metallic, roughness, normal maps -- basic PBR shader |
| R-004 | Texture loading & binding | P0 | Load images as GPU textures, bind to materials |
| R-005 | Entity-to-mesh mapping | P0 | MeshRendererComponent drives what gets rendered per entity |
| R-006 | Transform matrix pipeline | P0 | Entity transform -> model matrix -> GPU uniform |
| R-007 | Frustum culling | P1 | Basic frustum test to skip off-screen entities |
| R-008 | Instanced rendering | P2 | Batch identical meshes for performance |

#### 1.2 Lighting System

| ID | Task | Priority | Details |
|----|------|----------|---------|
| R-010 | Directional light | P0 | Sun light with direction, color, intensity |
| R-011 | Point light | P0 | Omni light with position, range, attenuation |
| R-012 | Spot light | P1 | Cone light with angle, range |
| R-013 | Shadow mapping (directional) | P0 | Basic shadow map with cascade support |
| R-014 | Shadow mapping (point/spot) | P1 | Cube map / single map shadows |
| R-015 | Ambient/environment lighting | P0 | Skybox or ambient color for fill light |
| R-016 | Light component integration | P0 | LightComponent in inspector drives rendering |

#### 1.3 Editor Visualization

| ID | Task | Priority | Details |
|----|------|----------|---------|
| R-020 | Selection outline/highlight | P0 | Outline or tint for selected entities |
| R-021 | Bounding box display | P1 | Wireframe AABB for selected entities |
| R-022 | Light gizmos | P0 | Directional arrow, point sphere, spot cone icons |
| R-023 | Camera preview | P1 | Frustum wireframe for camera entities |
| R-024 | Physics collider debug vis | P1 | Wireframe collider shapes (use physics.rs debug viz) |
| R-025 | Grid improvements | P1 | Infinite grid shader, configurable spacing, fade with distance |
| R-026 | Per-entity AABB picking | P0 | Replace hardcoded radius=1.0 with actual mesh bounds |
| R-027 | Box/lasso selection | P1 | Drag rectangle in viewport to select entities |
| R-028 | MSAA anti-aliasing | P1 | Configurable MSAA (2x/4x/8x) |

#### 1.4 Camera System

| ID | Task | Priority | Details |
|----|------|----------|---------|
| R-030 | Camera component rendering | P0 | Render from camera entity's perspective (game view) |
| R-031 | Picture-in-picture preview | P1 | Small camera preview in corner of viewport |
| R-032 | Orthographic view modes | P1 | Top/Front/Right orthographic projections |

**Definition of Done**:
- [ ] Can place a GLTF mesh in scene and see it rendered with materials
- [ ] Lights illuminate the scene with shadows
- [ ] Selected entities have visual feedback (outline/highlight)
- [ ] Clicking entities in viewport selects them with proper bounds
- [ ] Editor camera orbits around scene content smoothly

---

### Phase 2: Asset Pipeline Integration
**Target**: Alpha 0.3 release

#### 2.1 Asset Browser Enhancement

| ID | Task | Priority | Details |
|----|------|----------|---------|
| A-001 | Create folder/file | P0 | Right-click -> New Folder, New Material, etc. |
| A-002 | Asset import dialog | P0 | Import external files with settings preview |
| A-003 | Asset preview panel | P1 | Dedicated preview for selected asset (3D spin for meshes) |
| A-004 | Drag-drop to viewport | P0 | Drag mesh/prefab from browser, place in scene |
| A-005 | Drag-drop to inspector | P0 | Drag asset onto mesh/material/audio fields |
| A-006 | Asset database | P1 | Indexed asset metadata for fast search and dependency tracking |

#### 2.2 Thumbnail System Completion

| ID | Task | Priority | Details |
|----|------|----------|---------|
| A-010 | 3D model thumbnails | P1 | Render mesh preview with basic lighting (requires Phase 1) |
| A-011 | Material thumbnails | P1 | Sphere preview with material applied |
| A-012 | Audio waveform thumbnails | P2 | Visual waveform representation |
| A-013 | Fix async thumbnail I/O | P1 | Current `async fn` does synchronous `fs::read` -- use tokio::fs |
| A-014 | Fix disk cache instantiation | P1 | New DiskCache instance created per request -- share single instance |
| A-015 | Fix extract_path_from_error | P1 | Always returns None -- failed thumbnails never update state |

#### 2.3 Import Pipeline

| ID | Task | Priority | Details |
|----|------|----------|---------|
| A-020 | Texture import | P0 | Compression (BC/ASTC/ETC2), mipmaps, sRGB, size limits |
| A-021 | Model import | P0 | GLTF/GLB/OBJ/FBX: scale, axis conversion, mesh optimization |
| A-022 | Audio import | P1 | Format conversion, compression, streaming flag |
| A-023 | Import settings UI | P0 | Per-asset-type configuration in inspector when asset selected |
| A-024 | Batch reimport | P1 | Reimport multiple assets with updated settings |
| A-025 | Import presets | P2 | Save/load import configurations |

#### 2.4 Hot Reload (Fix & Complete)

| ID | Task | Priority | Details |
|----|------|----------|---------|
| A-030 | Fix debounce event loss bug | P0 | `drain(..)` discards events not past debounce -- buffer instead |
| A-031 | Fix `watch_directory` replacement | P0 | Each call replaces watcher, losing previous watches |
| A-032 | Add rename event mapping | P1 | `FileEvent::Renamed` defined but never emitted from OS events |
| A-033 | Add ignore patterns | P1 | Skip .git, node_modules, temp files |
| A-034 | Texture hot reload | P0 | GPU texture re-upload on file change |
| A-035 | Shader hot reload | P0 | Recompile WGSL and rebuild pipeline |
| A-036 | Material hot reload | P0 | Update material parameters, refresh viewport |
| A-037 | Reload toast notifications | P1 | Visual feedback when assets reload |
| A-038 | Dependency-aware reload | P1 | Material change triggers re-render of objects using that material |

**Definition of Done**:
- [ ] Can import GLTF/GLB models with configurable settings
- [ ] Modifying a texture externally updates viewport within 500ms
- [ ] Drag-drop from asset browser places entity in scene
- [ ] All asset types have functional thumbnails
- [ ] Hot reload works without crashes or event loss

---

### Phase 3: Advanced Inspector & Property System
**Target**: Alpha 0.3 (parallel with Phase 2)

#### 3.1 Property System Integration

| ID | Task | Priority | Details |
|----|------|----------|---------|
| I-001 | Connect PropertyDrawer to Inspector | P0 | Replace hardcoded component labels with drawer-based editing |
| I-002 | Implement Inspectable trait | P0 | Add implementations for all 11 component types |
| I-003 | Fix draw_asset_ref browse button | P0 | TODO: "Open asset browser popup" -- implement asset picker |
| I-004 | Fix draw_f32 range bug | P1 | Range setting overwrites -- fix conditional logic |
| I-005 | Nested struct expansion | P0 | Recursive property tree for complex components |
| I-006 | Collection editing | P1 | Vec/HashMap add/remove/reorder with undo |
| I-007 | Enum dropdown rendering | P0 | Automatic ComboBox for enum fields (LightType, RigidbodyType, etc.) |
| I-008 | Property search/filter | P1 | Already exists for sections -- extend to individual properties |
| I-009 | Property copy/paste | P2 | Copy values between entities |
| I-010 | Reset to default (right-click) | P1 | Reset individual properties to component defaults |

#### 3.2 Component Management Enhancement

| ID | Task | Priority | Details |
|----|------|----------|---------|
| I-020 | Extend PropertyEditCommand | P0 | Support all component property types, not just Transform/name/active/is_static |
| I-021 | Component dependency validation | P1 | Warn if collider added without rigidbody |
| I-022 | Component copy/paste | P2 | Copy component between entities |

#### 3.3 Multi-Entity Editing Completion

| ID | Task | Priority | Details |
|----|------|----------|---------|
| I-030 | Multi-entity component editing | P1 | Edit shared component properties across selection |
| I-031 | Mixed-value display | P1 | Show "--" or mixed indicator when values differ |

**Definition of Done**:
- [ ] Every component property editable via appropriate widget (DragValue, ColorPicker, ComboBox, etc.)
- [ ] All edits go through undo system
- [ ] Asset reference fields have browse/drag-drop support
- [ ] Property_drawer infrastructure has zero unused code

---

### Phase 4: Prefab System
**Target**: Beta 0.5

#### 4.1 Fix Existing Implementation

| ID | Task | Priority | Details |
|----|------|----------|---------|
| P-001 | Fix instantiate child ID mismatch | P0 | Returned children IDs don't match actual instantiated children |
| P-002 | Implement override application | P0 | Overrides tracked but never applied to entity data |
| P-003 | Implement nested prefab detection | P0 | `from_entities()` has TODO: "detect nested prefabs" |
| P-004 | Fix `revert_property_from_prefab` | P0 | Only handles 4 property paths -- extend to all component properties |
| P-005 | Implement `reload_prefab` propagation | P1 | Currently reloads file but doesn't update existing instances |

#### 4.2 Prefab Features

| ID | Task | Priority | Details |
|----|------|----------|---------|
| P-010 | Drag prefab to viewport/hierarchy | P0 | Instantiate on drop with preview |
| P-011 | Override visualization in inspector | P0 | Already partially implemented (orange bold) -- complete for all fields |
| P-012 | Nested prefab instantiation | P1 | Process NestedPrefabRef during instantiation |
| P-013 | Break prefab link | P1 | Convert instance to regular entities |
| P-014 | Prefab variants | P2 | Create prefab inheriting from another prefab |
| P-015 | Deep hierarchy duplication | P0 | Fix DuplicateCommand to preserve child hierarchies |

---

### Phase 5: Graph Framework
**Target**: Beta 0.5

#### 5.1 Fix Core Graph Interaction

| ID | Task | Priority | Details |
|----|------|----------|---------|
| G-001 | Wire connection creation from port drag | P0 | `hovered_port` detection exists but no code path enters `CreatingConnection` mode |
| G-002 | Implement right-click context menu | P0 | `_registry` parameter unused -- wire to `add_node_menu()` |
| G-003 | Implement connection hover detection | P0 | `hovered_connection` tracked but never set to non-None |
| G-004 | Implement cycle detection at connect time | P1 | TODO in graph.rs line 108 -- call `topological_order()` check |
| G-005 | Graph save/load | P0 | RON serialization for graph state |
| G-006 | Graph undo/redo | P0 | All node/connection operations undoable |
| G-007 | Copy/paste nodes | P1 | With connection preservation |
| G-008 | Node comments/annotations | P2 | Annotation boxes |
| G-009 | Node groups with collapse | P2 | Visual grouping |

#### 5.2 Node Evaluation Engine

| ID | Task | Priority | Details |
|----|------|----------|---------|
| G-010 | Implement NodeEvaluator trait for math nodes | P0 | Add, multiply, lerp, clamp, etc. -- zero implementations exist |
| G-011 | Implement NodeEvaluator for texture nodes | P0 | Texture sample, UV ops |
| G-012 | Implement NodeEvaluator for constant nodes | P0 | Float, Vec2/3/4, Color |
| G-013 | Implement NodeEvaluator for utility nodes | P1 | Split/combine, HSV, fresnel |
| G-014 | Implement NodeEvaluator for procedural nodes | P1 | Perlin, voronoi, checkerboard |

#### 5.3 Material Graph -> WGSL Compilation

| ID | Task | Priority | Details |
|----|------|----------|---------|
| G-020 | WGSL code generation from graph | P0 | Compile button currently does nothing (TODO line 895) |
| G-021 | Save material asset | P0 | Save button currently does nothing (TODO line 883) |
| G-022 | Live preview in viewport | P0 | Apply compiled material to preview mesh |
| G-023 | Inline node preview | P1 | Show intermediate result in node body |
| G-024 | Material templates | P1 | PBR, unlit, subsurface presets |
| G-025 | Error display on compile failure | P0 | Show shader compile errors in graph UI |

#### 5.4 Gameplay Graph (Visual Scripting) Completion

> **Current state**: Only 4 nodes exist (begin_play, tick, branch, print_string).
> A functional visual scripting system needs 40+ node types.

| ID | Task | Priority | Details |
|----|------|----------|---------|
| G-030 | Execution flow rendering | P0 | White execution wires distinct from data wires |
| G-031 | Loop nodes | P0 | For, ForEach, While, DoWhile |
| G-032 | Sequence node | P0 | Execute multiple outputs in order |
| G-033 | Variable get/set nodes | P0 | Local, entity, and global variables |
| G-034 | Math operation nodes | P0 | +, -, *, /, %, pow, sqrt, abs, min, max, clamp, lerp, sin, cos, atan2 |
| G-035 | Vector math nodes | P0 | Dot, cross, normalize, length, distance |
| G-036 | Comparison nodes | P0 | ==, !=, <, >, <=, >=, AND, OR, NOT |
| G-037 | Entity operation nodes | P0 | Get/set transform, find by name/tag, spawn, destroy |
| G-038 | Component access nodes | P0 | Get/set component properties, has component |
| G-039 | Input nodes | P1 | Key pressed, axis value, mouse position |
| G-040 | Physics nodes | P1 | Raycast, apply force/impulse, overlap test |
| G-041 | Audio nodes | P1 | Play sound, set volume |
| G-042 | Timer/delay nodes | P1 | Delay, interval, countdown |
| G-043 | String nodes | P1 | Concatenate, format, contains, split |
| G-044 | Debug nodes | P1 | Print, breakpoint, draw debug line/sphere |
| G-045 | Graph functions (subgraphs) | P2 | Reusable encapsulated graphs |
| G-046 | Gameplay evaluator | P0 | Execute gameplay graph at runtime |

#### 5.5 Animation Graph Completion

> **Current state**: Only 3 nodes (state, blend, output).

| ID | Task | Priority | Details |
|----|------|----------|---------|
| G-050 | Animation clip node | P1 | Play specific animation asset |
| G-051 | 1D blend space | P1 | Blend between clips based on parameter |
| G-052 | 2D blend space | P2 | Blend on two axes (direction + speed) |
| G-053 | State machine transitions | P1 | Conditional transitions between states |
| G-054 | Parameters (float/bool/trigger) | P1 | Drive transitions and blends |
| G-055 | Additive animation layers | P2 | Layer animations on top of base |
| G-056 | Preview window | P1 | Animated skeleton preview |
| G-057 | Animation evaluator | P1 | Evaluate graph to produce final pose |

#### 5.6 VFX Graph Completion

> **Current state**: Only 4 nodes (spawn, init_pos, init_vel, output).

| ID | Task | Priority | Details |
|----|------|----------|---------|
| G-060 | Lifetime module | P1 | Particle lifetime, kill conditions |
| G-061 | Force modules | P1 | Gravity, wind, turbulence, drag |
| G-062 | Size/color over life | P1 | Gradient curves |
| G-063 | Collision module | P2 | Particle-world collision |
| G-064 | Rendering modes | P1 | Billboard, stretched, mesh particles |
| G-065 | Sub-emitters | P2 | Spawn particles on events |
| G-066 | VFX evaluator (CPU) | P1 | Simulate and render particles |
| G-067 | GPU particle compute | P2 | Compute shader simulation |

---

### Phase 6: Sequencer & Timeline
**Target**: Beta 0.5

#### 6.1 Fix Existing Implementation

| ID | Task | Priority | Details |
|----|------|----------|---------|
| S-001 | Wire keyframe drag interaction | P0 | `DragOperation::Keyframes` declared but no code path enters it |
| S-002 | Wire box selection interaction | P0 | `DragOperation::BoxSelect` declared but never entered |
| S-003 | Implement right-click context menus | P0 | Add track, add keyframe, delete, interpolation mode |
| S-004 | Implement bezier tangent evaluation | P0 | Currently falls back to linear (line 233) |
| S-005 | Implement tangent handle editing | P0 | Curve editor shows points but no tangent handles |
| S-006 | Implement waveform rendering | P1 | `show_waveforms` flag exists but no rendering code |
| S-007 | Undo/redo integration | P0 | No undo for any sequencer operation |
| S-008 | Copy/paste keyframes | P1 | Not implemented |
| S-009 | Track reordering | P1 | Not implemented |
| S-010 | Sequencer save/load | P0 | RON dependency present but unused |

#### 6.2 Track Types

| ID | Task | Priority | Details |
|----|------|----------|---------|
| S-020 | Fix TransformTrack create_keyframe | P1 | Only stores position -- rotation/scale parameters ignored (underscore-prefixed) |
| S-021 | Audio track playback integration | P1 | AudioClip data exists but no audio engine hookup |
| S-022 | CameraTrack evaluate helpers | P1 | focus_distance and aperture channels have no evaluate methods |
| S-023 | Entity binding runtime integration | P1 | EntityBinding is data-only, not connected to scene entities |

#### 6.3 Sequencer Features

| ID | Task | Priority | Details |
|----|------|----------|---------|
| S-030 | Recording mode | P1 | Auto-key transforms as entities are moved |
| S-031 | Sequence asset save/load | P0 | Persist sequences as project assets |
| S-032 | Sub-sequences | P2 | Nested sequence references |
| S-033 | Sequence playback in viewport | P0 | Animate entities in viewport during playback |

---

### Phase 7: Runtime Systems (NEW - Previously Missing)

> **These systems are essential for v1.0 but were absent from the previous roadmap.**
> Without them, the editor cannot produce playable games.

#### 7.1 Scripting Integration

| ID | Task | Priority | Details |
|----|------|----------|---------|
| RT-001 | Embedded scripting language | P0 | Integrate Rhai or Lua as text-based scripting |
| RT-002 | Script editing in editor | P0 | Basic syntax-highlighted text editor panel |
| RT-003 | Script hot reload | P0 | Detect script file changes, reload without restart |
| RT-004 | Script API: entity access | P0 | Get/set transform, components, find entities |
| RT-005 | Script API: input | P0 | Check keys, mouse, gamepad from scripts |
| RT-006 | Script API: physics | P1 | Raycast, apply forces from scripts |
| RT-007 | Script API: audio | P1 | Play sounds, set volume from scripts |
| RT-008 | Script error display | P0 | Show script errors in console with line numbers |
| RT-009 | Script debugging | P2 | Breakpoints, variable inspection |

#### 7.2 Input System

| ID | Task | Priority | Details |
|----|------|----------|---------|
| RT-010 | Input action mapping | P0 | Abstract actions (Jump, Fire) bound to keys/buttons |
| RT-011 | Input mapping editor UI | P0 | Define actions in project settings (already partially in project.rs) |
| RT-012 | Gamepad support | P1 | Axis/button mapping for controllers |
| RT-013 | Input processing runtime | P0 | Poll/event-driven input state accessible from scripts/graphs |
| RT-014 | Runtime rebinding | P2 | Allow players to rebind keys at runtime |

#### 7.3 Particle/VFX System

| ID | Task | Priority | Details |
|----|------|----------|---------|
| RT-020 | ParticleSystem component | P0 | Add to components.rs (currently missing) |
| RT-021 | CPU particle emitter | P0 | Emit, simulate, render billboard particles |
| RT-022 | Basic modules | P0 | Lifetime, velocity, size over life, color over life |
| RT-023 | Viewport preview | P0 | See particles in editor viewport |
| RT-024 | Particle editor panel | P1 | Modify particle properties with live preview |
| RT-025 | GPU compute particles | P2 | Compute shader simulation for high counts |

#### 7.4 Runtime UI/HUD System

| ID | Task | Priority | Details |
|----|------|----------|---------|
| RT-030 | UI canvas system | P0 | Screen-space UI layer rendered on top of 3D |
| RT-031 | UI widget library | P0 | Text, Image, Button, Panel, ProgressBar |
| RT-032 | UI anchoring/layout | P0 | Anchor points, responsive sizing |
| RT-033 | UI event handling | P0 | Button clicks, hover, focus from scripts |
| RT-034 | UI editor panel | P1 | Visual layout editor for runtime UI |
| RT-035 | UI animation | P2 | Tweens, transitions for UI elements |

#### 7.5 Build & Export System

| ID | Task | Priority | Details |
|----|------|----------|---------|
| RT-040 | Fix build pipeline: real texture processing | P0 | Currently just copies files -- implement compression, mipmaps |
| RT-041 | Fix build pipeline: async/threaded | P0 | Currently blocking despite Arc<BuildState> infrastructure |
| RT-042 | Executable/bundle generation | P0 | Produce standalone runnable artifact for Windows |
| RT-043 | CLI/headless build mode | P0 | `ordoplay build --profile release --platform windows` for CI |
| RT-044 | Shader compilation pipeline | P0 | Pre-compile WGSL shaders for target platform |
| RT-045 | Asset cooking | P0 | Convert source assets to optimized runtime format |
| RT-046 | Incremental builds | P1 | Only reprocess changed assets |
| RT-047 | Build progress UI | P0 | Already exists in project_settings.rs -- wire to real build system |

#### 7.6 Play Mode Integration

| ID | Task | Priority | Details |
|----|------|----------|---------|
| RT-050 | Wire play_mode.rs to physics | P0 | Step physics simulation during play |
| RT-051 | Wire play_mode.rs to audio | P0 | Start/stop audio playback with play mode |
| RT-052 | Wire play_mode.rs to scripts | P0 | Execute script callbacks (start, update, physics) during play |
| RT-053 | Wire play_mode.rs to particles | P1 | Simulate particles during play |
| RT-054 | Runtime entity inspector | P1 | Inspect entity state while game is running |
| RT-055 | Debug draw API | P1 | Draw lines, spheres, text in viewport from scripts |
| RT-056 | Console commands at runtime | P1 | Execute commands while game plays |

#### 7.7 Camera System (Runtime)

| ID | Task | Priority | Details |
|----|------|----------|---------|
| RT-060 | Camera component rendering | P0 | Render scene from camera entity perspective |
| RT-061 | Active camera selection | P0 | Designate which camera is "main" |
| RT-062 | Camera follow/look-at behaviors | P1 | Basic camera scripts |
| RT-063 | Camera shake | P1 | Trauma-based screen shake |

---

### Phase 8: Physics System Hardening
**Target**: Beta 0.7

> **Current state**: physics.rs has a basic simulation but with critical
> limitations that make it unsuitable for production games.

| ID | Task | Priority | Details |
|----|------|----------|---------|
| PH-001 | Fix capsule collision | P0 | Currently simplified to sphere (line 667-672) |
| PH-002 | Implement OBB collision | P0 | Box-box currently AABB only -- rotated boxes collide incorrectly |
| PH-003 | Implement friction response | P0 | Friction values computed but never applied |
| PH-004 | Implement raycasting | P0 | Essential for gameplay (shooting, interaction, ground check) |
| PH-005 | Shape casting | P1 | Sweep test for character controllers |
| PH-006 | Broadphase optimization | P0 | Replace O(n^2) with spatial hash or BVH |
| PH-007 | Proper angular physics | P1 | Inertia tensor instead of scalar inv_mass |
| PH-008 | Joints/constraints | P1 | Hinge, spring, fixed, distance constraints |
| PH-009 | Sleeping/deactivation | P1 | Rest resting bodies to save CPU |
| PH-010 | Trigger exit events | P1 | Currently only enter events generated |
| PH-011 | Continuous collision detection | P2 | Prevent fast objects from tunneling |
| PH-012 | Mesh collider support | P2 | Triangle mesh collision detection |
| PH-013 | Physics material editor UI | P0 | Edit friction/restitution in inspector (already has PhysicsMaterialComponent) |
| PH-014 | Collider shape editing gizmos | P1 | Visual handles to resize colliders in viewport |

**Alternative**: Consider integrating Rapier physics library instead of building from scratch. Rapier provides production-grade rigid body dynamics, collision detection, and raycasting out of the box for Rust.

---

### Phase 9: Audio System Completion
**Target**: Beta 0.7

> **Current state**: audio.rs has rodio backend but is not wired into the app.

| ID | Task | Priority | Details |
|----|------|----------|---------|
| AU-001 | Wire audio.rs into main app | P0 | Declare module, initialize engine |
| AU-002 | Fix channel assignment | P0 | Hardcoded to SFx -- use AudioSourceComponent.channel or add field |
| AU-003 | Fix looping re-read | P1 | Re-opens file from disk on loop -- buffer in memory |
| AU-004 | Connect AudioSettings from project.rs | P0 | Max sources, doppler, speed of sound currently unused |
| AU-005 | Wire AudioListenerComponent | P0 | Find active listener entity each frame |
| AU-006 | 3D panning | P1 | Left/right panning based on listener orientation |
| AU-007 | Doppler effect | P2 | Use `doppler_scale` and `speed_of_sound` from settings |
| AU-008 | Audio mixer UI panel | P1 | Volume faders for master/music/sfx/voice/ambient |
| AU-009 | Audio preview in asset browser | P0 | Already partially implemented -- ensure functional |
| AU-010 | Reverb zones | P2 | Area-based reverb effect |

---

### Phase 10: Polish & Production Readiness
**Target**: RC 0.9 -> v1.0

#### 10.1 Command Palette

| ID | Task | Priority | Details |
|----|------|----------|---------|
| X-001 | Already implemented | - | Fuzzy search, keyboard nav, shortcut display all work |
| X-002 | Fix context-aware shortcuts | P0 | Only `NonTextInput` context checked -- wire `Viewport` and `Hierarchy` contexts |
| X-003 | Fix `reset_to_default` | P1 | Currently just removes customization, doesn't restore default binding |
| X-004 | Fix text focus detection | P1 | `focused().is_some()` is too broad -- blocks shortcuts for non-text widgets |
| X-005 | Persist shortcut customizations | P1 | Save/load to file |

#### 10.2 Profiler: Replace Mock Data

| ID | Task | Priority | Details |
|----|------|----------|---------|
| X-010 | Wire tracing spans to profiler | P0 | All profiling data is currently fake/hardcoded |
| X-011 | Real frame timing | P0 | Capture actual frame durations |
| X-012 | Real CPU scope measurement | P0 | Instrument key code paths with tracing spans |
| X-013 | Real GPU timing queries | P1 | wgpu timestamp queries |
| X-014 | Real memory tracking | P1 | Allocation counters instead of "1.8 GB" hardcoded |
| X-015 | Real draw call/triangle counting | P1 | Count actual render operations |
| X-016 | Remove fake stats | P0 | Delete all hardcoded values ("1,234 draw calls", "2.4M triangles", etc.) |
| X-017 | Remove unused fields | P0 | `show_trace`, `show_debug`, `show_info` in profiler are dead |

#### 10.3 Project Settings Completion

| ID | Task | Priority | Details |
|----|------|----------|---------|
| X-020 | Add Scene button | P1 | "Add Current Scene" to build scene list |
| X-021 | Revert/Reset to Defaults | P1 | Reset settings to factory defaults |
| X-022 | Mouse/joystick input config | P1 | Only keyboard axis config exists currently |
| X-023 | Quality level presets | P1 | Define what Low/Medium/High/Ultra mean |
| X-024 | Settings validation | P1 | Prevent invalid resolution, negative timesteps, etc. |

#### 10.4 Theme System Completion

| ID | Task | Priority | Details |
|----|------|----------|---------|
| X-030 | Wire theme colors to viewport | P0 | Viewport renderer uses hardcoded colors, not theme's `grid_color`, `axis_x/y/z` |
| X-031 | Wire semantic colors | P1 | `success`, `warning`, `error`, `info` defined but never read |
| X-032 | Theme persistence | P0 | Save/load to file |
| X-033 | Full custom color editor | P1 | `ThemePreset::Custom` only edits accent currently |

#### 10.5 Editor Persistence

| ID | Task | Priority | Details |
|----|------|----------|---------|
| X-040 | Save/restore window layout | P0 | Dock configuration, panel sizes |
| X-041 | Save/restore editor state | P0 | Recent scenes, last project, window position |
| X-042 | Autosave | P0 | Periodic scene autosave with recovery |
| X-043 | Crash recovery | P1 | Detect unclean shutdown, offer recovery |

#### 10.6 Documentation & Onboarding

| ID | Task | Priority | Details |
|----|------|----------|---------|
| X-050 | Tooltips on all UI elements | P0 | Every button, field, and control should have a tooltip |
| X-051 | Sample project | P0 | 3D game vertical slice demonstrating all features |
| X-052 | Online documentation | P0 | User guide, API reference, tutorials |
| X-053 | Remove false claims from main.rs | P0 | Doc comment mentions "Nanite, Lumen" which don't exist |
| X-054 | In-editor help system | P1 | F1 context-sensitive help |

#### 10.7 Testing Infrastructure

| ID | Task | Priority | Details |
|----|------|----------|---------|
| X-060 | Scene save/load round-trip tests | P0 | Save scene, reload, verify identical |
| X-061 | Undo/redo comprehensive tests | P0 | Every command type: execute, undo, verify state restored |
| X-062 | Graph operation tests | P0 | Add/remove/connect nodes, verify graph integrity |
| X-063 | Prefab instantiation tests | P0 | Create, instantiate, override, verify correctness |
| X-064 | Hot reload tests | P1 | Simulate file changes, verify reload triggered |
| X-065 | CI pipeline (GitHub Actions) | P0 | Build + test on every commit, Windows + Linux |
| X-066 | Performance regression tests | P1 | Automated benchmarks for scene load, undo, render frame |

---

### Phase 11: World Building at Scale
**Target**: RC 0.9

#### 11.1 World Partition

| ID | Task | Priority | Details |
|----|------|----------|---------|
| W-001 | Grid-based world division | P1 | Configurable cell size |
| W-002 | Entity-to-cell assignment | P1 | Automatic based on position |
| W-003 | Cell file format | P1 | Separate files per cell |
| W-004 | Editor cell loading | P1 | Load/unload based on camera |
| W-005 | Cell boundary visualization | P1 | Grid overlay in viewport |
| W-006 | Cell statistics | P2 | Entity count, memory per cell |

#### 11.2 Data Layers

| ID | Task | Priority | Details |
|----|------|----------|---------|
| W-007 | Layer definition | P1 | Named layers with properties |
| W-008 | Entity layer assignment | P1 | Tag entities to layers |
| W-009 | Layer visibility toggle | P1 | Editor layer on/off |
| W-010 | Runtime layer loading | P1 | API for gameplay layer streaming |

#### 11.3 Scene Management

| ID | Task | Priority | Details |
|----|------|----------|---------|
| W-020 | Additive scene loading (runtime) | P0 | Load additional scenes without unloading current |
| W-021 | Scene transition API | P1 | Fade/crossfade between scenes |
| W-022 | Multi-scene editing (editor) | P2 | View/edit multiple scenes simultaneously |

#### 11.4 HLOD Pipeline

| ID | Task | Priority | Details |
|----|------|----------|---------|
| W-030 | HLOD cluster generation | P2 | Automatic spatial grouping |
| W-031 | Proxy mesh generation | P2 | Simplified merged meshes |
| W-032 | HLOD LOD transitions | P2 | Smooth switching |

---

### Phase 12: Plugin/Extension System
**Target**: RC 0.9 -> v1.0

| ID | Task | Priority | Details |
|----|------|----------|---------|
| PL-001 | Plugin trait definition | P0 | Interface for adding panels, components, asset types, menu items |
| PL-002 | Plugin loading | P0 | Dynamic (dylib) or compile-time (cargo features) plugin loading |
| PL-003 | Plugin lifecycle | P0 | init, update, shutdown callbacks |
| PL-004 | Custom panel registration | P0 | Plugins can add new dock panels |
| PL-005 | Custom component registration | P0 | Plugins can define new component types |
| PL-006 | Custom asset type registration | P0 | Plugins can define new importable asset types |
| PL-007 | Plugin configuration UI | P1 | Enable/disable plugins in project settings |
| PL-008 | Plugin API documentation | P0 | How to write a plugin |

---

### Phase 13: Collaboration System
**Target**: Post v1.0 (v1.5)

#### 13.1 CRDT Implementation

| ID | Task | Priority | Details |
|----|------|----------|---------|
| C-001 | LWW Register | P2 | Last-writer-wins for properties |
| C-002 | OR-Set | P2 | Observed-remove set for entities |
| C-003 | Operation serialization | P2 | Binary operation format |
| C-004 | Conflict resolution | P2 | Deterministic merge strategies |

#### 13.2 Networking

| ID | Task | Priority | Details |
|----|------|----------|---------|
| C-005 | WebSocket server | P2 | Local network collaboration |
| C-006 | Client connection | P2 | Join/leave session |
| C-007 | Operation broadcast | P2 | Fan-out to all clients |
| C-008 | State sync on connect | P2 | Full state transfer for new clients |

#### 13.3 Presence & UX

| ID | Task | Priority | Details |
|----|------|----------|---------|
| C-009 | User cursors | P2 | See other users' selections |
| C-010 | User colors | P2 | Unique color per collaborator |
| C-011 | Selection sharing | P2 | Highlight what others have selected |
| C-012 | User list panel | P2 | Active collaborators list |

---

## Part 4: Quality Gates

### Performance Requirements

| Metric | Target | Measurement |
|--------|--------|-------------|
| UI Frame Time | < 16ms (60 FPS) | egui frame timing |
| Input Latency | < 50ms | Time from input to visual update |
| Scene Load (1000 entities) | < 2s | Cold load time |
| Undo Operation | < 10ms | CoW snapshot restore |
| Asset Hot Reload | < 500ms | File change to viewport update |
| Memory (empty project) | < 200MB | Baseline RAM usage |
| Memory (large project) | < 2GB | 10,000 entities, 1000 assets |
| Viewport Render (1000 entities) | < 16ms | GPU frame time |
| Build time (small project) | < 30s | 100 assets, single platform |

### Stability Requirements

| Requirement | Target |
|-------------|--------|
| Crash-free sessions | 99.9% |
| Data loss prevention | 100% (autosave every 60s) |
| Undo reliability | 100% operations reversible |
| File format compatibility | Forward compatible for 2 major versions |
| Unsaved changes protection | Never lose work on close/crash |

### Test Coverage

| Category | Target Coverage |
|----------|-----------------|
| Unit tests | 80% |
| Integration tests | 60% |
| UI interaction tests | 40% |
| Property-based tests (fuzzing) | Key serialization paths |
| Scene round-trip tests | 100% of entity/component combinations |

---

## Part 5: SOTA Enhancements Beyond Industry Standard

### 5.1 Advanced Rendering (Post v1.0)

| Feature | Description | Priority |
|---------|-------------|----------|
| Path Tracing Preview | Reference quality rendering in viewport | P2 |
| Virtual Geometry (Nanite-style) | Automatic LOD in viewport | P3 |
| Global Illumination | Real-time GI approximation (SDFGI/probe-based) | P2 |
| Material Baking | Bake complex materials to textures | P2 |
| Volumetric Fog | Atmospheric effects | P2 |
| Light Probes | Baked indirect lighting for dynamic objects | P2 |
| Reflection Probes | Cubemap-based reflections | P1 |

### 5.2 Terrain System (Post v1.0)

| Feature | Description | Priority |
|---------|-------------|----------|
| Heightmap sculpting | Raise/lower/smooth/flatten brushes | P2 |
| Paint layers | Multi-texture terrain with splatmap | P2 |
| Foliage placement | Scatter grass/trees/rocks | P2 |
| Terrain LOD | Distance-based mesh simplification | P2 |
| Erosion tools | Hydraulic/thermal erosion simulation | P3 |

### 5.3 Navigation/AI (Post v1.0)

| Feature | Description | Priority |
|---------|-------------|----------|
| NavMesh generation | Recast-based navigation mesh | P2 |
| NavMesh agent component | Pathfinding for entities | P2 |
| NavMesh visualization | Debug display in editor | P2 |
| Behavior tree editor | Visual behavior authoring | P3 |

### 5.4 Cross-Platform & Cloud

| Feature | Description | Priority |
|---------|-------------|----------|
| macOS Support | Native Metal rendering via wgpu | P1 |
| Linux Support | Vulkan rendering via wgpu | P1 |
| Android/iOS export | Mobile build targets | P2 |
| Web Editor (WASM) | Browser-based light editor | P3 |
| Cloud Build | Remote asset processing | P3 |

### 5.5 Accessibility

| Feature | Description | Priority |
|---------|-------------|----------|
| Keyboard Navigation | Full keyboard-only operation | P1 |
| High Contrast Mode | Already implemented (ThemePreset::HighContrastDark) | ✅ |
| Colorblind Modes | Deuteranopia, protanopia support | P2 |
| Customizable Font Size | UI scaling (partially in theme.rs) | P1 |
| Screen Reader Support | egui AccessKit integration | P3 |

### 5.6 Localization

| Feature | Description | Priority |
|---------|-------------|----------|
| i18n Framework | String externalization | P2 |
| English (default) | Base language | P0 |
| Chinese (Simplified) | Major market | P2 |
| Japanese | Major market | P2 |

### 5.7 Version Control Integration

| Feature | Description | Priority |
|---------|-------------|----------|
| VCS status in asset browser | Show modified/added/ignored icons | P2 |
| .gitignore generation | Auto-generate for OrdoPlay projects | P1 |
| Asset locking | Prevent concurrent binary edits | P3 |

### 5.8 AI-Assisted Development (Future)

| Feature | Description | Priority |
|---------|-------------|----------|
| Natural Language Commands | "Create a red cube at origin" | P3 |
| Code Generation | Generate gameplay scripts from description | P3 |
| Asset Generation | AI-powered texture/model generation | P3 |

---

## Part 6: Risk Mitigation

### Technical Risks

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| egui performance at scale | High | Medium | Profile early, optimize hot paths, consider partial rendering, batch draw calls |
| Viewport renderer complexity | High | High | Consider integrating rend3 or bevy_render rather than building from scratch |
| Physics engine completeness | High | High | Consider integrating Rapier instead of custom physics |
| Graph editor complexity | Medium | High | Modular architecture, extensive testing |
| CRDT edge cases | High | Medium | Defer to post-v1.0, use formal verification when implemented |
| Scripting language integration | Medium | Medium | Start with Rhai (pure Rust) for simplest integration |
| Cross-platform compatibility | Medium | Low | wgpu handles most platform differences; test early on macOS/Linux |
| Breaking changes in dependencies | Low | High | Pin versions, maintain forks if needed |

### Schedule Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Feature creep | High | Strict milestone scope, defer P2+ |
| 3D renderer scope | High | Start with basic forward renderer, iterate |
| Integration complexity (9 unwired modules) | High | Wire them incrementally in Phase 0, test each |
| Performance regressions | Medium | Automated benchmarks in CI |

### Build-vs-Buy Decisions

| Component | Build | Buy/Integrate | Recommendation |
|-----------|-------|---------------|----------------|
| Physics | physics.rs (custom, limited) | Rapier (production-grade Rust) | **Rapier** -- saves months of work |
| 3D Rendering | viewport_renderer.rs (grid only) | rend3, wgpu-based custom | **Custom wgpu** -- need tight editor control |
| Audio | audio.rs (rodio, basic) | rodio is fine for MVP | **Keep rodio**, consider kira later |
| Scripting | Not started | Rhai (pure Rust) or mlua (Lua) | **Rhai** for simplicity, **Lua** for ecosystem |
| NavMesh | Not started | recastnavigation-rs | **Integrate** when needed (post-v1.0) |

---

## Part 7: Success Metrics

### v1.0 Release Criteria

- [ ] All P0 and P1 tasks complete across Phases 0-12
- [ ] Zero known crash bugs
- [ ] Performance targets met
- [ ] Documentation coverage > 80% (tooltips, API docs, user guide)
- [ ] Test coverage targets met
- [ ] Successful creation of sample project (3D game vertical slice)
- [ ] Can build and export a standalone playable game
- [ ] Community beta feedback addressed
- [ ] No mock/fake/hardcoded data in any panel (profiler, console, etc.)
- [ ] No read-only-label component properties in inspector
- [ ] No stub context menu items
- [ ] No unimplemented button handlers

### Post-Release Metrics

| Metric | Target (6 months post-release) |
|--------|--------------------------------|
| Active users | 1,000+ |
| GitHub stars | 500+ |
| Community plugins | 10+ |
| Bug reports resolved | < 7 day average |
| Documentation satisfaction | 4/5 rating |

---

## Appendix A: Known Bugs & Technical Debt Registry

### Critical Bugs

| # | File | Line | Description | Phase |
|---|------|------|-------------|-------|
| B-001 | commands.rs | 70-77 | TransformData quaternion: stores euler angles with w=1.0 -- invalid quaternion | Phase 0 |
| B-002 | hot_reload.rs | 217-219 | `drain(..)` discards events not past debounce window | Phase 0 |
| B-003 | prefab.rs | 182 | `instantiate()` child IDs don't match recursive instantiation | Phase 0 |
| B-004 | commands.rs | 385/877 | DuplicateCommand clears children -- hierarchy lost | Phase 0 |
| B-005 | inspector.rs | - | Color picker writes to local array, never applied to entity | Phase 0 |
| B-006 | asset_browser.rs | 1043 | Grid view nav sets `current_path` directly, skips history | Phase 0 |

### Dead Code

| # | File | Lines | Description | Action |
|---|------|-------|-------------|--------|
| D-001 | asset_browser.rs | 1146-1224 | `render_grid_item` method superseded | Delete |
| D-002 | viewport_renderer.rs | 491-494 | `DeviceExt` trait shadowing real import | Delete |
| D-003 | console.rs | - | `min_level`, `pending_jump` fields | Delete |
| D-004 | profiler.rs | - | `show_trace`, `show_debug`, `show_info` fields | Delete |
| D-005 | tools.rs | - | `GizmoOperation` struct, `AxisConstraint` enum | Wire up or delete |
| D-006 | menus.rs | - | Multiple unused methods on Command, Shortcut, ShortcutRegistry | Wire up or delete |
| D-007 | graph/ui.rs | - | `hovered_connection` tracked but never set | Implement or delete |
| D-008 | play_mode.rs | - | `maximize_on_play`, `was_maximized` fields | Wire up or delete |

### Stub/No-Op Functions

| # | File | Description | Phase |
|---|------|-------------|-------|
| S-001 | app.rs:956-964 | Cut/Copy/Paste menu items are no-ops | Phase 0 |
| S-002 | app.rs:1038-1054 | Tools menu items don't open panels | Phase 0 |
| S-003 | app.rs:1057-1065 | Help > Documentation and About do nothing | Phase 10 |
| S-004 | app.rs:1317 | `entity.rename` just logs, no UI triggered | Phase 0 |
| S-005 | asset_browser.rs | Show in Explorer, Rename, Delete stubs | Phase 0 |
| S-006 | console.rs | `reload` command logs fake success | Phase 2 |
| S-007 | material.rs:883 | Save button no-op (TODO) | Phase 5 |
| S-008 | material.rs:895 | Compile button no-op (TODO) | Phase 5 |
| S-009 | profiler.rs | All data is mock/hardcoded | Phase 10 |
| S-010 | build.rs:359-371 | `process_texture` just copies files | Phase 7 |

### `#![allow(dead_code)]` Suppressions

Every one of these must be removed in Phase 0, and the underlying dead code either used or deleted:

- `crates/ordoplay_editor_app/src/state.rs`
- `crates/ordoplay_editor_app/src/commands.rs`
- `crates/ordoplay_editor_app/src/history.rs`
- `crates/ordoplay_editor_app/src/menus.rs`
- `crates/ordoplay_editor_app/src/tools.rs`
- `crates/ordoplay_editor_app/src/theme.rs`
- `crates/ordoplay_editor_app/src/thumbnail.rs`
- `crates/ordoplay_editor_app/src/panels/viewport.rs`
- `crates/ordoplay_editor_app/src/panels/property_drawer.rs`
- `crates/ordoplay_editor_app/src/audio.rs`
- `crates/ordoplay_editor_app/src/build.rs`
- `crates/ordoplay_editor_app/src/components.rs`
- `crates/ordoplay_editor_app/src/file_watcher.rs`
- `crates/ordoplay_editor_app/src/hot_reload.rs`
- `crates/ordoplay_editor_app/src/physics.rs`
- `crates/ordoplay_editor_app/src/project.rs`

---

## Appendix B: Keyboard Shortcuts Reference

### Global

| Shortcut | Action |
|----------|--------|
| Ctrl+N | New Scene |
| Ctrl+O | Open Scene |
| Ctrl+S | Save Scene |
| Ctrl+Shift+S | Save Scene As |
| Ctrl+Z | Undo |
| Ctrl+Y / Ctrl+Shift+Z | Redo |
| Ctrl+P | Command Palette |
| Ctrl+, | Settings |
| F1 | Help |
| F5 | Play |
| F6 | Pause |
| Escape | Stop |

### Viewport

| Shortcut | Action |
|----------|--------|
| W | Translate Mode |
| E | Rotate Mode |
| R | Scale Mode |
| F | Focus Selection |
| G | Toggle Grid |
| Alt+LMB | Orbit Camera |
| RMB Drag | Orbit Camera |
| MMB | Pan Camera |
| Scroll | Zoom Camera |
| Delete | Delete Selection |
| Ctrl+D | Duplicate Selection |
| Ctrl+G | Group Selection |

### Hierarchy

| Shortcut | Action |
|----------|--------|
| Ctrl+Shift+N | New Entity |
| F2 | Rename |
| Ctrl+C | Copy |
| Ctrl+V | Paste |
| Ctrl+X | Cut |

### Graph Editor

| Shortcut | Action |
|----------|--------|
| Tab / Space | Add Node Menu |
| Ctrl+C | Copy Nodes |
| Ctrl+V | Paste Nodes |
| Delete | Delete Selection |
| Home | Fit All |
| F | Focus Selection |

### Sequencer

| Shortcut | Action |
|----------|--------|
| Space | Play/Pause |
| K | Add Keyframe |
| Home | Go to Start |
| End | Go to End |
| , | Previous Frame |
| . | Next Frame |

---

## Appendix C: File Format Specifications

### Scene File (.ordoscene)

```ron
// Human-readable RON format
Scene(
    version: 2,
    name: "My Level",
    entities: [
        Entity(
            id: "550e8400-e29b-41d4-a716-446655440000",
            name: "Player",
            transform: Transform(
                position: Vec3(0.0, 1.0, 0.0),
                rotation: Quat(0.0, 0.0, 0.0, 1.0),
                scale: Vec3(1.0, 1.0, 1.0),
            ),
            components: [
                Component("MeshRenderer", {
                    "mesh": Asset("meshes/character.glb"),
                    "material": Asset("materials/player.ordomat"),
                }),
            ],
            children: ["child-uuid-here"],
        ),
    ],
    settings: SceneSettings(
        ambient_color: Color(0.1, 0.1, 0.15, 1.0),
        skybox: Asset("textures/sky_hdri.exr"),
    ),
)
```

### Material Graph (.ordomat)

```ron
MaterialGraph(
    version: 1,
    nodes: [
        Node(id: 0, type: "Output", position: Vec2(500, 200)),
        Node(id: 1, type: "TextureSample", position: Vec2(200, 150),
             data: { "texture": Asset("textures/brick_albedo.png") }),
        Node(id: 2, type: "TextureSample", position: Vec2(200, 300),
             data: { "texture": Asset("textures/brick_normal.png") }),
    ],
    connections: [
        Connection(from: (1, "rgb"), to: (0, "albedo")),
        Connection(from: (2, "rgb"), to: (0, "normal")),
    ],
)
```

### Project Settings (.ordoproject)

```ron
ProjectSettings(
    version: 1,
    metadata: ProjectMetadata(
        name: "My Game",
        version: "0.1.0",
        company: "Studio Name",
    ),
    physics: PhysicsSettings(
        gravity: Vec3(0.0, -9.81, 0.0),
        fixed_timestep: 0.02,
    ),
    graphics: GraphicsSettings(
        quality: High,
        vsync: true,
        msaa: 4,
    ),
    input: InputSettings(
        axes: [
            InputAxis(name: "Horizontal", positive: "D", negative: "A"),
            InputAxis(name: "Vertical", positive: "W", negative: "S"),
        ],
    ),
)
```

---

## Appendix D: Glossary

| Term | Definition |
|------|------------|
| AABB | Axis-Aligned Bounding Box - simplest collision shape |
| CoW | Copy-on-Write - memory optimization for undo snapshots |
| CRDT | Conflict-free Replicated Data Type - for real-time collaboration |
| HLOD | Hierarchical Level of Detail - for large world optimization |
| OBB | Oriented Bounding Box - rotation-aware collision shape |
| PBR | Physically Based Rendering - realistic material model |
| RON | Rusty Object Notation - human-readable serialization format |
| WGSL | WebGPU Shading Language - shader language for wgpu |
| egui | Immediate-mode GUI library for Rust |
| wgpu | Cross-platform GPU abstraction over Vulkan/Metal/DX12/WebGPU |
| Rapier | Production-grade Rust physics engine (recommended integration) |
| Rhai | Embedded scripting language for Rust |

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-01-17 | Initial task list and design document |
| 2.0 | 2026-01-20 | Consolidated roadmap with SOTA enhancements |
| 3.0 | 2026-01-29 | **Full codebase audit**: corrected all completion percentages, added bug registry, added 8 missing phases (3D rendering, runtime systems, physics hardening, audio, build/export, scripting, input, particles/UI), registered all dead code / stubs / no-ops, added build-vs-buy analysis |

---

*This document is the authoritative source for OrdoPlayEditor development planning. All contributors should reference this document for feature scope, priorities, and implementation guidelines.*
