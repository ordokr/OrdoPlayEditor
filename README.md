# OrdoPlay Editor

State-of-the-art game engine editor for OrdoPlay, combining the best features from Unreal Engine, Unity, Godot, O3DE, and CryEngine.

## Architecture

```
OrdoPlayEditor/
├── crates/
│   ├── ordoplay_editor_app/      # Main editor binary
│   │   └── src/
│   │       ├── panels/           # UI panels (viewport, hierarchy, inspector, etc.)
│   │       ├── tools/            # Transform gizmos, camera controls
│   │       ├── menus/            # Menu system, command palette
│   │       ├── commands.rs       # Editor commands with undo/redo
│   │       └── state.rs          # Editor state management
│   │
│   ├── ordoplay_editor_graph/    # Node graph framework
│   │   └── src/
│   │       ├── graphs/           # Material, gameplay, animation, VFX graphs
│   │       └── ui/               # Graph UI rendering
│   │
│   └── ordoplay_editor_sequencer/ # Timeline/sequencer
│       └── src/
│           ├── track.rs          # Track types
│           ├── keyframe.rs       # Keyframe management
│           └── sequence.rs       # Sequence container
│
└── assets/                       # Editor assets (icons, themes, layouts)
```

## Features

### Core Editor
- **Docking UI** - Customizable panel layout with egui_dock
- **Viewport** - Full 3D rendering with OrdoPlay renderer (Nanite, Lumen)
- **Hierarchy** - Entity tree with drag-drop reparenting
- **Inspector** - Reflection-based property editing
- **Asset Browser** - Grid/list view with thumbnails and hot-reload
- **Console** - Log output with filtering and command input
- **Profiler** - Frame timing, CPU/GPU analysis

### Graph Framework
- **Material Graph** - Visual shader authoring (integrates with ordoplay_materialx)
- **Gameplay Graph** - Blueprint-style visual scripting
- **Animation Graph** - State machines and blend trees
- **VFX Graph** - Particle and effect authoring

### Sequencer
- **Multi-track Timeline** - Transform, property, event, audio, camera tracks
- **Keyframe Editing** - Multiple interpolation modes
- **Curve Editor** - Bezier curve editing
- **Entity Binding** - Animate any entity/component

### Undo/Redo
- **CoW-based History** - Copy-on-Write snapshots (from ordoplay_editor)
- **Transaction Grouping** - Batch operations
- **Memory Management** - Configurable history depth

## Building

```bash
cd C:\Users\Administrator\Desktop\OrdoPlayEditor
cargo build
```

## Running

```bash
cargo run --bin ordoplay_editor
```

## Dependencies

The editor uses path dependencies to the main OrdoPlay workspace:

- `ordoplay_core` - Math, reflection, utilities
- `ordoplay_ecs` - Entity Component System
- `ordoplay_editor` - Undo/redo system
- `ordoplay_render` - Full rendering pipeline
- `ordoplay_ui` - UI framework
- `ordoplay_picking` - Hit testing and selection
- `ordoplay_gizmos` - Debug drawing
- `ordoplay_scene` - Scene serialization
- `ordoplay_asset` - Asset loading and hot-reload
- `ordoplay_materialx` - Material graph nodes

## Documentation

- [Design Document](./ORDOPLAY_EDITOR_DESIGN_DOCUMENT.md) - Full architecture specification
- [Task List](./ORDOPLAY_EDITOR_TASK_LIST.md) - Implementation tasks with priorities

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| W | Translate mode |
| E | Rotate mode |
| R | Scale mode |
| F | Focus on selection |
| Ctrl+Z | Undo |
| Ctrl+Y / Ctrl+Shift+Z | Redo |
| Ctrl+S | Save scene |
| Ctrl+D | Duplicate |
| Delete | Delete selected |
| Alt+LMB | Orbit camera |
| MMB | Pan camera |
| Scroll | Zoom camera |

## License

MIT OR Apache-2.0
