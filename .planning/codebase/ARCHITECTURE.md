# Architecture

**Analysis Date:** 2026-01-28

## Pattern Overview

**Overall:** Multi-layered, backend-agnostic architecture with clean separation between domain logic, view state, and rendering.

**Key Characteristics:**
- **Domain-driven design** - Core editor logic isolated in `core_editor` crate, platform-agnostic
- **Backend abstraction** - Rendering implementations (wgpu_client, ratatui_client) are separate crates that depend on core_editor
- **Command pattern** - User actions mapped to high-level `EditorCommand` before reaching domain layer
- **View-Document separation** - EditorView (cursor, selection, viewport) is separate from Document (text buffer, metadata), enabling multiple views of same document
- **ViewModel projection** - RenderModel layer decouples rendering from domain internals

## Layers

**Domain Layer:**
- Purpose: Represents editor state without UI concerns (text buffers, documents, commands, undo/redo)
- Location: `core_editor/src/domain/` and `core_editor/src/commands/`
- Contains: `Document`, `TextBuffer`, `Workspace`, `CommandDispatcher`, `CommandHistory`
- Depends on: Only standard library and minimal external crates
- Used by: View layer, ViewModel layer, all rendering backends

**View Layer:**
- Purpose: Manages UI-specific state (cursor position, selection, scroll viewport, focus state)
- Location: `core_editor/src/view/`
- Contains: `EditorView`, `Caret`, `Selection`, `Viewport`, `Sidebar`, `FocusState`
- Depends on: Domain layer (Document, TextBuffer types)
- Used by: ViewModel layer, application state

**ViewModel Layer:**
- Purpose: Projects domain and view state into render-ready format without prescribing how rendering works
- Location: `core_editor/src/view_model/`
- Contains: `RenderModel`, `TextStyle` (semantic style tokens, not visual), `StyledSpan`
- Depends on: Domain and View layers
- Used by: Rendering backends

**Application Shell:**
- Purpose: Glues together Workspace, CommandDispatcher, and UI-only state (dialogs, status messages)
- Location: `core_editor/src/app.rs`
- Contains: `App` struct holding workspace, dispatcher, dialogs, focus state
- Depends on: All core_editor modules
- Used by: Each rendering backend via their own app wrapper

**Rendering Backend Layer (wgpu_client example):**
- Purpose: Converts high-level events into editor commands; renders ViewModel to GPU/screen
- Location: `wgpu_client/src/` - separate crate
- Contains: `WgpuApp` (wraps core `App`), `GpuRenderer`, input translation, component hierarchy
- Depends on: core_editor, wgpu, winit, glyphon (text rendering)
- Pattern: Backend receives winit events → translates to EditorCommand → dispatches to core App → re-renders using RenderModel

## Data Flow

**Rendering Flow:**

1. Backend receives event (keyboard, mouse, window resize)
2. Input handler translates OS event to `EditorCommand` (e.g., KeyEvent → MoveCursor with Direction/Scope)
3. `CommandDispatcher::execute()` processes command against current Document and EditorView
4. Domain state updates (TextBuffer changes, Caret moves, Selection adjusts)
5. `CommandHistory` records reversible `EditOperation` for undo/redo
6. `ViewModelBuilder` creates `RenderModel` from updated Document + EditorView
7. `GpuRenderer` receives RenderModel and renders all components
8. Components (TextArea, Gutter, TabBar, etc.) reference ViewModel for styling tokens

**Example - Text insertion at cursor:**
```
User types 'a'
  → winit KeyboardInput event
  → translate_key() → EditorCommand::InsertChar('a')
  → dispatcher.execute(cmd, context)
  → document.insert_at(caret_pos, 'a')
  → history.push(EditOperation::Insert {pos, char})
  → caret advances by 1
  → render_model = ViewModelBuilder::build(document, view)
  → gpu_renderer.render(render_model)
```

**State Management:**

- **Immutable facts:** Document text, document identity (DocumentId), view identity (ViewId)
- **Mutable state:** Cursor position, selection range, viewport scroll, undo/redo stack, open tabs
- **Derived state:** Syntax highlighting (computed from Document + SyntaxHighlighter), layout bounds (computed from window size + UI configuration)
- **Transient state:** Pending dialogs (unsaved changes), status messages, cursor icon, drag-to-select state

## Key Abstractions

**Document and TextBuffer:**
- Purpose: Document wraps TextBuffer (raw text storage) and adds identity, persistence, language detection, derived state
- Examples: `core_editor/src/domain/document.rs`, `core_editor/src/domain/text_buffer.rs`
- Pattern: TextBuffer provides coordinate systems (TextPosition, TextOffset, TextRange), Document adds semantics (file path, line endings, revision tracking)

**EditorCommand:**
- Purpose: High-level user intent independent of platform (MoveCursor, InsertChar, Delete, etc.)
- Examples: `core_editor/src/commands/editor_command.rs` defines `EditorCommand` enum
- Pattern: Translators convert platform-specific events (winit KeyEvent, mouse position) to EditorCommand before reaching dispatcher

**RenderModel:**
- Purpose: Compact projection of editor state ready for any rendering backend
- Examples: `core_editor/src/view_model/mod.rs`
- Pattern: Contains styled line segments for each line, cursor bounds, selection ranges, but no rendering directives

**TextStyle Tokens:**
- Purpose: Semantic token types (Keyword, String, Comment, Selection, etc.) that map to visual styles downstream
- Examples: `core_editor/src/view_model/mod.rs` enum TextStyle
- Pattern: Domain emits TextStyle::Keyword; rendering layer (e.g., draw.rs in ratatui_client) maps to concrete colors/modifiers

**Component Hierarchy (GPU client):**
- Purpose: Reusable UI components for rendering specific parts of IDE
- Examples: `wgpu_client/src/components/text_area.rs`, `wgpu_client/src/components/gutter.rs`
- Pattern: Each component receives Bounds and RenderModel, produces GPU commands; TextArea renders text + selection, Gutter renders line numbers

**Design System:**
- Purpose: Unified token system (colors, spacing, elevation, animation) for consistent IDE styling
- Examples: `wgpu_client/src/design_system/tokens/` - Color, Spacing, Radius, Elevation, Theme
- Pattern: Components reference tokens (e.g., `theme::primary_background`) not hardcoded colors; facilitates light/dark theme switching

## Entry Points

**core_editor library:**
- Location: `core_editor/src/lib.rs`
- Triggers: Imported by rendering backends (ratatui_client, wgpu_client)
- Responsibilities: Exports all public types needed by frontends (App, Document, EditorCommand, RenderModel, etc.)

**wgpu_client binary:**
- Location: `wgpu_client/src/main.rs`
- Triggers: Executable entry point; accepts optional file/directory path as argument
- Responsibilities: Initialize logging, parse CLI args, create WgpuApp, run winit event loop

**wgpu_client app loop:**
- Location: `wgpu_client/src/app.rs` - WgpuApp implements ApplicationHandler trait from winit
- Triggers: Every winit event (resume, window_event, redraw_requested, etc.)
- Responsibilities: Delegate events to renderer, translate input to commands, dispatch to core App, request redraws

**ratatui_client binary:**
- Location: `ratatui_client/src/main.rs`
- Triggers: Executable entry point; accepts optional file/directory path as argument
- Responsibilities: CLI parsing, event loop (crossterm), terminal drawing coordination

## Error Handling

**Strategy:** Domain operations return Result types; blocking errors are surfaced via status messages in App.status_message

**Patterns:**
- File I/O errors (open_file, save) → caught at App level, logged, status message displayed
- Protection errors (unsaved changes) → App.pending_action stores dialog state; user confirms/cancels before proceeding
- Command execution errors (e.g., clipboard unavailable) → dispatcher logs and silently continues (graceful degradation)
- Syntax parsing timeouts → large files skip incremental parsing, defer updates (MAX_INCREMENTAL_PARSE_SIZE threshold)

## Cross-Cutting Concerns

**Logging:**
- Framework: `log` crate with `simplelog` (WriteLogger)
- Location: `wgpu_client/src/main.rs` and `ratatui_client/src/main.rs` initialize logging to "editor.log"
- Usage: Debug logs at key points (app startup, file open/save, command dispatch, layout calculations)

**Validation:**
- Performed at domain boundary (CommandDispatcher validates cursor position, text range bounds)
- TextBuffer validates that TextRange is within document bounds before operations
- EditorView validates that Viewport scroll offset doesn't exceed document dimensions

**Authentication:**
- Not applicable (single-user local editor)

**Syntax Highlighting:**
- Framework: tree-sitter (incremental parsing)
- Location: `core_editor/src/syntax.rs` - SyntaxHighlighter with language detection
- Process: Parse language (Rust, Python, JS, JSON, Markdown); run tree-sitter queries on parse tree; emit TextStyle tokens for each span
- Incremental: Tracks last_content_len; only reparses if document changes exceed threshold (500KB)

**Undo/Redo:**
- Location: `core_editor/src/commands/history.rs` - CommandHistory stack
- Pattern: Each EditOperation is reversible; UndoTransaction groups related operations (e.g., word replacement = 1 delete + 1 insert, treated as 1 undo step)
- Storage: In-memory transaction stack; cleared on document save as optimization

---

*Architecture analysis: 2026-01-28*
