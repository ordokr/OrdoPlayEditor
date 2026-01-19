# OrdoPlay Editor: SOTA Design Document

## Executive Summary

This document specifies the design for **OrdoPlay Editor** - a state-of-the-art, future-proof game engine editor that amalgamates the best features from Unreal Engine 5, Unity 6, Godot 4, O3DE, CryEngine Sandbox, and emerging open-source solutions. The architecture prioritizes leveraging existing OrdoPlay source code verbatim, with focused adaptations where necessary.

**Architecture Decision: Standalone Workspace**

After analyzing the OrdoPlay codebase (69 crates, comprehensive rendering/UI infrastructure) and industry best practices, the recommendation is:

- **Location**: `C:\Users\Administrator\Desktop\OrdoPlayEditor\` (standalone workspace)
- **Shared Crates**: `ordoplay_editor_core`, `ordoplay_editor_schema` live in main OrdoPlay workspace
- **Editor App**: Standalone binary with its own Cargo.toml referencing OrdoPlay crates via path

This approach provides:
- Clean separation between shipping runtime and editor code
- Faster editor-specific iteration without rebuilding the engine
- No UI/editor dependencies polluting the runtime build
- The ability to independently version and release the editor

---

## Part 1: Research Synthesis - SOTA Editor Features (2026)

### 1.1 Feature Matrix: Best-in-Class by Engine

| Feature Domain | Unreal Engine 5 | Unity 6 | Godot 4 | O3DE | CryEngine | Best Implementation |
|----------------|-----------------|---------|---------|------|-----------|---------------------|
| **World Building** | World Partition + Data Layers | Subscenes + Prefabs | Scene/Node Tree | World Editor + Prefabs | Level Editor | UE5 World Partition |
| **Undo/Redo** | Transaction-based grouping | Deep serialization | Command pattern | Transaction system | CoW snapshots | UE5 Transactions |
| **Visual Scripting** | Blueprints (node graph) | Visual Scripting (deprecated) | GDScript + VisualScript | Script Canvas | Flow Graph | UE5 Blueprints |
| **Material Authoring** | Material Editor | Shader Graph | Visual Shader | Material Editor | Material Editor | Unity Shader Graph |
| **Animation** | Sequencer + Control Rig | Timeline + Animator | AnimationPlayer + AnimationTree | Animation Editor | Track View | UE5 Sequencer |
| **Inspector** | Details Panel + Property Editor | Inspector + Custom Drawers | Inspector + @export | Entity Inspector | Property Panel | Godot Inspector |
| **Asset Pipeline** | Asset Import + DDC | Asset Pipeline + Addressables | Import Dock + Resources | Asset Processor | Asset Browser | Unity Asset Pipeline |
| **Profiling** | Unreal Insights | Unity Profiler | Debugger + Profiler | Profiler | - | UE5 Insights |
| **Collaboration** | UEFN Multi-user | Plastic SCM Integration | - | Multi-user | - | UEFN Collaboration |
| **Editor Extensibility** | Editor Utility Widgets | UI Toolkit + Editor Window | @tool scripts + EditorPlugin | Gems + Python | Plugins | Unity UI Toolkit |
| **Docking/Layout** | Configurable tabs | Dockable panels | 4-panel default | Qt-based docking | Full docking | CryEngine Docking |

### 1.2 Critical SOTA Capabilities to Implement

Based on research from [Game Engine Architecture](https://www.gameenginebook.com/), [O3DE Documentation](https://docs.o3de.org/), and industry trends:

1. **Time-to-Feedback Optimization**
   - Hot reload for assets (shaders, textures, materials, scenes)
   - Live preview in viewport during property edits
   - Instant play/stop with preserved state (Godot-style)

2. **Non-Destructive Workflows**
   - Prefabs with nested overrides (Unity/O3DE pattern)
   - Full undo/redo with transaction grouping
   - "Preview/Apply" mode for dangerous operations

3. **Multi-Scale World Support**
   - World partitioning with cell streaming (UE5 pattern)
   - Data layers for runtime/editor asset separation
   - HLOD generation pipeline

4. **Graph-Based Authoring**
   - Unified graph framework for all domains
   - Shader graphs, gameplay graphs, animation graphs
   - Visual state machines and behavior trees

5. **Real-Time Collaboration** (Future-Proof Priority)
   - CRDT-based scene editing for conflict resolution
   - Presence indicators and collaborator tracking
   - Comment/annotation system in-viewport

---

## Part 2: OrdoPlay Existing Infrastructure Leverage

### 2.1 Available Foundation (Copy Verbatim)

| OrdoPlay Crate | Editor Leverage | Adaptation Needed |
|----------------|-----------------|-------------------|
| `ordoplay_editor` | **Undo/Redo System** - CoW-based History with OperationGroups | Add ECS command integration |
| `ordoplay_ui` | **Full UI Framework** - Flexbox/Grid via Taffy, widgets, picking | Build docking layer on top |
| `ordoplay_picking` | **Selection System** - Pointer events, hit testing, bubbling | Add gizmo picking backend |
| `ordoplay_gizmos` | **Transform Gizmos** - Lines, arrows, circles, primitives | Add interaction handles |
| `ordoplay_scene` | **Scene Serialization** - RON format, hierarchy, FP-PFDS | Add prefab overrides |
| `ordoplay_blueprint` | **Prefab System** - Template instantiation | Add nested prefab support |
| `ordoplay_asset` | **Asset Loading** - Hot-reload, async pipeline, file watching | Add import settings UI |
| `ordoplay_profiler` | **Profiling** - CPU/GPU capture, Tracy, Chrome trace export | Add editor UI panels |
| `ordoplay_dev_tools` | **Debug Overlays** - FPS, frame time, screenshot | Integrate into editor |
| `ordoplay_materialx` | **Material Graphs** - 95+ node types, WGSL codegen | Build graph UI |
| `ordoplay_kernel` | **Plugin System** - Hot-reload, event bus, schemes | Use for editor plugins |
| `ordoplay_render` | **Full Renderer** - Nanite, Lumen, post-processing | Use for viewport |
| `ordoplay_input` | **Input System** - Keyboard, mouse, gamepad | Use for shortcuts |
| `ordoplay_transform` | **Transform Hierarchy** - Parent/child, matrices | Core editing target |

### 2.2 Reference Implementations to Adapt

From `reference/engines/Ambient/crates/editor/`:
- `Selection` struct with entity management (add/remove/toggle/union)
- `EditorAction<T>` pattern for throttled intent pushing with undo
- `TransformMode` enum (Translate/Rotate/Scale/Place)
- UI patterns for toolbar, selection panel, entity browser

### 2.3 External Dependencies to Integrate

| Dependency | Purpose | Version |
|------------|---------|---------|
| `egui_dock` or `egui_tiles` | Docking/layout system | Latest |
| `bevy_egui` patterns | egui integration reference | - |
| `bevy-inspector-egui` patterns | Reflect-based inspector | - |
| `space_editor` patterns | Prefab workflows, scene editing | - |

---

## Part 3: Architecture Specification

### 3.1 Workspace Structure

```
C:\Users\Administrator\Desktop\OrdoPlayEditor\
├── Cargo.toml                     # Workspace root
├── crates/
│   ├── ordoplay_editor_app/       # Main editor binary
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── app.rs             # Editor application setup
│   │   │   ├── panels/            # All UI panels
│   │   │   │   ├── mod.rs
│   │   │   │   ├── viewport.rs
│   │   │   │   ├── hierarchy.rs
│   │   │   │   ├── inspector.rs
│   │   │   │   ├── asset_browser.rs
│   │   │   │   ├── console.rs
│   │   │   │   └── profiler.rs
│   │   │   ├── tools/             # Transform tools, etc.
│   │   │   ├── menus/             # Menu bar + command palette
│   │   │   └── settings/          # Editor preferences
│   │   └── Cargo.toml
│   │
│   ├── ordoplay_editor_graph/     # Unified graph framework
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── node.rs            # Base node types
│   │   │   ├── port.rs            # Input/output ports
│   │   │   ├── connection.rs      # Edge management
│   │   │   ├── evaluation.rs      # Graph execution
│   │   │   ├── ui/                # Graph UI rendering
│   │   │   └── graphs/            # Specific graph types
│   │   │       ├── material.rs    # Material/shader graph
│   │   │       ├── gameplay.rs    # Blueprint-like visual scripting
│   │   │       ├── animation.rs   # State machines
│   │   │       └── vfx.rs         # VFX graph
│   │   └── Cargo.toml
│   │
│   ├── ordoplay_editor_sequencer/ # Timeline/sequencer
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── track.rs           # Track types
│   │   │   ├── keyframe.rs        # Keyframe management
│   │   │   ├── binding.rs         # Entity/component bindings
│   │   │   └── ui/                # Sequencer UI
│   │   └── Cargo.toml
│   │
│   └── ordoplay_editor_collab/    # Collaboration (Phase 7)
│       ├── src/
│       │   ├── lib.rs
│       │   ├── crdt.rs            # CRDT implementation
│       │   ├── presence.rs        # User presence
│       │   └── sync.rs            # State synchronization
│       └── Cargo.toml
│
└── assets/                        # Editor assets (icons, themes)
    ├── icons/
    ├── themes/
    └── layouts/                   # Saved workspace layouts

# In main OrdoPlay workspace (shared crates):
C:\Users\Administrator\Desktop\OrdoPlay\crates\
├── ordoplay_editor/               # Already exists - undo/redo core
├── ordoplay_editor_core/          # NEW: Shared editor primitives
│   ├── src/
│   │   ├── lib.rs
│   │   ├── selection.rs           # Entity selection management
│   │   ├── commands.rs            # Editor command definitions
│   │   ├── gizmo_modes.rs         # Transform mode enums
│   │   └── prefab_overrides.rs    # Override patch system
│   └── Cargo.toml
│
└── ordoplay_editor_bridge/        # NEW: Runtime ↔ Editor communication
    ├── src/
    │   ├── lib.rs
    │   ├── protocol.rs            # IPC message definitions
    │   ├── inspector.rs           # Component inspection
    │   └── hot_reload.rs          # Asset hot-reload protocol
    └── Cargo.toml
```

### 3.2 Dependency Graph

```
ordoplay_editor_app
├── ordoplay_editor_graph
├── ordoplay_editor_sequencer
├── ordoplay_editor_collab (optional)
├── ordoplay_editor_core
├── ordoplay_editor_bridge
├── egui_dock / egui_tiles
└── (OrdoPlay runtime crates via path)
    ├── ordoplay_editor (undo/redo)
    ├── ordoplay_ui
    ├── ordoplay_picking
    ├── ordoplay_gizmos
    ├── ordoplay_scene
    ├── ordoplay_blueprint
    ├── ordoplay_asset
    ├── ordoplay_render
    ├── ordoplay_profiler
    └── ordoplay_kernel
```

### 3.3 Core Data Model

#### Selection System (Adapt from Ambient)
```rust
// ordoplay_editor_core/src/selection.rs
use ordoplay_ecs::EntityId;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Selection {
    pub entities: Vec<EntityId>,
    pub mode: SelectMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectMode {
    Set,      // Replace selection
    Add,      // Union (Shift+Click)
    Remove,   // Difference (Ctrl+Click)
    Toggle,   // XOR (Ctrl+Shift+Click)
}

impl Selection {
    pub fn add(&mut self, id: EntityId) { ... }
    pub fn remove(&mut self, id: &EntityId) { ... }
    pub fn toggle(&mut self, id: EntityId) { ... }
    pub fn union(&mut self, other: &Self) { ... }
    pub fn difference(&mut self, other: &Self) { ... }
}
```

#### Editor Command System (Integrate with existing undo/redo)
```rust
// ordoplay_editor_core/src/commands.rs
use ordoplay_editor::{History, Operation, OperationGroup, StateSnapshot};

pub trait EditorCommand: Send + Sync {
    fn execute(&self, world: &mut World) -> Result<StateSnapshot>;
    fn description(&self) -> &str;
}

pub struct TransformCommand {
    pub entities: Vec<EntityId>,
    pub transforms: Vec<Transform>,
}

pub struct SpawnCommand {
    pub prefab_path: String,
    pub position: Vec3,
    pub entity_id: EntityId,
}

pub struct DeleteCommand {
    pub entities: Vec<EntityId>,
}

pub struct ComponentEditCommand {
    pub entity: EntityId,
    pub component_type: TypeId,
    pub field_path: String,
    pub old_value: Box<dyn Reflect>,
    pub new_value: Box<dyn Reflect>,
}
```

#### Prefab Override System
```rust
// ordoplay_editor_core/src/prefab_overrides.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabOverride {
    pub entity_path: EntityPath,      // Path within prefab hierarchy
    pub component_type: String,       // Type name
    pub field_path: String,           // Dot-separated field path
    pub value: DynamicValue,          // Override value
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabInstance {
    pub prefab_asset: AssetId,
    pub overrides: Vec<PrefabOverride>,
    pub added_entities: Vec<EntityId>,
    pub removed_entities: Vec<EntityPath>,
}
```

### 3.4 Panel Architecture

```rust
// ordoplay_editor_app/src/panels/mod.rs

pub trait EditorPanel {
    fn name(&self) -> &'static str;
    fn icon(&self) -> &'static str;
    fn ui(&mut self, ctx: &egui::Context, state: &mut EditorState);
    fn on_selection_changed(&mut self, selection: &Selection) {}
    fn accepts_docking(&self) -> bool { true }
}

pub struct ViewportPanel {
    pub camera: EditorCamera,
    pub gizmo_mode: GizmoMode,
    pub render_target: RenderTarget,
    pub picking_backend: PickingBackend,
}

pub struct HierarchyPanel {
    pub filter: String,
    pub show_hidden: bool,
    pub expand_state: HashMap<EntityId, bool>,
}

pub struct InspectorPanel {
    pub custom_drawers: HashMap<TypeId, Box<dyn PropertyDrawer>>,
    pub expanded_sections: HashMap<TypeId, bool>,
}

pub struct AssetBrowserPanel {
    pub current_path: PathBuf,
    pub view_mode: AssetViewMode,  // Grid, List
    pub filter: AssetFilter,
    pub selected_assets: Vec<AssetId>,
}

pub struct ConsolePanel {
    pub log_filter: LogLevel,
    pub search: String,
    pub auto_scroll: bool,
}

pub struct ProfilerPanel {
    pub capture: Option<ProfileCapture>,
    pub selected_frame: Option<usize>,
    pub view_mode: ProfilerViewMode,  // Flame, Timeline, Stats
}
```

---

## Part 4: Implementation Phases

### Phase 1: Foundations (MVP Editor Shell)
**Goal**: Basic editor with viewport, hierarchy, and property editing

| Task | Source | Effort |
|------|--------|--------|
| Create standalone workspace structure | New | 1 day |
| Set up egui_dock/egui_tiles integration | New + egui_dock | 2 days |
| Create viewport panel with ordoplay_render | ordoplay_render | 3 days |
| Implement editor camera (orbit, pan, zoom) | New | 2 days |
| Create hierarchy panel | ordoplay_scene | 2 days |
| Implement basic inspector with Reflect | ordoplay_core/reflect | 4 days |
| Wire selection system | Ambient reference | 2 days |
| Implement transform gizmos | ordoplay_gizmos | 3 days |
| Connect undo/redo | ordoplay_editor | 2 days |
| Scene save/load | ordoplay_scene | 1 day |
| **Total Phase 1** | | **~22 days** |

### Phase 2: Asset Pipeline Integration
**Goal**: Full asset workflow with import, browse, hot-reload

| Task | Source | Effort |
|------|--------|--------|
| Asset browser panel | ordoplay_asset | 3 days |
| Thumbnail generation system | New | 3 days |
| Import settings UI per asset type | New | 4 days |
| Hot-reload integration | ordoplay_asset | 2 days |
| Drag-drop from browser to viewport | New | 2 days |
| Asset dependency visualization | ordoplay_asset | 2 days |
| **Total Phase 2** | | **~16 days** |

### Phase 3: Prefab System
**Goal**: Unity/O3DE-grade prefab workflows

| Task | Source | Effort |
|------|--------|--------|
| Nested prefab support | ordoplay_blueprint | 4 days |
| Override patch system | New (based on O3DE) | 5 days |
| Prefab edit mode | New | 3 days |
| Prefab variants | New | 2 days |
| Apply/Revert UI | New | 2 days |
| **Total Phase 3** | | **~16 days** |

### Phase 4: Graph Framework
**Goal**: Unified graph system for all authoring domains

| Task | Source | Effort |
|------|--------|--------|
| Base graph framework (nodes, ports, edges) | New | 5 days |
| Graph UI renderer | New | 4 days |
| Material graph integration | ordoplay_materialx | 3 days |
| Gameplay graph (Blueprint-like) | New | 5 days |
| Animation state machine graph | ordoplay_animation | 3 days |
| VFX graph | New | 3 days |
| **Total Phase 4** | | **~23 days** |

### Phase 5: Sequencer/Timeline
**Goal**: Unreal Sequencer-class cinematic/gameplay authoring

| Task | Source | Effort |
|------|--------|--------|
| Track system architecture | New | 4 days |
| Transform tracks | New | 2 days |
| Event tracks | New | 2 days |
| Audio tracks | ordoplay_audio | 2 days |
| Camera tracks | ordoplay_camera | 2 days |
| Keyframe editing UI | New | 3 days |
| Curve editor | New | 3 days |
| Entity binding system | New | 2 days |
| **Total Phase 5** | | **~20 days** |

### Phase 6: World Building at Scale
**Goal**: UE5-class world partition and streaming

| Task | Source | Effort |
|------|--------|--------|
| World partition grid system | New (based on UE5) | 5 days |
| Cell streaming in editor | New | 4 days |
| Data layer system | New (based on UE5) | 4 days |
| HLOD generation pipeline | New | 5 days |
| Terrain integration | ordoplay_terra | 3 days |
| Large-world performance optimization | New | 3 days |
| **Total Phase 6** | | **~24 days** |

### Phase 7: Collaboration (Future-Proof)
**Goal**: Real-time multi-user editing

| Task | Source | Effort |
|------|--------|--------|
| CRDT implementation for scene edits | New | 7 days |
| Networking layer (WebSocket/QUIC) | ordoplay_net | 3 days |
| Presence system | New | 2 days |
| Conflict resolution UI | New | 3 days |
| Comments/annotations | New | 3 days |
| **Total Phase 7** | | **~18 days** |

### Phase 8: Polish & Observability
**Goal**: Production-ready tooling

| Task | Source | Effort |
|------|--------|--------|
| Command palette (Ctrl+P) | New | 2 days |
| Keyboard shortcut system | ordoplay_input | 2 days |
| Layout save/restore | egui_dock | 1 day |
| Theme system (dark/light) | New | 2 days |
| Profiler panel integration | ordoplay_profiler | 3 days |
| Console panel | New | 2 days |
| Crash reporting | New | 2 days |
| **Total Phase 8** | | **~14 days** |

---

## Part 5: Prioritized Task List

### Immediate (Week 1-2): MVP Vertical Slice
1. [ ] Create `C:\Users\Administrator\Desktop\OrdoPlayEditor\` workspace
2. [ ] Set up Cargo.toml with path dependencies to OrdoPlay
3. [ ] Create `ordoplay_editor_app` binary crate
4. [ ] Integrate egui_dock for docking layout
5. [ ] Create empty panel stubs (Viewport, Hierarchy, Inspector, AssetBrowser)
6. [ ] Render ordoplay_render output to egui texture
7. [ ] Implement editor camera controls
8. [ ] Wire selection to ordoplay_picking

### Short-Term (Week 3-4): Core Editing
9. [ ] Create `ordoplay_editor_core` in main workspace
10. [ ] Port Selection struct from Ambient reference
11. [ ] Build hierarchy panel with ordoplay_scene
12. [ ] Build Reflect-based inspector
13. [ ] Wire transform gizmos from ordoplay_gizmos
14. [ ] Connect ordoplay_editor undo/redo to commands
15. [ ] Scene save/load with RON

### Medium-Term (Month 2): Production Features
16. [ ] Asset browser with ordoplay_asset
17. [ ] Prefab system with overrides
18. [ ] Material graph editor
19. [ ] Command palette
20. [ ] Profiler panel

### Long-Term (Month 3+): Advanced Features
21. [ ] Gameplay graph (visual scripting)
22. [ ] Sequencer/timeline
23. [ ] World partition
24. [ ] Collaboration

---

## Part 6: Technical Decisions

### 6.1 UI Framework Choice

**Decision**: Use `ordoplay_ui` for viewport/game rendering, `egui` + `egui_dock` for editor panels.

**Rationale**:
- `ordoplay_ui` is ECS-driven, optimized for game UI, not editor workflows
- `egui` is immediate-mode, perfect for rapid editor iteration
- `egui_dock` provides mature docking with drag/drop tab support
- This hybrid approach matches Bevy ecosystem patterns (bevy_egui + bevy-inspector-egui)

### 6.2 Graph Framework Design

**Decision**: Build single generic graph framework, specialize via node registry.

**Rationale**:
- Unreal Blueprints, Unity's graphs, and node editors all share common patterns
- Single implementation reduces maintenance, ensures consistent UX
- Node registry allows domain-specific graphs (material, gameplay, animation, VFX)
- Leverage ordoplay_graph crate for underlying data structures

### 6.3 Undo/Redo Strategy

**Decision**: Keep existing CoW-based ordoplay_editor::History, extend with ECS command integration.

**Rationale**:
- Existing implementation is well-designed (RedoxFS patterns)
- OperationGroup already supports transaction batching
- Add command types for common editor operations
- Preserve per-operation memory tracking

### 6.4 Prefab Override System

**Decision**: Field-path based override patches (Unity/O3DE pattern).

**Rationale**:
- Overrides stored as delta from prefab template
- Field paths allow fine-grained override tracking
- Nested prefabs work via recursive resolution
- Compatible with reflection-based property editing

### 6.5 Collaboration Architecture

**Decision**: CRDT-based with operation-centric sync.

**Rationale**:
- Commands are already operations (command pattern)
- CRDTs guarantee eventual consistency without complex locking
- Operation-based sync minimizes bandwidth
- Matches UEFN/Roblox collaboration models

---

## Part 7: Quality Standards (SOTA Checklist)

The editor is "SOTA" when it achieves:

- [ ] **Search-first UX**: Everything searchable (assets, commands, entities, components)
- [ ] **Instant feedback**: No UI stalls, async operations, hot-reload
- [ ] **Consistent interaction**: Same selection/editing semantics everywhere
- [ ] **Safe editing**: Full undo/redo, non-destructive, validation
- [ ] **Extensibility**: Plugins can add panels, importers, inspectors, graph nodes
- [ ] **Scalability**: World partition + layers for large projects
- [ ] **Observability**: Integrated profiling and diagnostics
- [ ] **Collaboration-ready**: Architecture supports multi-user (even if deferred)

---

## Part 8: Sources & References

### Game Engine Editor Research
- [Game Engine Architecture (Gameenginebook.com)](https://www.gameenginebook.com/)
- [O3DE Documentation](https://docs.o3de.org/)
- [O3DE Key Concepts](https://docs.o3de.org/docs/welcome-guide/key-concepts/)
- [Unreal Engine World Partition](https://dev.epicgames.com/documentation/en-us/unreal-engine/world-partition-in-unreal-engine)
- [Unreal Engine Data Layers](https://dev.epicgames.com/documentation/en-us/unreal-engine/world-partition---data-layers-in-unreal-engine)
- [Unity 6 Roadmap](https://unity.com/roadmap/detail)
- [Unity Graph Toolkit](https://discussions.unity.com/t/unity-graph-toolkit-update-q2-2025/1656349)
- [Godot Inspector System](https://deepwiki.com/godotengine/godot/10.1-inspector-system)
- [Godot Scene/Node Architecture](https://foro3d.com/en/2026/january/project-organization-in-godot-scene-and-node-system.html)

### Rust/Bevy Editor Development
- [bevy-inspector-egui](https://github.com/jakobhellermann/bevy-inspector-egui)
- [space_editor (Bevy Prefab Editor)](https://github.com/rewin123/space_editor)
- [Bevy Editor Prototypes Discussion](https://github.com/bevyengine/bevy_editor_prototypes/discussions/1)
- [Bevy Editor Design Questions](https://hackmd.io/@bevy/editor_questions)
- [egui_dock](https://github.com/Adanos020/egui_dock)
- [egui_tiles](https://crates.io/crates/egui_tiles)
- [Bevy Dev Tools](https://bevy-cheatbook.github.io/setup/bevy-tools.html)

### Undo/Redo & Command Pattern
- [Command Pattern - Game Programming Patterns](http://gameprogrammingpatterns.com/command.html)
- [Undo/Redo in Game Programming](https://medium.com/@xainulabideen600/undo-redo-in-game-programming-using-command-pattern-d49eba152ca3)
- [The Unseen Hand: Undo/Redo in Level Design](https://www.wayline.io/blog/undo-redo-level-design)
- [Command Pattern in Unity](https://unity.com/how-to/use-command-pattern-flexible-and-extensible-game-systems)
- [How We Implement Undo (Wolfire Games)](http://blog.wolfire.com/2009/02/how-we-implement-undo/)

### Visual Scripting & Node Graphs
- [Node Graph Architecture (Wikipedia)](https://en.wikipedia.org/wiki/Node_graph_architecture)
- [Designing Visual Programming Languages](https://dev.to/cosmomyzrailgorynych/designing-your-own-node-based-visual-programming-language-2mpg)
- [Rete.js](https://retejs.org/)
- [LiteGraph.js](https://github.com/jagenjo/litegraph.js)

### Collaboration & CRDTs
- [Real-Time Collaboration: OT vs CRDT](https://www.tiny.cloud/blog/real-time-collaboration-ot-vs-crdt/)
- [CRDTs Unleashed (Rust)](https://medium.com/@FAANG/crdts-unleashed-building-a-real-time-collaborative-editing-engine-with-rust-ac6b6cdcfdb3)
- [CRDT-Based Game State Sync in VR](https://arxiv.org/html/2503.17826)
- [Introduction to CRDTs](https://dev.to/nyxtom/introduction-to-crdts-for-realtime-collaboration-2eb1)

---

## Appendix A: File-by-File Leverage Plan

### From ordoplay_editor (Copy Verbatim)
- `history.rs` → Use directly for undo/redo
- `error.rs` → Use for error types

### From ordoplay_ui (Use As-Is)
- Layout system (Taffy integration) → For complex panel layouts
- Widget system → For in-viewport UI overlays
- Picking backend → For UI element selection

### From ordoplay_picking (Use As-Is)
- Pointer events → For selection
- Hit testing → For gizmo interaction
- Event bubbling → For hierarchical selection

### From ordoplay_gizmos (Extend)
- Primitives → Base for transform gizmos
- Styling → Consistent visual language

### From ordoplay_scene (Use + Extend)
- Serialization → RON scene format
- Hierarchy → Entity parent/child
- FP-PFDS → O(1) scene snapshots

### From ordoplay_blueprint (Use + Extend)
- Template system → Prefab base
- Instantiation → Prefab spawning

### From ordoplay_asset (Use As-Is)
- Hot-reload → Asset updates
- Async loading → Background import
- File watching → Change detection

### From ordoplay_profiler (Integrate)
- CPU profiling → Performance panels
- GPU profiling → Render analysis
- Tracy → External profiler support

### From ordoplay_materialx (Build UI On Top)
- Node types → Material graph nodes
- WGSL codegen → Shader compilation
- Graph evaluation → Runtime preview

### From reference/engines/Ambient/crates/editor (Adapt Patterns)
- `Selection` struct → Entity selection
- `EditorAction<T>` → Command pattern
- `TransformMode` → Gizmo modes
- UI patterns → Toolbar, panels

---

*Document Version: 1.0*
*Created: January 17, 2026*
*Author: Claude Code Assistant*
*For: OrdoPlay Engine Editor Development*
