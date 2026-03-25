# Codebase Structure

**Analysis Date:** 2026-01-28

## Directory Layout

```
muda/
├── Cargo.toml                  # Workspace manifest defining three crates
├── Cargo.lock                  # Dependency lock file
├── core_editor/                # Domain logic crate (platform-agnostic)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs              # Crate root, public module exports
│   │   ├── app.rs              # App struct gluing workspace + dispatcher
│   │   ├── domain/             # Core domain types
│   │   │   ├── mod.rs
│   │   │   ├── text_buffer.rs  # TextBuffer, TextPosition, TextRange, TextOffset
│   │   │   ├── document.rs     # Document with identity, persistence
│   │   │   ├── workspace.rs    # Workspace managing multiple documents
│   │   │   └── protection.rs   # ProtectionError, DiscardAcknowledgment
│   │   ├── view/               # View-specific state (not document)
│   │   │   ├── mod.rs
│   │   │   ├── editor_view.rs  # EditorView, ViewId, ViewOptions
│   │   │   ├── caret.rs        # Caret, CaretSet
│   │   │   ├── selection.rs    # Selection, SelectionSet
│   │   │   ├── viewport.rs     # Viewport (scroll offset, dimensions)
│   │   │   └── sidebar.rs      # Sidebar, FileEntry, FocusState
│   │   ├── commands/           # Command dispatch system
│   │   │   ├── mod.rs
│   │   │   ├── dispatcher.rs   # CommandDispatcher routes commands to handlers
│   │   │   ├── editor_command.rs # EditorCommand enum (MoveCursor, InsertChar, etc.)
│   │   │   ├── edit_operation.rs # EditOperation (Delete, Insert, Replace - reversible)
│   │   │   ├── history.rs      # CommandHistory undo/redo stack
│   │   │   └── transaction.rs  # UndoTransaction groups operations
│   │   ├── view_model/         # Render-ready projections
│   │   │   ├── mod.rs          # RenderModel, TextStyle enum (semantic tokens)
│   │   │   └── builder.rs      # ViewModelBuilder constructs RenderModel from state
│   │   ├── syntax.rs           # SyntaxHighlighter with tree-sitter integration
│   │   ├── events.rs           # DomainEvent and EventBus
│   │   └── benches/            # Performance benchmarks
├── wgpu_client/                # GPU-accelerated desktop client crate
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs             # Binary entry point, CLI parsing, event loop start
│   │   ├── app.rs              # WgpuApp (implements winit ApplicationHandler)
│   │   ├── input.rs            # Translates winit events to EditorCommand
│   │   ├── theme/              # Color and styling definitions
│   │   │   ├── mod.rs
│   │   │   ├── colors.rs       # Color struct and predefined colors
│   │   │   └── [theme modes]
│   │   ├── design_system/      # Design tokens and rendering primitives
│   │   │   ├── mod.rs
│   │   │   ├── tokens/         # Design tokens
│   │   │   │   ├── mod.rs
│   │   │   │   ├── color.rs    # ColorPalette, ColorRole
│   │   │   │   ├── spacing.rs  # Space, Spacing (margin, padding)
│   │   │   │   ├── sizing.rs   # ComponentSize enum and helpers
│   │   │   │   ├── radius.rs   # Radius, CornerRadii for border-radius
│   │   │   │   ├── elevation.rs # Shadow, Elevation for depth
│   │   │   │   ├── motion.rs   # Duration, Easing for animations
│   │   │   │   └── theme.rs    # Theme struct (light/dark mode settings)
│   │   │   ├── primitives/     # GPU-rendered primitives
│   │   │   │   ├── mod.rs
│   │   │   │   ├── styled_rect.rs # StyledRect (rect with border, shadow, radius)
│   │   │   │   ├── styled_rect_renderer.rs # StyledRectRenderer GPU pipeline
│   │   │   │   └── text_block.rs # TextBlock (text + styling)
│   │   │   ├── interaction/    # Interaction state (hover, focus, active)
│   │   │   │   ├── mod.rs
│   │   │   │   ├── state.rs    # InteractionState enum
│   │   │   │   └── focus.rs    # Focus tracking
│   │   │   ├── layout/         # Flexbox-like layout system
│   │   │   │   ├── mod.rs
│   │   │   │   └── flex.rs     # FlexLayout, FlexDirection, AlignItems, JustifyContent
│   │   │   └── animation/      # Animation and tweening
│   │   │       ├── mod.rs
│   │   │       ├── tween.rs    # Tween, TweenState
│   │   │       └── easing.rs   # Easing functions
│   │   ├── renderer/           # GPU rendering orchestrator
│   │   │   ├── mod.rs          # GpuRenderer (wgpu Surface, Device, Queue)
│   │   │   └── debug.rs        # Debug overlay visualization (debug builds only)
│   │   ├── components/         # UI components
│   │   │   ├── mod.rs
│   │   │   ├── text_area.rs    # TextArea (main editor content with syntax highlighting)
│   │   │   ├── gutter.rs       # Gutter (line numbers)
│   │   │   ├── caret.rs        # Caret renderer
│   │   │   ├── tab_bar.rs      # Tab bar (open documents)
│   │   │   ├── sidebar.rs      # Sidebar panel
│   │   │   ├── status_bar.rs   # Status bar (mode, line/col info)
│   │   │   ├── dialog.rs       # Dialog overlays (confirmations, etc.)
│   │   │   ├── rect.rs         # Rect and RectRenderer (basic GPU rectangles)
│   │   │   ├── ui_text_renderer.rs # UITextRenderer (theme-aware text rendering)
│   │   │   └── shaders/        # WGSL shader files (if any)
│   │   ├── ui/                 # High-level UI layout and components
│   │   │   ├── mod.rs
│   │   │   ├── app_layout.rs   # AppLayout (toolbar, sidebars, editor, panels, status)
│   │   │   ├── command_palette.rs # CommandPalette (Ctrl-P search)
│   │   │   ├── editor_tabs.rs  # EditorTabs (document tabs at top)
│   │   │   ├── file_tree.rs    # FileTree (left sidebar file browser)
│   │   │   ├── panels.rs       # PanelManager (right/bottom panels)
│   │   │   ├── status_line.rs  # StatusLine (bottom status bar)
│   │   │   ├── toolbar.rs      # Toolbar (top menu bar)
│   │   │   └── split_view.rs   # Split view management (multiple panes)
│   │   └── widgets/            # Complex composite widgets (if any)
│   │       ├── mod.rs
│   │       ├── core/
│   │       ├── composites/
│   │       └── navigation/
├── ratatui_client/             # TUI client using ratatui
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs             # Binary entry point
│   │   └── draw/               # Terminal drawing logic
│   │       └── ui.rs
├── .planning/                  # GSD planning directory
│   ├── codebase/               # Codebase analysis documents
│   │   ├── ARCHITECTURE.md
│   │   ├── STRUCTURE.md
│   │   ├── STACK.md
│   │   ├── CONVENTIONS.md
│   │   ├── TESTING.md
│   │   ├── INTEGRATIONS.md
│   │   └── CONCERNS.md
│   ├── phases/                 # Phase execution plans
│   └── config.json
├── scripts/                    # Build and dev scripts
├── BENCHMARKS.md               # Performance analysis
├── MUDAEDITOR.md               # Project overview
├── README.md                   # User-facing documentation
└── editor.log                  # Runtime log file (generated)
```

## Directory Purposes

**core_editor/**
- Purpose: Platform-agnostic editor domain logic and state management
- Contains: Domain types (Document, TextBuffer, Workspace), view state (EditorView, Caret, Selection), command dispatch system, syntax highlighting, undo/redo
- Key files: `lib.rs` (public exports), `app.rs` (App state), `domain/*.rs` (data model), `commands/*.rs` (command execution), `view_model/*.rs` (render projection)

**wgpu_client/src/**
- Purpose: GPU-accelerated native desktop client implementation
- Contains: winit event loop integration, input translation, wgpu rendering pipeline, UI components, design system
- Key files: `main.rs` (CLI entry), `app.rs` (event handler), `renderer/mod.rs` (GPU orchestrator), `input.rs` (event translation)

**wgpu_client/src/design_system/**
- Purpose: Unified design token system and rendering primitives for IDE UI
- Contains: Colors, spacing, sizing, elevation, animations, interaction states, flexbox-like layout, GPU primitives (styled rects, text)
- Key files: `tokens/*.rs` (design tokens), `primitives/*.rs` (GPU rendering), `interaction/*.rs` (state tracking), `layout/*.rs` (flexbox)

**wgpu_client/src/components/**
- Purpose: Reusable UI components for editor chrome
- Contains: TextArea (main editor), Gutter (line numbers), Caret renderer, TabBar, Sidebar, StatusBar, Dialog
- Key files: Organized by responsibility (text rendering, navigation, feedback)

**wgpu_client/src/ui/**
- Purpose: High-level IDE layout management and complex composite UI
- Contains: AppLayout (main layout grid), CommandPalette, FileTree, EditorTabs, PanelManager, StatusLine, split view management
- Key files: `app_layout.rs` (main grid layout), `command_palette.rs` (Ctrl-P), `file_tree.rs` (file browser)

**ratatui_client/**
- Purpose: Alternative TUI (terminal UI) client using ratatui for headless/SSH workflows
- Contains: Terminal event handling, TUI drawing logic
- Key files: `main.rs` (entry), `draw/ui.rs` (drawing primitives)

**scripts/**
- Purpose: Development and build automation
- Contains: Build scripts, test runners, benchmarking tools

## Key File Locations

**Entry Points:**
- `wgpu_client/src/main.rs`: GPU client binary entry point; initializes logging, parses CLI args, creates WgpuApp, runs event loop
- `ratatui_client/src/main.rs`: TUI client binary entry point; similar structure for terminal client
- `core_editor/src/lib.rs`: Library root; re-exports public API for all clients

**Configuration:**
- `Cargo.toml`: Workspace manifest (members, shared package config)
- `Cargo.lock`: Dependency lock file
- Not applicable: No external config files (.env, .toml configs) at project root

**Core Logic:**
- `core_editor/src/domain/`: Text models, document identity, persistence
- `core_editor/src/commands/`: Command dispatch system, undo/redo, edit operations
- `core_editor/src/view/`: Cursor, selection, viewport, focus state
- `core_editor/src/view_model/`: RenderModel projection, styling tokens
- `core_editor/src/syntax.rs`: Syntax highlighting via tree-sitter

**Testing:**
- Not found: No dedicated `tests/` directories
- Test organization: Tests likely in unit test modules within respective source files (mod tests { ... })

**Rendering & UI:**
- `wgpu_client/src/renderer/mod.rs`: GPU rendering orchestrator, wgpu pipeline management
- `wgpu_client/src/design_system/`: Design tokens, primitives, interaction tracking, layout, animation
- `wgpu_client/src/components/`: Component implementations
- `wgpu_client/src/ui/`: High-level layout and UI logic

## Naming Conventions

**Files:**
- Snake_case module files: `text_buffer.rs`, `editor_view.rs`, `command_palette.rs`
- Each public module has `mod.rs` with public exports and documentation
- Suffix patterns: `_renderer.rs` (GPU rendering), `_palette.rs` (complex UI), `_bar.rs` (strip UI)

**Directories:**
- Snake_case folders: `design_system`, `editor_view`, `ui_text_renderer`
- Domain: singular nouns (`domain/`) containing multiple types (Document, TextBuffer, Workspace)
- Feature areas: `components/`, `ui/`, `design_system/`, `widgets/` for organizational categories

**Modules:**
- Descriptive names reflecting responsibility: `dispatcher`, `text_area`, `command_palette`, `styled_rect_renderer`
- Crate structure: Three top-level crates (core_editor, wgpu_client, ratatui_client) with clear ownership boundaries

## Where to Add New Code

**New Feature (e.g., line wrapping):**
- Primary implementation: `core_editor/src/view/editor_view.rs` (add ViewOption flag) or `core_editor/src/view/viewport.rs` (modify scroll logic)
- Update: `core_editor/src/commands/dispatcher.rs` if new EditorCommand is needed
- View model: `core_editor/src/view_model/builder.rs` if render projection changes
- Rendering: `wgpu_client/src/components/text_area.rs` if GPU rendering needs adjustment
- Tests: Unit tests in same module as implementation

**New UI Component (e.g., autocomplete popup):**
- Create new file in `wgpu_client/src/components/` (e.g., `autocomplete.rs`)
- Export in `wgpu_client/src/components/mod.rs`
- Add rendering logic using design system tokens from `wgpu_client/src/design_system/`
- Register in `wgpu_client/src/renderer/mod.rs` to include in main render pass
- Hit testing: Add LayoutRegion variant in `wgpu_client/src/ui/app_layout.rs` if it affects layout

**New Design Token (e.g., new color):**
- Add to appropriate enum/struct in `wgpu_client/src/design_system/tokens/` (e.g., `color.rs` for colors, `spacing.rs` for dimensions)
- Update `wgpu_client/src/design_system/mod.rs` re-exports if public
- Update `wgpu_client/src/theme/colors.rs` with concrete values for light/dark modes
- Update components that use it in `wgpu_client/src/components/`

**New Command Handler:**
- Define variant in `core_editor/src/commands/editor_command.rs`
- Add handler logic in `core_editor/src/commands/dispatcher.rs` (execute method)
- Create EditOperation if text changes in `core_editor/src/commands/edit_operation.rs`
- Translate events in `wgpu_client/src/input.rs` to dispatch the command

**Syntax Language Support:**
- Add language enum variant to `SyntaxLanguage` in `core_editor/src/syntax.rs`
- Add file extension match in `from_extension()` method
- Add tree-sitter language binding and query string (HIGHLIGHTS constant)
- Library dependency: Add tree_sitter_[language] crate to Cargo.toml

**Utilities & Helpers:**
- General editor utilities: `core_editor/src/` root or dedicated module
- GPU/rendering utilities: `wgpu_client/src/renderer/` or create `wgpu_client/src/utils.rs`
- Math/geometry helpers: `wgpu_client/src/design_system/` or component-specific modules

## Special Directories

**target/**
- Purpose: Rust compilation output (generated)
- Contains: Debug and release binaries, dependencies, intermediate build artifacts
- Generated: Yes
- Committed: No (ignored in .gitignore)

**.idea/**
- Purpose: IntelliJ/RustRover IDE settings
- Contains: Project configuration, run profiles, inspections
- Generated: Yes
- Committed: No (should be in .gitignore)

**.planning/**
- Purpose: GSD (Guided Software Design) planning documents
- Contains: Phase plans, codebase analysis, research documents
- Generated: Partially (created by GSD commands)
- Committed: Yes (version controlled)

**.claude/**
- Purpose: Claude Code extension metadata and preferences
- Contains: Extension configuration, session state
- Generated: Yes
- Committed: No

**.git/**
- Purpose: Git repository metadata
- Contains: Commit history, branch information, object database
- Generated: Yes
- Committed: No

---

*Structure analysis: 2026-01-28*
