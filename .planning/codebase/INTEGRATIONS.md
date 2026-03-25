# External Integrations

**Analysis Date:** 2026-01-28

## APIs & External Services

**None detected**

The codebase contains no integration with remote APIs, web services, or cloud platforms. All functionality is local-only.

## Data Storage

**Databases:**
- Not used - No database integration detected
- All state is in-memory or file-based

**File Storage:**
- Local filesystem only
  - Read/write via `std::fs::read_to_string()` and `std::fs::write()`
  - Location: Any file path provided via command-line argument or opened via file dialog
  - Automatic line-ending detection and preservation (LF, CRLF, CR)
  - Implementation: `Document::open()` in `core_editor/src/domain/document.rs` (line 132-135)

**Caching:**
- In-memory only
- Cached content string in Document for syntax highlighting: `cached_content` field in `core_editor/src/domain/document.rs`
- No external caching service

## Authentication & Identity

**Auth Provider:**
- None - Application is single-user, local-only
- No authentication mechanism implemented
- No user accounts or identity management

## Monitoring & Observability

**Error Tracking:**
- None - No error tracking service (Sentry, etc.)

**Logs:**
- Local file logging only
- Location: `editor.log` in current working directory
- Format: Debug-level logs via `simplelog::WriteLogger`
- Implementation: Initialized in both clients
  - TUI: `ratatui_client/src/main.rs` (line 16-17)
  - GPU: `wgpu_client/src/main.rs` (line 29-30)
- No log rotation or management

## Clipboard Integration

**System Clipboard:**
- Provider: `arboard` crate (3.6.1)
- Scope: Copy/Cut/Paste operations
- Platforms: Windows, macOS, Linux
- Implementation:
  - Core commands in `core_editor/src/commands/editor_command.rs`
  - TUI integration via `crossterm` bracketed paste events: `EnableBracketedPaste` in `ratatui_client/src/main.rs` (line 61)
  - GPU client: Native paste handling via winit window events
- Bidirectional: Reads from and writes to system clipboard

## CI/CD & Deployment

**Hosting:**
- None - Desktop application only, no remote deployment infrastructure

**CI Pipeline:**
- Not detected - No GitHub Actions, GitLab CI, or similar automation found

**Build Process:**
- Manual: `cargo build --release`
- Output binaries:
  - `target/release/muda` (GPU client binary)
  - `target/release/muda-tui` (TUI client binary)

## Window Management & Input

**Window Framework:**
- Provider: `winit` (0.30) for GPU client
- Features: Event loop management, window creation, input capture
- Input handling:
  - Keyboard: Modifier keys (Ctrl, Shift), standard keycodes
  - Mouse: Not implemented (TUI/keyboard-centric)
  - Window events: Resize, focus, close

**Terminal Control (TUI):**
- Provider: `crossterm` (0.29.0)
- Features: Raw mode, alternate screen buffer, mouse/paste event support
- Input capture: Standard terminal events

## Environment Configuration

**Required env vars:**
- None - Application is fully self-contained with no external configuration

**Secrets location:**
- No secrets or credentials used
- Clipboard access is OS-level (no authentication required)

## Webhooks & Callbacks

**Incoming:**
- None - Application is local-only

**Outgoing:**
- None - No external callbacks or integrations

## Event System

**Internal Event Bus:**
- Custom implementation: `EventBus` in `core_editor/src/events/mod.rs`
- Purpose: Domain event dispatch (document opened, text changed, etc.)
- Not connected to external systems

## Language Support

**Syntax Highlighting:**
- Providers: Tree-sitter language parsers (bundled)
- Supported languages:
  - Rust (tree-sitter-rust)
  - Python (tree-sitter-python)
  - JavaScript (tree-sitter-javascript)
  - JSON (tree-sitter-json)
  - TOML (tree-sitter-toml)
  - Markdown (tree-sitter-md)
  - Plain text (fallback)
- No remote language server protocol (LSP) integration
- No code completion or intelligent analysis

---

*Integration audit: 2026-01-28*
