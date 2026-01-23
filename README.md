MUDA Unacceptably Does Anything

MUDA is an extensible IDE.
MUDA does anything.
MUDA does things it probably shouldn’t.
MUDA is unacceptably extensible.

MUDA Unacceptably Does Anything

## About

MUDA (MUDA Unacceptably Does Anything) is an experimental, highly extensible text editor and IDE framework built in Rust. It emphasizes a strict clean architecture, domain-driven design, and a clear separation between the core editing domain and the view/presentation layer.

Designed to be "unacceptably extensible," MUDA aims to provide a robust foundation where every aspect of the editor—from keybindings and commands to rendering and language services—can be customized and extended.

## Key Features

*   **Clean Architecture**: Core domain logic (documents, text buffers) is completely decoupled from the UI/View layer (cursors, scrolling).
*   **Workspace-Centric**: Manages multiple documents, views, and projects within a unified lifecycle.
*   **Dual-Mode Launch**: Automatically adapts its UI based on context—launching with a directory opens the sidebar explorer, while launching with a file maximizes the editor.
*   **Robust Text Model**: Features a high-performance text buffer with efficient edit operations and coordinate mappings.
*   **Command System**: A transactional command dispatch system supporting robust undo/redo history.
*   **Extensibility First**: Built with an event-driven architecture and plugin system in mind.

## Getting Started

### Prerequisites

*   Rust stable toolchain (1.70+ recommended)

### Building

```bash
cargo build --release
```

### Running

To run the editor:

```bash
cargo run --release
```

To open a specific file or directory:

```bash
# Opens the current directory with the file explorer sidebar visible
cargo run --release -- .

# Opens a specific file with the editor focused and sidebar hidden
cargo run --release -- src/main.rs
```

## Keybindings

### Global
*   `Ctrl + s`: Save current document
*   `Ctrl + b`: Toggle Sidebar visibility
*   `Ctrl + h`: Focus Sidebar (if visible)
*   `Ctrl + l`: Focus Editor
*   `Esc`: Quit (prompts if unsaved changes exist)

### Editor
*   `Ctrl + c / x / v`: Copy, Cut, Paste
*   `Ctrl + z / y`: Undo, Redo
*   `Ctrl + a`: Select All
*   `Ctrl + l`: Toggle Line Numbers (when in editor)
*   `Shift + Arrows`: Create/Extend Selection
*   `Ctrl + Left/Right`: Move cursor by word
*   `Ctrl + Home/End`: Move to start/end of file

### Sidebar (File Explorer)
*   `j / k` or `Up / Down`: Navigate files
*   `Enter`: Open file or Enter directory
*   `Backspace / Left` or `h`: Go up a directory
*   `Esc`: Return focus to editor (if file is open)

## Project Structure

The codebase follows a strict separation of concerns:

```
src/
├── domain/       # Core business logic (Document, TextBuffer, etc.)
│   └── text/     # Text manipulation and storage models
├── view/         # UI state (Caret, Selection, Viewport)
├── view_model/   # Presentation layer (Render models, styling)
├── commands/     # Command pattern implementation (Undo/Redo transactions)
├── events/       # Event bus and domain event definitions
├── syntax/       # Syntax highlighting and parsing logic
├── app.rs        # Main application state and event loop orchestration
├── draw.rs       # TUI rendering logic (using Ratatui)
└── main.rs       # Entry point and terminal setup
```

## Architecture

For a deep dive into the project's domain glossaries, core concepts, and architectural decisions, please refer to [MUDAEDITOR.md](./MUDAEDITOR.md).

### Core Concepts

*   **Document**: The authoritative source of truth for text. It owns the `TextBuffer` and `UndoHistory` but knows nothing about how it is displayed.
*   **EditorView**: A projection of a document. It owns the `Caret`, `Selection`, and `Viewport` (scroll position). Multiple views can point to the same document.
*   **ViewModel**: An immutable snapshot created from the Document and View, ready for rendering. This ensures the UI is always drawing a consistent state.

## License

[MIT License](LICENSE)
