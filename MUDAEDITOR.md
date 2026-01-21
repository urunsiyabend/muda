# Muda Editor Domain Glossary

This document defines the core domain concepts, responsibilities, and data flows of the Muda Editor.

---

## 1. Core Identity & Lifetime

### Workspace
- **Definition**: The top-level runtime container for an editing session. Owns open documents, open editor views, global settings, and services (language, diagnostics, command dispatch).
- **Key Responsibilities**:
  - Manage **Document** lifecycle (open/close, lookup by id/URI).
  - Manage **EditorView** lifecycle (create/split/close, focus switching).
  - Route **Commands** to the correct targets (active view/document).
  - Hold cross-cutting services (`LanguageService`, `PluginHost`, `Persistence`).
- **Relationships**: `Workspace` → many `Documents`; `Workspace` → many `EditorViews`.

### Project
- **Definition**: A logical scope (often a folder/repository) that provides language/config context: build settings, LSP servers, formatters, indexing.
- **Key Responsibilities**:
  - Bind files/documents to project configuration.
  - Provide `LanguageService` configuration and diagnostics context.
- **Relationships**: `Workspace` → one or more `Projects`; `Documents` may be associated with a `Project`.

### Resource Identifier (URI)
- **Definition**: A stable identifier for a document’s backing storage location (file path, untitled, remote).
- **Used for**: Persistence, project association, reopen lists.

---

## 2. Document & Text Model

### Document
- **Definition**: An editable text artifact with identity and persistence semantics. Wraps a `TextBuffer` and owns document-level derived state (undo history, language analysis, diagnostics).
- **Owns**:
  - `TextBuffer` (the authoritative text)
  - `DocumentMetadata` (URI/path, encoding, line endings, dirty flag)
  - `UndoHistory` (document-scoped)
  - `LanguageState` (parse tree, tokenization caches, semantic layers)
  - `Diagnostics` (errors/warnings/information tied to ranges)
- **Does Not Own**: Caret, selection, or scroll (those are view-specific).
- **Relationships**: `Document` is referenced by one or many `EditorViews`.

### TextBuffer
- **Definition**: The source of truth for the document’s textual content. Provides efficient editing operations and coordinate mappings.
- **Provides**:
  - Insert/Delete/Replace operations by `TextRange` / offset.
  - Conversions between `TextOffset` and `TextPosition` (line/column).
  - Line retrieval and line indexing.
- **Implementation Note**: Rope/piece table/gap buffer is an internal choice; the domain concept stays the same.

### TextOffset
- **Definition**: A scalar position in the buffer (typically UTF-8 byte offset or Unicode scalar index—pick one and commit).
- **Purpose**: Fast, canonical location for storage and edits.

### TextPosition
- **Definition**: Human-facing location in `(line, column)` terms.
- **Important**: Column definition must be explicit (codepoint vs grapheme vs visual column).

### TextRange
- **Definition**: A half-open interval `[start, end)` in `TextOffset` coordinates.
- **Used for**: Edits, selections, diagnostics, decorations.

### DocumentSnapshot
- **Definition**: An immutable, point-in-time view of document text and derived state at a specific revision.
- **Why it matters**: Enables stable rendering and async services (tokenization, LSP) without racey reads.

### DocumentRevision
- **Definition**: A monotonically increasing version number (or opaque token) representing document state evolution.
- **Used for**: Cache invalidation, async response correlation.

---

## 3. View & Interaction Model

### EditorView
- **Definition**: A viewport onto a `Document`: caret(s), selection(s), scroll position, and view-specific presentation settings. Multiple views may point to the same `Document`.
- **Owns (View State)**:
  - `Viewport` (scroll position and visible region)
  - `CaretSet` (one or more carets)
  - `SelectionSet` (one or more selections)
  - **View Options** (line numbers on/off, tab width, wrap mode, folding state)
- **Does Not Own**: Document text, undo history, language analysis caches.

### Viewport
- **Definition**: The mapping from the document to what is currently visible.
- **Fields**: Top line, left column (or pixel offsets later in GUI), and dimensions.
- **Key Idea**: Scroll belongs here.

### Caret
- **Definition**: A single insertion point (a cursor).
- **Note**: In modern editors, multiple carets are common; thus `CaretSet`.

### CaretSet
- **Definition**: An ordered set of carets with one `PrimaryCaret`.
- **Purpose**: Multi-cursor editing and consistent command semantics.

### Selection
- **Definition**: A range of text associated with a caret; may be empty (caret only).
- **Important**: Stored as `TextRange` in document coordinates.
- **Why View-Owned**: Same `Document` can be shown in multiple views with different selections.

### SelectionSet
- **Definition**: One or more selections (often one per caret).
- **Convention**: Each selection is anchored; selection direction matters for extend operations.

### Logical vs Visual Coordinates
- **LogicalPosition**: Line/column in the document model.
- **VisualPosition**: Row/column on screen after wrapping, folding, tab expansion, etc.
- **Rule of Thumb**: Core editing uses logical positions; rendering uses visual positions.

---

## 4. Editing Behavior & History

### EditorCommand
- **Definition**: A user-intent operation (e.g., `InsertChar`, `DeleteWord`, `SaveDocument`, `ToggleLineNumbers`).
- **Key Property**: UI-agnostic. TUI and GUI produce the same commands.

### EditOperation
- **Definition**: A minimal reversible change to the `TextBuffer` (insert/delete/replace with sufficient data to undo).
- **Used by**: `UndoHistory`.

### UndoHistory
- **Definition**: A document-scoped log/stack of undoable operations grouped into `UndoTransaction` units.
- **Key Responsibilities**: Undo, redo, coalescing, transaction boundaries.

### UndoTransaction
- **Definition**: A grouping boundary for multiple `EditOperations` that should undo as one step (e.g., paste, auto-indent, multi-caret insert).
- **Why it matters**: Professional feel and plugin compatibility.

### CommandDispatcher
- **Definition**: The orchestrator that routes `EditorCommands` to the correct targets and services (active view/document/workspace).
- **Responsibilities**:
  - Resolve target context (active view, active document).
  - Apply editing commands as transactions.
  - Emit events for downstream consumers (view model, plugins, UI).

---

## 5. Language & Analysis Model

### LanguageId
- **Definition**: The document’s language classification (Rust, Python, Markdown, PlainText).
- **Used for**: Syntax highlighting, formatting, diagnostics, LSP routing.

### LanguageService
- **Definition**: The provider of language-aware features for a language or project: tokenization, parsing, formatting, diagnostics, symbol indexing, LSP integration.
- **Contracts**:
  - Accept document snapshots.
  - Return `TokenSpans`, `Diagnostics`, formatting edits, symbol lists, etc.

### SyntaxTree
- **Definition**: The structural parse representation (tree-sitter or equivalent).
- **Note**: Typically derived, cacheable, and updated incrementally later.

### TokenSpan
- **Definition**: A range with a semantic class (keyword/string/comment/identifier).
- **Purpose**: Syntax highlighting and semantic styling.

### Diagnostic
- **Definition**: A reported issue over a `TextRange` with severity and message (error/warning/info/hint).
- **Presentation**: Underlines, gutter icons, list panels.

---

## 6. Presentation Model (ViewModel / Projection)

### ViewModel (EditorViewModel / PresentationModel)
- **Definition**: A render-ready projection built from (`DocumentSnapshot` + `EditorView` state + presentation settings).
- **Role**: Decouple rendering from domain internals, support full rerender now and incremental updates later.
- **Contains**:
  - Visible lines (or visual lines).
  - Styled spans (merged layers: tokens + selection + diagnostics decorations).
  - Gutter model (line numbers, markers).
  - Caret visual coordinates for the current viewport.
  - Statusline model (filename, dirty, position, language, diagnostics summary).

### RenderModel
- **Definition**: The minimal data the UI needs to draw a frame. Often synonymous with `ViewModel`, but can be UI-backend-specific if needed.
- **Rule**: UI never reads `TextBuffer` directly; it draws `RenderModel`.

### Decoration
- **Definition**: A non-text styling overlay: selection highlight, diagnostics underline, bracket match, search matches, inline hints.
- **Note**: Represented as layers of spans to keep it extensible for plugins.

---

## 7. Eventing & Extensibility

### DomainEvent
- **Definition**: A typed notification that something meaningful changed (e.g., `DocumentChanged`, `SelectionChanged`, `ViewportChanged`, `DiagnosticsUpdated`).
- **Purpose**: Drive view model rebuild and plugin reactions; enable future incremental rendering.

### EventBus
- **Definition**: The mechanism to publish/subscribe `DomainEvents` within the app core.
- **Note**: Can be synchronous initially; designed to evolve.

### Plugin
- **Definition**: A module that extends behavior via commands, event handlers, language providers, and decorations.
- **Key Principle**: Plugins target domain concepts (`Workspace`, `Document`, `View`, `Commands`, `Events`), not UI toolkits.

### PluginHost
- **Definition**: Loads plugins, manages their lifecycle, exposes stable extension points.

### Extension Point
- **Definition**: A stable integration contract: command registration, event subscription, decoration provider, language feature provider, UI contribution slot.

---

## 8. Interaction & Data Flow

Below is the “full rerender pipeline” written as a step-by-step flow.

### A) Input → Command → Domain Mutation
1.  **UI Backend** receives an input event (key press, paste, scroll, resize).
2.  **UI Shell** maps raw input to an `EditorCommand` (Keymap/bindings). *Note: UI does not directly edit text.*
3.  **CommandDispatcher** resolves the target context:
    - Determine `ActiveEditorView` (focused view).
    - Resolve `TargetDocument` from the view.
4.  **CommandDispatcher** executes the command:
    - **Navigation commands**: Mutate `EditorView` state (caret, selection, viewport).
    - **Edit commands**: Open an `UndoTransaction`, apply `EditOperations` to `TextBuffer`, commit transaction.
    - **Save commands**: Delegate to `PersistenceService` using metadata and current text.
5.  **Domain** emits `DomainEvents` (e.g., `DocumentChanged`, `ViewStateChanged`).
6.  **EventBus** publishes events to all subscribers.

### B) Domain → ViewModel Build (Projection)
1.  **ViewModelBuilder** receives a “render requested” signal (frame-driven or event-driven).
2.  **ViewModelBuilder** reads the current `EditorView` state:
    - Viewport (scroll, dimensions).
    - CaretSet & SelectionSet.
    - Presentation options (line numbers, etc.).
    - Target `DocumentId`.
3.  **ViewModelBuilder** obtains a `DocumentSnapshot` (text, revision, tokens, diagnostics).
4.  **ViewModelBuilder** computes the **Visible Logical Range** based on viewport.
5.  **ViewModelBuilder** constructs **VisibleLines**:
    - For each line: Fetch text → Fetch tokens → Compute selection overlap → Fetch decorations.
    - Merge layers into styled spans.
    - Produce `LinePresentation` (gutter + spans).
6.  **ViewModelBuilder** computes `CaretVisualPosition`.
7.  **ViewModelBuilder** constructs `StatusPresentation`.
8.  **ViewModelBuilder** outputs a `ViewModel / RenderModel`.

### C) ViewModel → UI Rendering
1.  **UI Renderer** receives `RenderModel`.
2.  **UI Renderer** chooses how to draw (TUI/GUI) and maps semantic classes to theme colors.
3.  **UI Renderer** draws the frame:
    - Editor region lines.
    - Gutter.
    - Caret.
    - Status bar.
    - Overlays (dialogs, command palette).
4.  **Loop**: Steps repeats for the next frame.

---

## 9. Naming Principles

To keep the vocabulary stable and industry-standard:

- **Document**: For file-like text artifacts (JetBrains-style).
- **EditorView / TextView**: For view/viewport state (VS-style).
- **TextBuffer**: For the authoritative text store (universal).
- **ViewModel / PresentationModel**: For render-ready projections.
- **EditorCommand**: For user intents.
- **EditOperation**: For undoable atomic changes.
- **UndoTransaction**: For grouping operations.
- **DomainEvent / EventBus**: For decoupled reactions and future incremental rendering.
